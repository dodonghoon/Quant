//! # Execution Engine Error Types
//!
//! 주문 집행 및 리스크 관리 계층 전용 에러 타입.
//! OMS 상태 전이, 리스크 위반, 킬 스위치 관련 오류를 포괄합니다.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    /// 킬 스위치 활성화 상태 — 모든 주문 거부
    #[error("Kill Switch 활성화 — 모든 주문 거부")]
    KillSwitchActive,

    /// 일일 손실 한도 초과
    #[error("일일 손실 한도 도달: 현재 PnL = {current_pnl:.2}, 한도 = {limit:.2}")]
    DailyLossLimitReached { current_pnl: f64, limit: f64 },

    /// Pre-trade 리스크 위반
    #[error("리스크 위반 [{rule}]: {detail}")]
    RiskViolation {
        rule: &'static str,
        detail: String,
    },

    /// 심볼별 포지션 한도 초과
    #[error("포지션 한도 초과: {symbol} — 현재 {current:.4}, 최대 {max:.4}")]
    PositionLimitExceeded {
        symbol: String,
        current: f64,
        max: f64,
    },

    /// 존재하지 않는 주문 ID
    #[error("알 수 없는 주문 ID: {0}")]
    UnknownOrderId(String),

    /// OMS 상태 전이 오류
    #[error("유효하지 않은 상태 전이: {from} → {to}")]
    InvalidStateTransition { from: String, to: String },

    /// Kelly Criterion 계산 오류
    #[error("Kelly 계산 오류: {0}")]
    KellyError(String),

    /// 게이트웨이 통신 오류
    #[error("거래소 게이트웨이 오류: {0}")]
    GatewayError(String),
}

pub type Result<T> = std::result::Result<T, ExecutionError>;
