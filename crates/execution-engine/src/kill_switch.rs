//! # Kill Switch
//!
//! 시스템 전역 비상 정지 메커니즘.
//!
//! ## 설계 원칙 (기술문서 4.3)
//! - `AtomicBool`로 구현 — **락(Lock) 없이 원자적 연산**으로 지연 시간 최소화
//! - 모든 모듈(Feed, Strategy, Execution)이 동일한 `Arc<AtomicBool>` 공유
//! - 활성화 시: 신규 주문 즉시 거부, 미체결 주문 취소 요청, 포지션 청산 트리거
//!
//! ## 트리거 조건
//! 1. 일일 손실 한도 도달
//! 2. 포지션 한도 위반
//! 3. 연결 끊김 (Feed 이상)
//! 4. 수동 개입 (Ctrl+C, API 호출)
//! 5. 연속 주문 실패 임계값 초과

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// 킬 스위치 활성화 사유
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillReason {
    /// 일일 손실 한도 도달
    DailyLossLimit,
    /// 포지션 한도 위반
    PositionLimit,
    /// 데이터 피드 연결 끊김
    FeedDisconnected,
    /// 수동 개입 (운영자)
    ManualIntervention,
    /// 연속 주문 실패
    ConsecutiveOrderFailures,
    /// 리스크 엔진 이상 감지
    RiskAnomaly,
}

impl std::fmt::Display for KillReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DailyLossLimit => write!(f, "DAILY_LOSS_LIMIT"),
            Self::PositionLimit => write!(f, "POSITION_LIMIT"),
            Self::FeedDisconnected => write!(f, "FEED_DISCONNECTED"),
            Self::ManualIntervention => write!(f, "MANUAL_INTERVENTION"),
            Self::ConsecutiveOrderFailures => write!(f, "CONSECUTIVE_ORDER_FAILURES"),
            Self::RiskAnomaly => write!(f, "RISK_ANOMALY"),
        }
    }
}

/// 전역 킬 스위치.
///
/// 모든 필드가 Atomic → 락 없이 모든 스레드에서 안전하게 읽기/쓰기.
pub struct KillSwitch {
    /// 핵심 플래그 — 이것만 확인하면 됨
    active: Arc<AtomicBool>,
    /// 활성화 시각 (나노초, 0이면 미활성화)
    activated_at_ns: AtomicU64,
    /// 활성화 사유 코드 (0이면 미활성화)
    reason_code: AtomicU64,
}

impl KillSwitch {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            activated_at_ns: AtomicU64::new(0),
            reason_code: AtomicU64::new(0),
        }
    }

    /// 다른 모듈과 공유할 `Arc<AtomicBool>` 반환.
    ///
    /// Feed Handler, Strategy Engine이 이 핸들을 받아서
    /// `load(Ordering::Acquire)`로 킬 스위치 상태를 확인합니다.
    pub fn shared_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.active)
    }

    /// 킬 스위치 활성화 (비상 정지)
    ///
    /// **이 함수는 한 번 호출되면 시스템 전체가 정지합니다.**
    ///
    /// `Ordering::Release`로 설정하여 다른 스레드가 `Acquire`로
    /// 읽을 때 이전의 모든 메모리 쓰기가 보이도록 보장합니다.
    pub fn activate(&self, reason: KillReason) {
        // 이미 활성화된 경우 중복 로깅 방지
        if self.active.load(Ordering::Relaxed) {
            return;
        }

        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        self.reason_code
            .store(reason_to_code(reason), Ordering::Relaxed);
        self.activated_at_ns.store(now_ns, Ordering::Relaxed);

        // 핵심: Release ordering으로 모든 사전 쓰기를 가시화
        self.active.store(true, Ordering::Release);

        tracing::error!(
            reason = %reason,
            "🚨 KILL SWITCH ACTIVATED — all trading halted"
        );
    }

    /// 킬 스위치 상태 확인
    ///
    /// Hot path에서 호출 — `Acquire` ordering으로 최소 오버헤드.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// 킬 스위치 리셋 (운영자 수동 복구 시에만 사용)
    ///
    /// ⚠️ 주의: 프로덕션에서는 자동 리셋 금지.
    /// 반드시 운영자가 원인을 확인한 후 수동으로 호출해야 합니다.
    pub fn reset(&self) {
        self.active.store(false, Ordering::Release);
        self.activated_at_ns.store(0, Ordering::Relaxed);
        self.reason_code.store(0, Ordering::Relaxed);

        tracing::warn!("kill switch RESET — trading may resume");
    }

    /// 활성화 사유
    pub fn reason(&self) -> Option<KillReason> {
        let code = self.reason_code.load(Ordering::Relaxed);
        code_to_reason(code)
    }

    /// 활성화 시각 (나노초)
    pub fn activated_at_ns(&self) -> Option<u64> {
        let ts = self.activated_at_ns.load(Ordering::Relaxed);
        if ts > 0 {
            Some(ts)
        } else {
            None
        }
    }
}

// ── KillReason ↔ u64 변환 (AtomicU64에 저장하기 위해) ──

fn reason_to_code(r: KillReason) -> u64 {
    match r {
        KillReason::DailyLossLimit => 1,
        KillReason::PositionLimit => 2,
        KillReason::FeedDisconnected => 3,
        KillReason::ManualIntervention => 4,
        KillReason::ConsecutiveOrderFailures => 5,
        KillReason::RiskAnomaly => 6,
    }
}

fn code_to_reason(code: u64) -> Option<KillReason> {
    match code {
        1 => Some(KillReason::DailyLossLimit),
        2 => Some(KillReason::PositionLimit),
        3 => Some(KillReason::FeedDisconnected),
        4 => Some(KillReason::ManualIntervention),
        5 => Some(KillReason::ConsecutiveOrderFailures),
        6 => Some(KillReason::RiskAnomaly),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_switch_lifecycle() {
        let ks = KillSwitch::new();
        assert!(!ks.is_active());
        assert!(ks.reason().is_none());

        ks.activate(KillReason::DailyLossLimit);
        assert!(ks.is_active());
        assert_eq!(ks.reason(), Some(KillReason::DailyLossLimit));
        assert!(ks.activated_at_ns().is_some());

        ks.reset();
        assert!(!ks.is_active());
    }

    #[test]
    fn test_shared_flag_cross_thread() {
        let ks = KillSwitch::new();
        let flag = ks.shared_flag();

        assert!(!flag.load(Ordering::Acquire));

        ks.activate(KillReason::ManualIntervention);

        // 다른 스레드에서 보는 것과 동일
        assert!(flag.load(Ordering::Acquire));
    }
}
