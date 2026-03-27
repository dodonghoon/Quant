//! # Execution Orchestrator
//!
//! Strategy Engine의 `TradingSignal`을 수신하여 주문 집행까지의
//! 전체 파이프라인을 조율합니다.
//!
//! ## 파이프라인
//! ```text
//! TradingSignal (crossbeam)
//!     │
//!     ▼
//! [1. Kelly Sizer] → optimal position fraction
//!     │
//!     ▼
//! [2. Position Delta] → 현재 vs 목표 포지션 차이 계산
//!     │
//!     ▼
//! [3. Risk Check] → pre-trade 리스크 검증
//!     │
//!     ▼
//! [4. OMS] → 주문 생성 & 상태 관리
//!     │
//!     ▼
//! [5. Gateway] → 거래소 전송 (async)
//! ```
//!
//! ## 스레딩
//! - Signal 수신: crossbeam channel (Strategy 스레드로부터)
//! - Gateway 전송: tokio spawn (I/O bound)
//! - 나머지: 동기 처리 (CPU-light)

use crate::error::Result;
use crate::kelly::{KellyConfig, KellyOutput, KellySizer};
use crate::kill_switch::{KillReason, KillSwitch};
use crate::oms::{
    ExchangeGateway, FillReport, OrderManager, OrderRequest, OrderSide,
    OrderStatus, OrderType, TimeInForce,
};
use crate::risk::{RiskConfig, RiskEngine};
use data_ingestion::types::Symbol;
use strategy_engine::{SignalDirection, TradingSignal};

/// Execution Engine 설정
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub risk_config: RiskConfig,
    pub kelly_config: KellyConfig,
    /// 기본 승률 (전략 통계가 없을 때)
    pub default_win_rate: f64,
    /// 기본 수익/손실 비율
    pub default_win_loss_ratio: f64,
    /// OMS 완료 주문 보관 한도
    pub oms_history_size: usize,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            risk_config: RiskConfig::default(),
            kelly_config: KellyConfig::default(),
            default_win_rate: 0.52,
            default_win_loss_ratio: 1.5,
            oms_history_size: 10_000,
        }
    }
}

/// Execution Engine 오케스트레이터
pub struct ExecutionEngine {
    config: ExecutionConfig,
    risk: RiskEngine,
    kelly: KellySizer,
    oms: OrderManager,

    /// 처리 통계
    signals_received: u64,
    orders_sent: u64,
    orders_rejected: u64,
}

impl ExecutionEngine {
    pub fn new(config: ExecutionConfig) -> Self {
        let risk = RiskEngine::new(config.risk_config.clone());
        let kelly = KellySizer::new(config.kelly_config.clone());
        let oms = OrderManager::new(config.oms_history_size);

        Self {
            config,
            risk,
            kelly,
            oms,
            signals_received: 0,
            orders_sent: 0,
            orders_rejected: 0,
        }
    }

    /// 킬 스위치 공유 핸들 (Feed/Strategy에 전달)
    pub fn shared_kill_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.risk.shared_kill_flag()
    }

    /// Risk Engine 참조 (외부 가격 업데이트용)
    pub fn risk_engine_mut(&mut self) -> &mut RiskEngine {
        &mut self.risk
    }

    /// OMS 참조 (외부 체결 보고서 적용용)
    pub fn oms_mut(&mut self) -> &mut OrderManager {
        &mut self.oms
    }

    /// **메인 신호 처리**: TradingSignal → 주문 (동기)
    ///
    /// ## 반환
    /// - `Some((order_id, OrderRequest))`: 주문 생성 성공 → Gateway에 전송 필요
    /// - `None`: 신호 무시 (Neutral, 리스크 위반, Kelly 거부 등)
    pub fn process_signal(
        &mut self,
        symbol: &Symbol,
        signal: &TradingSignal,
    ) -> Option<(u64, OrderRequest)> {
        self.signals_received += 1;

        // ── 1. 방향 필터 ──
        if signal.direction == SignalDirection::Neutral {
            return None;
        }

        // ── 2. Kelly 포지션 사이징 ──
        let kelly_out = self.kelly.size_from_signal(
            signal.raw_position_frac,
            self.config.default_win_rate,
            self.config.default_win_loss_ratio,
        );

        if !kelly_out.should_trade() {
            tracing::debug!(
                symbol = %symbol,
                reason = ?kelly_out.reject_reason,
                "kelly rejected signal"
            );
            return None;
        }

        // ── 3. 목표 포지션 vs 현재 포지션 → 델타 계산 ──
        let current_pos = self
            .risk
            .position(symbol)
            .map(|p| p.quantity)
            .unwrap_or(0.0);

        // 목표 포지션 = Kelly 비율 × 자본 / 현재 가격 (단순화)
        // 실제로는 notional 기반으로 계산해야 하지만 여기서는 비율만 사용
        let target_frac = kelly_out.signed_fraction();
        let delta = target_frac - self.normalize_position(current_pos, symbol);

        // 의미 있는 변화가 아니면 무시
        if delta.abs() < 0.0001 {
            return None;
        }

        // ── 4. 주문 구성 ──
        let (side, quantity) = if delta > 0.0 {
            (OrderSide::Buy, delta.abs())
        } else {
            (OrderSide::Sell, delta.abs())
        };

        // 수량을 실제 단위로 변환 (단순화: 비율을 그대로 사용)
        let order_qty = (quantity * self.config.risk_config.max_order_size)
            .min(self.config.risk_config.max_order_size);

        if order_qty < 0.0001 {
            return None;
        }

        let request = OrderRequest {
            symbol: *symbol,
            side,
            order_type: OrderType::Ioc, // HFT 기본: IOC
            quantity: order_qty,
            price: 0.0, // Market / IOC는 시장가
            time_in_force: TimeInForce::Ioc,
        };

        // ── 5. Pre-trade Risk Check ──
        match self.risk.check_order(&request, signal.ts_ns) {
            Ok(()) => {}
            Err(e) => {
                self.orders_rejected += 1;
                tracing::warn!(
                    symbol = %symbol,
                    error = %e,
                    "order rejected by risk engine"
                );
                return None;
            }
        }

        // ── 6. OMS에 등록 ──
        let order_id = self.oms.create_order(request.clone(), signal.ts_ns);
        self.orders_sent += 1;

        tracing::info!(
            order_id = order_id,
            symbol = %symbol,
            side = ?request.side,
            qty = format!("{:.6}", request.quantity),
            direction = ?signal.direction,
            kelly_frac = format!("{:.4}", kelly_out.final_fraction),
            confidence = format!("{:.3}", signal.confidence),
            "ORDER CREATED"
        );

        Some((order_id, request))
    }

    /// 체결 보고서 처리
    pub fn on_fill(&mut self, report: &FillReport) -> Result<()> {
        let fill_result = self.oms.apply_fill(report)?;

        // Risk Engine에 포지션 반영
        self.risk.on_fill(
            fill_result.symbol,
            fill_result.signed_qty(),
            fill_result.fill_price,
        );

        tracing::info!(
            symbol = %fill_result.symbol,
            side = ?fill_result.side,
            qty = format!("{:.6}", fill_result.filled_qty),
            price = format!("{:.2}", fill_result.fill_price),
            complete = fill_result.is_complete,
            "FILL"
        );

        Ok(())
    }

    /// 주문 전송 완료 콜백
    pub fn on_order_sent(&mut self, order_id: u64, exchange_id: String) -> Result<()> {
        self.oms.set_exchange_id(order_id, exchange_id)?;
        self.oms
            .transition(order_id, OrderStatus::Sent, self.now_ns())
    }

    /// 주문 실패 콜백
    pub fn on_order_failed(&mut self, order_id: u64) {
        let _ = self
            .oms
            .transition(order_id, OrderStatus::Rejected, self.now_ns());
        self.risk.on_order_failure();
        self.orders_rejected += 1;
    }

    /// 킬 스위치 활성화 시 모든 주문 취소
    pub fn emergency_shutdown(&mut self) {
        let cancelled = self.oms.cancel_all(self.now_ns());
        tracing::error!(
            cancelled_orders = cancelled.len(),
            "🚨 EMERGENCY SHUTDOWN — all orders cancelled"
        );
    }

    /// 시가 갱신 (BBO 업데이트 시 호출)
    pub fn update_market_price(&mut self, symbol: Symbol, price: f64) {
        self.risk.update_price(symbol, price);
    }

    /// 실행 통계
    pub fn stats(&self) -> ExecutionStats {
        ExecutionStats {
            signals_received: self.signals_received,
            orders_sent: self.orders_sent,
            orders_rejected: self.orders_rejected,
            active_orders: self.oms.active_count(),
            daily_pnl: self.risk.daily_pnl(),
        }
    }

    /// 일일 리셋
    pub fn reset_daily(&mut self) {
        self.risk.reset_daily();
        self.signals_received = 0;
        self.orders_sent = 0;
        self.orders_rejected = 0;
    }

    // ── Private ──

    /// 포지션을 정규화된 비율로 변환 (단순화)
    fn normalize_position(&self, quantity: f64, _symbol: &Symbol) -> f64 {
        // 실제로는 notional / capital 계산
        // 여기서는 max_position 대비 비율로 단순화
        if self.config.risk_config.max_position_per_symbol > 0.0 {
            quantity / self.config.risk_config.max_position_per_symbol
        } else {
            0.0
        }
    }

    fn now_ns(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

/// 실행 통계
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub signals_received: u64,
    pub orders_sent: u64,
    pub orders_rejected: u64,
    pub active_orders: usize,
    pub daily_pnl: f64,
}

impl std::fmt::Display for ExecutionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "signals={} sent={} rejected={} active={} pnl={:.2}",
            self.signals_received,
            self.orders_sent,
            self.orders_rejected,
            self.active_orders,
            self.daily_pnl
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strategy_engine::signal::{AlphaBreakdown, SignalDirection, TradingSignal};

    fn make_signal(direction: SignalDirection, z: f64, confidence: f64) -> TradingSignal {
        TradingSignal {
            direction,
            composite_z: z,
            confidence,
            raw_position_frac: direction.sign() * direction.strength() * confidence,
            ts_ns: 12345,
            alpha_breakdown: AlphaBreakdown {
                ou_z: z,
                ou_weight: 0.7,
                ou_mean_reverting: true,
                kalman_innovation: 0.0,
                kalman_gain: 0.1,
                kalman_weight: 0.3,
            },
        }
    }

    #[test]
    fn test_neutral_signal_ignored() {
        let mut engine = ExecutionEngine::new(ExecutionConfig::default());
        let sym = Symbol::from_str("BTCUSDT");

        let signal = make_signal(SignalDirection::Neutral, 0.0, 0.0);
        assert!(engine.process_signal(&sym, &signal).is_none());
    }

    #[test]
    fn test_strong_buy_creates_order() {
        let mut engine = ExecutionEngine::new(ExecutionConfig::default());
        let sym = Symbol::from_str("BTCUSDT");

        let signal = make_signal(SignalDirection::StrongBuy, -3.0, 0.9);
        let result = engine.process_signal(&sym, &signal);

        assert!(result.is_some(), "strong buy should create an order");
        let (order_id, request) = result.unwrap();
        assert!(order_id > 0);
        assert_eq!(request.side, OrderSide::Buy);
    }

    #[test]
    fn test_fill_updates_risk() {
        let mut engine = ExecutionEngine::new(ExecutionConfig::default());
        let sym = Symbol::from_str("BTCUSDT");

        // 주문 생성
        let signal = make_signal(SignalDirection::Buy, -2.0, 0.8);
        let (order_id, _request) = engine.process_signal(&sym, &signal).unwrap();

        // 전송 성공
        engine.on_order_sent(order_id, "EX-1".into()).unwrap();

        // 체결
        let report = FillReport {
            internal_id: order_id,
            exchange_id: "EX-1".into(),
            filled_qty: 1.0,
            fill_price: 50000.0,
            is_final: true,
            ts_ns: 99999,
        };
        engine.on_fill(&report).unwrap();

        // Risk에 포지션 반영 확인
        let pos = engine.risk_engine_mut().position(&sym).unwrap();
        assert!((pos.quantity - 1.0).abs() < 1e-10);
    }
}
