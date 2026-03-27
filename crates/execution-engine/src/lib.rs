//! # Execution & Risk Engine
//!
//! 퀀트 트레이딩 시스템의 주문 집행 및 리스크 관리 계층.
//!
//! ## 모듈 구성
//! - `error`: 집행 전용 에러 타입 (`ExecutionError`)
//! - `kelly`: Kelly Criterion 포지션 사이징 (이산/연속 모형)
//! - `risk`: Pre-trade 리스크 엔진 (6단계 검증)
//! - `kill_switch`: 전역 비상정지 (AtomicBool, 락프리)
//! - `oms`: Order Management System (상태 머신, 부분 체결)
//! - `executor`: 집행 오케스트레이터 (Signal → Kelly → Risk → OMS)
//! - `almgren_chriss`: 최적 집행 알고리즘 (Market Impact 최소화)
//!
//! ## 파이프라인
//! ```text
//! TradingSignal (crossbeam)
//!     │
//!     ▼
//! [1. Kelly Sizer] → optimal position fraction
//!     │
//!     ▼
//! [2. Position Delta] → 현재 vs 목표 차이
//!     │
//!     ▼
//! [3. Risk Check] → pre-trade 리스크 검증
//!     │
//!     ▼
//! [4. Almgren-Chriss] → 최적 집행 경로
//!     │
//!     ▼
//! [5. OMS] → 주문 생성 & 상태 관리
//!     │
//!     ▼
//! [6. Gateway] → 거래소 전송
//! ```

pub mod almgren_chriss;
pub mod redis_bridge;
pub mod error;
pub mod executor;
pub mod gateway;
pub mod kelly;
pub mod kill_switch;
pub mod oms;
pub mod risk;

// ── 편의 re-export ──
pub use almgren_chriss::{AlmgrenChrissConfig, AlmgrenChrissModel, ExecutionSchedule};
pub use error::{ExecutionError, Result};
pub use executor::{ExecutionConfig, ExecutionEngine, ExecutionStats};
pub use gateway::{Gateway, Order as GatewayOrder, OrderResponse};
pub use kelly::{KellyConfig, KellyOutput, KellySizer};
pub use kill_switch::{KillReason, KillSwitch};
pub use oms::{
    ExchangeGateway, FillReport, Order, OrderManager, OrderRequest, OrderSide, OrderStatus,
    OrderType, SimulatedGateway, TimeInForce,
};
pub use risk::{PositionTracker, RiskConfig, RiskEngine};
