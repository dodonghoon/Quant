//! # Order Management System (OMS)
//!
//! 주문의 전체 생명주기를 관리합니다:
//! ```text
//! Signal → OrderRequest → [Risk Check] → Order(Pending)
//!     → Sent → PartialFill / Filled / Cancelled / Rejected
//! ```
//!
//! ## 설계 원칙
//! - **상태 머신**: 유효한 상태 전이만 허용 (컴파일 타임 검증은 아니지만 런타임 체크)
//! - **고유 주문 ID**: 단조 증가 카운터 (AtomicU64)
//! - **거래소 추상화**: `ExchangeGateway` 트레이트로 거래소별 구현 분리

use crate::error::{ExecutionError, Result};
use data_ingestion::types::Symbol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

// ────────────────────────────────────────────
// Order Types & Enums
// ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    /// IOC (Immediate or Cancel) — 즉시 체결 안 되면 취소
    Ioc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good Till Cancel
    Gtc,
    /// Immediate or Cancel
    Ioc,
    /// Fill or Kill
    Fok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// 리스크 체크 통과, 전송 대기
    Pending,
    /// 거래소에 전송됨
    Sent,
    /// 부분 체결
    PartialFill,
    /// 완전 체결
    Filled,
    /// 취소됨 (사용자 요청 또는 IOC 미체결)
    Cancelled,
    /// 거래소에서 거부됨
    Rejected,
}

impl OrderStatus {
    /// 유효한 상태 전이인지 확인
    pub fn can_transition_to(&self, next: OrderStatus) -> bool {
        matches!(
            (self, next),
            (Self::Pending, OrderStatus::Sent)
                | (Self::Pending, OrderStatus::Rejected)
                | (Self::Pending, OrderStatus::Cancelled) // 전송 전 취소
                | (Self::Sent, OrderStatus::PartialFill)
                | (Self::Sent, OrderStatus::Filled)
                | (Self::Sent, OrderStatus::Cancelled)
                | (Self::Sent, OrderStatus::Rejected)
                | (Self::PartialFill, OrderStatus::Filled)
                | (Self::PartialFill, OrderStatus::Cancelled)
        )
    }

    /// 종결 상태인지 (더 이상 전이 불가)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Filled | Self::Cancelled | Self::Rejected)
    }
}

/// 주문 요청 (Strategy Engine → Execution)
#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub symbol: Symbol,
    pub side: OrderSide,
    pub order_type: OrderType,
    /// 양수 수량 (side로 방향 결정)
    pub quantity: f64,
    /// Limit 가격 (Market이면 무시)
    pub price: f64,
    pub time_in_force: TimeInForce,
}

/// 내부 주문 객체 (생명주기 추적)
#[derive(Debug, Clone)]
pub struct Order {
    /// 내부 고유 ID (단조 증가)
    pub internal_id: u64,
    /// 거래소 주문 ID (전송 후 설정)
    pub exchange_id: Option<String>,
    pub request: OrderRequest,
    pub status: OrderStatus,
    /// 체결된 수량
    pub filled_qty: f64,
    /// 가중평균 체결가
    pub avg_fill_price: f64,
    /// 생성 시각 (나노초)
    pub created_at_ns: u64,
    /// 마지막 업데이트 시각 (나노초)
    pub updated_at_ns: u64,
}

/// 체결 보고서 (거래소 → OMS)
#[derive(Debug, Clone)]
pub struct FillReport {
    pub internal_id: u64,
    pub exchange_id: String,
    pub filled_qty: f64,
    pub fill_price: f64,
    pub is_final: bool, // true면 완전 체결
    pub ts_ns: u64,
}

// ────────────────────────────────────────────
// Exchange Gateway Trait
// ────────────────────────────────────────────

/// 거래소 통신 추상화.
/// 각 거래소(Binance, Upbit 등)가 이 트레이트를 구현합니다.
#[allow(async_fn_in_trait)]
pub trait ExchangeGateway: Send + Sync {
    /// 주문 전송
    async fn send_order(&self, order: &Order) -> Result<String>; // 거래소 ID 반환

    /// 주문 취소
    async fn cancel_order(&self, exchange_id: &str, symbol: &Symbol) -> Result<()>;

    /// 거래소 이름
    fn name(&self) -> &str;
}

/// 시뮬레이션용 더미 거래소 (백테스팅/테스트)
pub struct SimulatedGateway {
    /// 즉시 체결 가정
    fill_probability: f64,
    /// 시뮬레이션 지연 (마이크로초)
    latency_us: u64,
}

impl SimulatedGateway {
    pub fn instant_fill() -> Self {
        Self {
            fill_probability: 1.0,
            latency_us: 0,
        }
    }

    pub fn with_latency(latency_us: u64) -> Self {
        Self {
            fill_probability: 1.0,
            latency_us,
        }
    }
}

impl ExchangeGateway for SimulatedGateway {
    async fn send_order(&self, order: &Order) -> Result<String> {
        if self.latency_us > 0 {
            tokio::time::sleep(tokio::time::Duration::from_micros(self.latency_us)).await;
        }
        // 시뮬레이션: 항상 성공, ID는 내부 ID 기반
        Ok(format!("SIM-{}", order.internal_id))
    }

    async fn cancel_order(&self, _exchange_id: &str, _symbol: &Symbol) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "Simulated"
    }
}

// ────────────────────────────────────────────
// Order Manager
// ────────────────────────────────────────────

/// 단조 증가 주문 ID 생성기
static ORDER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_order_id() -> u64 {
    ORDER_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// 주문 관리자
///
/// 주문의 전체 생명주기를 추적하고 상태 전이를 관리합니다.
pub struct OrderManager {
    /// 활성 주문 (terminal 상태가 아닌 주문들)
    active_orders: HashMap<u64, Order>,
    /// 완료된 주문 (최근 N개 보관, 감사 추적용)
    completed_orders: Vec<Order>,
    /// 완료 주문 보관 한도
    max_completed_history: usize,
}

impl OrderManager {
    pub fn new(max_completed_history: usize) -> Self {
        Self {
            active_orders: HashMap::new(),
            completed_orders: Vec::new(),
            max_completed_history,
        }
    }

    /// 새 주문 생성 (Pending 상태)
    pub fn create_order(&mut self, request: OrderRequest, ts_ns: u64) -> u64 {
        let id = next_order_id();
        let order = Order {
            internal_id: id,
            exchange_id: None,
            request,
            status: OrderStatus::Pending,
            filled_qty: 0.0,
            avg_fill_price: 0.0,
            created_at_ns: ts_ns,
            updated_at_ns: ts_ns,
        };

        tracing::debug!(
            order_id = id,
            symbol = %order.request.symbol,
            side = ?order.request.side,
            qty = order.request.quantity,
            price = order.request.price,
            "order created"
        );

        self.active_orders.insert(id, order);
        id
    }

    /// 상태 전이
    pub fn transition(
        &mut self,
        order_id: u64,
        new_status: OrderStatus,
        ts_ns: u64,
    ) -> Result<()> {
        let order = self
            .active_orders
            .get_mut(&order_id)
            .ok_or(ExecutionError::UnknownOrderId(order_id.to_string()))?;

        if !order.status.can_transition_to(new_status) {
            return Err(ExecutionError::InvalidStateTransition {
                from: format!("{:?}", order.status),
                to: format!("{:?}", new_status),
            });
        }

        let old_status = order.status;
        order.status = new_status;
        order.updated_at_ns = ts_ns;

        tracing::debug!(
            order_id = order_id,
            from = ?old_status,
            to = ?new_status,
            "order state transition"
        );

        // 종결 상태면 활성 목록에서 완료 목록으로 이동
        if new_status.is_terminal() {
            if let Some(completed) = self.active_orders.remove(&order_id) {
                if self.completed_orders.len() >= self.max_completed_history {
                    self.completed_orders.remove(0); // FIFO
                }
                self.completed_orders.push(completed);
            }
        }

        Ok(())
    }

    /// 거래소 ID 설정 (전송 성공 시)
    pub fn set_exchange_id(&mut self, order_id: u64, exchange_id: String) -> Result<()> {
        let order = self
            .active_orders
            .get_mut(&order_id)
            .ok_or(ExecutionError::UnknownOrderId(order_id.to_string()))?;

        order.exchange_id = Some(exchange_id);
        Ok(())
    }

    /// 체결 보고서 적용
    pub fn apply_fill(&mut self, report: &FillReport) -> Result<FillResult> {
        let order = self
            .active_orders
            .get_mut(&report.internal_id)
            .ok_or(ExecutionError::UnknownOrderId(report.internal_id.to_string()))?;

        // 가중평균 체결가 업데이트
        let total_filled_notional =
            order.avg_fill_price * order.filled_qty + report.fill_price * report.filled_qty;
        order.filled_qty += report.filled_qty;
        order.avg_fill_price = if order.filled_qty > 0.0 {
            total_filled_notional / order.filled_qty
        } else {
            0.0
        };

        // 상태 전이
        let new_status = if report.is_final || order.filled_qty >= order.request.quantity {
            OrderStatus::Filled
        } else {
            OrderStatus::PartialFill
        };

        let symbol = order.request.symbol;
        let side = order.request.side;
        let filled_qty = report.filled_qty;
        let fill_price = report.fill_price;

        self.transition(report.internal_id, new_status, report.ts_ns)?;

        Ok(FillResult {
            symbol,
            side,
            filled_qty,
            fill_price,
            is_complete: new_status == OrderStatus::Filled,
        })
    }

    /// 활성 주문 조회
    pub fn get_order(&self, order_id: u64) -> Option<&Order> {
        self.active_orders.get(&order_id)
    }

    /// 특정 심볼의 활성 주문 목록
    pub fn active_orders_for(&self, symbol: &Symbol) -> Vec<&Order> {
        self.active_orders
            .values()
            .filter(|o| o.request.symbol == *symbol)
            .collect()
    }

    /// 모든 활성 주문 취소 요청 (킬 스위치 시)
    pub fn cancel_all(&mut self, ts_ns: u64) -> Vec<Order> {
        let ids: Vec<u64> = self.active_orders.keys().copied().collect();
        let mut cancelled = Vec::new();

        for id in ids {
            if let Ok(()) = self.transition(id, OrderStatus::Cancelled, ts_ns) {
                // transition이 completed로 이동시킴
                if let Some(order) = self.completed_orders.last() {
                    if order.internal_id == id {
                        cancelled.push(order.clone());
                    }
                }
            }
        }

        tracing::warn!(count = cancelled.len(), "cancelled all active orders");
        cancelled
    }

    /// 활성 주문 수
    pub fn active_count(&self) -> usize {
        self.active_orders.len()
    }
}

/// 체결 결과 (Risk Engine에 전달)
#[derive(Debug, Clone)]
pub struct FillResult {
    pub symbol: Symbol,
    pub side: OrderSide,
    pub filled_qty: f64,
    pub fill_price: f64,
    pub is_complete: bool,
}

impl FillResult {
    /// Risk Engine에 전달할 부호 있는 수량 (Buy: +, Sell: -)
    pub fn signed_qty(&self) -> f64 {
        match self.side {
            OrderSide::Buy => self.filled_qty,
            OrderSide::Sell => -self.filled_qty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> OrderRequest {
        OrderRequest {
            symbol: Symbol::from_str("BTCUSDT"),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: 1.0,
            price: 50000.0,
            time_in_force: TimeInForce::Ioc,
        }
    }

    #[test]
    fn test_order_lifecycle() {
        let mut oms = OrderManager::new(100);

        // 생성 → Pending
        let id = oms.create_order(make_request(), 0);
        assert_eq!(oms.get_order(id).unwrap().status, OrderStatus::Pending);

        // Pending → Sent
        oms.transition(id, OrderStatus::Sent, 1).unwrap();
        assert_eq!(oms.get_order(id).unwrap().status, OrderStatus::Sent);

        // Sent → Filled
        let report = FillReport {
            internal_id: id,
            exchange_id: "EX-123".into(),
            filled_qty: 1.0,
            fill_price: 50000.0,
            is_final: true,
            ts_ns: 2,
        };
        let fill = oms.apply_fill(&report).unwrap();
        assert!(fill.is_complete);

        // 활성 목록에서 제거됨
        assert!(oms.get_order(id).is_none());
        assert_eq!(oms.active_count(), 0);
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let mut oms = OrderManager::new(100);
        let id = oms.create_order(make_request(), 0);

        // Pending → Filled (유효하지 않은 전이)
        let result = oms.transition(id, OrderStatus::Filled, 1);
        assert!(matches!(
            result,
            Err(ExecutionError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn test_cancel_all() {
        let mut oms = OrderManager::new(100);
        let _id1 = oms.create_order(make_request(), 0);
        let _id2 = oms.create_order(make_request(), 0);

        assert_eq!(oms.active_count(), 2);

        let cancelled = oms.cancel_all(1);
        assert_eq!(cancelled.len(), 2);
        assert_eq!(oms.active_count(), 0);
    }
}
