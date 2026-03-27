//! # Pre-trade Risk Engine
//!
//! 주문이 OMS에 도달하기 전에 다단계 리스크 체크를 수행합니다.
//!
//! ## 체크 순서 (빠른 거부 우선)
//! ```text
//! 1. Kill Switch (AtomicBool — ns 단위)
//! 2. Daily PnL Limit
//! 3. Per-symbol Position Limit
//! 4. Total Exposure Limit
//! 5. Order Rate Limit (초당 주문 수)
//! 6. Max Order Size
//! ```
//!
//! 모든 체크는 O(1)이며 락을 사용하지 않습니다.
//! 위반 시 즉시 거부하고, 중대 위반 시 킬 스위치를 활성화합니다.

use crate::error::{ExecutionError, Result};
use crate::kill_switch::{KillReason, KillSwitch};
use crate::oms::OrderRequest;
use data_ingestion::types::Symbol;
use std::collections::HashMap;

/// 리스크 한도 설정
#[derive(Debug, Clone)]
pub struct RiskConfig {
    /// 일일 최대 손실 (절대값, 계좌 통화 기준)
    pub max_daily_loss: f64,
    /// 심볼별 최대 포지션 크기 (기초자산 수량)
    pub max_position_per_symbol: f64,
    /// 총 노출도 한도 (전체 포지션 절대값 합 / 자본)
    pub max_total_exposure: f64,
    /// 단일 주문 최대 수량
    pub max_order_size: f64,
    /// 초당 최대 주문 수 (rate limit)
    pub max_orders_per_second: u32,
    /// 연속 실패 시 킬 스위치 활성화 임계값
    pub max_consecutive_failures: u32,
    /// 총 자본
    pub total_capital: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_daily_loss: 1000.0,           // $1,000
            max_position_per_symbol: 100.0,    // 100 units
            max_total_exposure: 2.0,           // 200% leverage max
            max_order_size: 10.0,              // 10 units per order
            max_orders_per_second: 50,
            max_consecutive_failures: 5,
            total_capital: 100_000.0,          // $100,000
        }
    }
}

/// 심볼별 포지션 추적
#[derive(Debug, Default, Clone)]
pub struct PositionTracker {
    /// 현재 포지션 (양수: long, 음수: short)
    pub quantity: f64,
    /// 평균 진입가
    pub avg_entry_price: f64,
    /// 실현 PnL (오늘 누적)
    pub realized_pnl: f64,
    /// 미실현 PnL (현재 시가 기준)
    pub unrealized_pnl: f64,
}

impl PositionTracker {
    /// 체결 반영
    pub fn apply_fill(&mut self, fill_qty: f64, fill_price: f64) {
        let old_qty = self.quantity;
        let new_qty = old_qty + fill_qty;

        if old_qty.signum() == fill_qty.signum() || old_qty.abs() < 1e-10 {
            // 같은 방향 추가 → 가중평균
            let total_cost = self.avg_entry_price * old_qty.abs() + fill_price * fill_qty.abs();
            self.avg_entry_price = if new_qty.abs() > 1e-10 {
                total_cost / new_qty.abs()
            } else {
                0.0
            };
        } else {
            // 반대 방향 → 일부/전체 청산
            let closed_qty = fill_qty.abs().min(old_qty.abs());
            let pnl = closed_qty * (fill_price - self.avg_entry_price) * old_qty.signum();
            self.realized_pnl += pnl;

            // 완전 청산 후 반대 방향 진입
            if new_qty.abs() > 1e-10 && new_qty.signum() != old_qty.signum() {
                self.avg_entry_price = fill_price;
            }
        }

        self.quantity = new_qty;
    }

    /// 현재 시가로 미실현 PnL 갱신
    pub fn mark_to_market(&mut self, current_price: f64) {
        if self.quantity.abs() > 1e-10 {
            self.unrealized_pnl =
                self.quantity * (current_price - self.avg_entry_price);
        } else {
            self.unrealized_pnl = 0.0;
        }
    }

    /// 총 PnL (실현 + 미실현)
    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl
    }
}

/// Pre-trade 리스크 엔진
pub struct RiskEngine {
    config: RiskConfig,
    kill_switch: KillSwitch,

    /// 심볼별 포지션
    positions: HashMap<Symbol, PositionTracker>,

    /// 일일 누적 PnL
    daily_pnl: f64,

    /// 주문 rate limit 추적
    orders_this_second: u32,
    current_second: u64, // 초 단위 epoch

    /// 연속 주문 실패 카운터
    consecutive_failures: u32,
}

impl RiskEngine {
    pub fn new(config: RiskConfig) -> Self {
        let kill_switch = KillSwitch::new();
        Self {
            config,
            kill_switch,
            positions: HashMap::new(),
            daily_pnl: 0.0,
            orders_this_second: 0,
            current_second: 0,
            consecutive_failures: 0,
        }
    }

    /// 킬 스위치 공유 핸들
    pub fn kill_switch(&self) -> &KillSwitch {
        &self.kill_switch
    }

    /// 킬 스위치 공유 AtomicBool (Feed/Strategy에 전달)
    pub fn shared_kill_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.kill_switch.shared_flag()
    }

    /// **Pre-trade Risk Check**: 주문 전송 전 모든 리스크 규칙 검증
    ///
    /// 빠른 거부 순서 — 가장 저비용 체크(kill switch)부터 수행.
    /// O(1) 연산, 락 없음.
    pub fn check_order(&mut self, order: &OrderRequest, ts_ns: u64) -> Result<()> {
        // ── 1. Kill Switch (ns 단위 체크) ──
        if self.kill_switch.is_active() {
            return Err(ExecutionError::KillSwitchActive);
        }

        // ── 2. Daily PnL Limit ──
        self.refresh_daily_pnl();
        if self.daily_pnl < -self.config.max_daily_loss {
            self.kill_switch.activate(KillReason::DailyLossLimit);
            return Err(ExecutionError::DailyLossLimitReached {
                current_pnl: self.daily_pnl,
                limit: -self.config.max_daily_loss,
            });
        }

        // ── 3. Max Order Size ──
        if order.quantity.abs() > self.config.max_order_size {
            return Err(ExecutionError::RiskViolation {
                rule: "MAX_ORDER_SIZE",
                detail: format!(
                    "qty {:.4} > max {:.4}",
                    order.quantity.abs(),
                    self.config.max_order_size
                ),
            });
        }

        // ── 4. Per-symbol Position Limit ──
        let current_pos = self
            .positions
            .get(&order.symbol)
            .map(|p| p.quantity)
            .unwrap_or(0.0);
        let projected_pos = current_pos + order.quantity;

        if projected_pos.abs() > self.config.max_position_per_symbol {
            return Err(ExecutionError::PositionLimitExceeded {
                symbol: order.symbol.to_string(),
                current: projected_pos,
                max: self.config.max_position_per_symbol,
            });
        }

        // ── 5. Total Exposure Limit ──
        let total_exposure = self.calculate_total_exposure(order);
        if total_exposure > self.config.max_total_exposure {
            return Err(ExecutionError::RiskViolation {
                rule: "MAX_TOTAL_EXPOSURE",
                detail: format!(
                    "projected exposure {:.2}x > max {:.2}x",
                    total_exposure, self.config.max_total_exposure
                ),
            });
        }

        // ── 6. Order Rate Limit ──
        let current_second = ts_ns / 1_000_000_000;
        if current_second != self.current_second {
            self.current_second = current_second;
            self.orders_this_second = 0;
        }
        self.orders_this_second += 1;

        if self.orders_this_second > self.config.max_orders_per_second {
            return Err(ExecutionError::RiskViolation {
                rule: "ORDER_RATE_LIMIT",
                detail: format!(
                    "{} orders/sec > max {}",
                    self.orders_this_second, self.config.max_orders_per_second
                ),
            });
        }

        Ok(())
    }

    /// 체결 반영
    pub fn on_fill(&mut self, symbol: Symbol, qty: f64, price: f64) {
        let pos = self.positions.entry(symbol).or_default();
        pos.apply_fill(qty, price);

        // 연속 실패 리셋
        self.consecutive_failures = 0;

        tracing::debug!(
            symbol = %symbol,
            fill_qty = qty,
            fill_price = price,
            position = pos.quantity,
            realized_pnl = format!("{:.2}", pos.realized_pnl),
            "fill applied"
        );
    }

    /// 주문 실패 기록
    pub fn on_order_failure(&mut self) {
        self.consecutive_failures += 1;

        if self.consecutive_failures >= self.config.max_consecutive_failures {
            self.kill_switch
                .activate(KillReason::ConsecutiveOrderFailures);
        }
    }

    /// 시가 갱신 (mark-to-market)
    pub fn update_price(&mut self, symbol: Symbol, price: f64) {
        if let Some(pos) = self.positions.get_mut(&symbol) {
            pos.mark_to_market(price);
        }
    }

    /// 일일 PnL 재계산
    fn refresh_daily_pnl(&mut self) {
        self.daily_pnl = self
            .positions
            .values()
            .map(|p| p.total_pnl())
            .sum();
    }

    /// 예상 총 노출도 계산
    fn calculate_total_exposure(&self, new_order: &OrderRequest) -> f64 {
        let mut total_notional = 0.0;

        for (sym, pos) in &self.positions {
            let projected = if *sym == new_order.symbol {
                (pos.quantity + new_order.quantity).abs() * pos.avg_entry_price
            } else {
                pos.quantity.abs() * pos.avg_entry_price
            };
            total_notional += projected;
        }

        // 새 심볼이면 추가
        if !self.positions.contains_key(&new_order.symbol) {
            total_notional += new_order.quantity.abs() * new_order.price;
        }

        if self.config.total_capital > 0.0 {
            total_notional / self.config.total_capital
        } else {
            f64::INFINITY
        }
    }

    /// 포지션 조회
    pub fn position(&self, symbol: &Symbol) -> Option<&PositionTracker> {
        self.positions.get(symbol)
    }

    /// 전체 일일 PnL
    pub fn daily_pnl(&self) -> f64 {
        self.positions.values().map(|p| p.total_pnl()).sum()
    }

    /// 일일 리셋 (자정 또는 세션 시작)
    pub fn reset_daily(&mut self) {
        for pos in self.positions.values_mut() {
            pos.realized_pnl = 0.0;
        }
        self.daily_pnl = 0.0;
        self.orders_this_second = 0;
        self.consecutive_failures = 0;

        tracing::info!("daily risk counters reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oms::{OrderRequest, OrderSide, OrderType};

    fn make_order(symbol: &str, qty: f64, price: f64) -> OrderRequest {
        OrderRequest {
            symbol: Symbol::from_str(symbol),
            side: if qty > 0.0 {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            },
            order_type: OrderType::Limit,
            quantity: qty,
            price,
            time_in_force: crate::oms::TimeInForce::Ioc,
        }
    }

    #[test]
    fn test_order_passes_risk_check() {
        let mut risk = RiskEngine::new(RiskConfig::default());
        let order = make_order("BTCUSDT", 1.0, 50000.0);
        assert!(risk.check_order(&order, 0).is_ok());
    }

    #[test]
    fn test_max_order_size_rejected() {
        let mut risk = RiskEngine::new(RiskConfig {
            max_order_size: 5.0,
            ..Default::default()
        });
        let order = make_order("BTCUSDT", 10.0, 50000.0);
        assert!(matches!(
            risk.check_order(&order, 0),
            Err(ExecutionError::RiskViolation { rule: "MAX_ORDER_SIZE", .. })
        ));
    }

    #[test]
    fn test_daily_loss_triggers_kill_switch() {
        let mut risk = RiskEngine::new(RiskConfig {
            max_daily_loss: 100.0,
            ..Default::default()
        });

        let sym = Symbol::from_str("BTCUSDT");
        // 큰 손실 포지션 시뮬레이션
        risk.on_fill(sym, 10.0, 100.0);   // 100에 매수
        risk.update_price(sym, 85.0);       // 85로 하락 → 미실현 PnL = -150

        let order = make_order("BTCUSDT", 1.0, 85.0);
        let result = risk.check_order(&order, 0);
        assert!(matches!(result, Err(ExecutionError::DailyLossLimitReached { .. })));
        assert!(risk.kill_switch().is_active());
    }

    #[test]
    fn test_position_tracker_pnl() {
        let mut pos = PositionTracker::default();

        // 100에 10개 매수
        pos.apply_fill(10.0, 100.0);
        assert!((pos.quantity - 10.0).abs() < 1e-10);

        // 110에 5개 매도 (일부 청산)
        pos.apply_fill(-5.0, 110.0);
        assert!((pos.quantity - 5.0).abs() < 1e-10);
        assert!((pos.realized_pnl - 50.0).abs() < 1e-10); // (110-100) * 5
    }
}
