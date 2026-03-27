//! # Strategy Engine
//!
//! 퀀트 트레이딩 시스템의 전략 연산 계층.
//!
//! ## 모듈 구성
//! - `features`: Rolling Window, EMA — 온라인 스트리밍 통계 (O(1) per update)
//! - `kalman`: 1D Kalman Filter — 시장 노이즈 제거, True Price 추정
//! - `ou_model`: Ornstein-Uhlenbeck 프로세스 — 평균 회귀 모델링 (Pairs Trading)
//! - `signal`: Signal Generator — 알파 소스 결합, 최종 트레이딩 신호
//! - `engine`: 오케스트레이터 — 링 버퍼 소비 → 전체 파이프라인 실행
//!
//! ## 데이터 흐름
//! ```text
//! MarketEvent (BBO/Trade)
//!     │
//!     ├─→ SymbolState
//!     │     ├─ Kalman Filter → filtered price
//!     │     ├─ Rolling Window → volatility, z-score
//!     │     └─ EMA (fast/slow) → trend detection
//!     │
//!     ├─→ PairState
//!     │     ├─ Spread = A - ratio × B
//!     │     └─ OU Model → z-score, kappa, half-life
//!     │
//!     └─→ Signal Generator
//!           ├─ Composite Z-Score (weighted)
//!           ├─ Direction (StrongBuy..StrongSell)
//!           └─ Confidence & Raw Position Fraction
//! ```

pub mod engine;
pub mod error;
pub mod features;
pub mod garch;
pub mod gbm;
pub mod kalman;
pub mod onnx_inference;
pub mod ou_model;
#[cfg(feature = "python")]
pub mod pyo3_bridge;
pub mod signal;

// 편의 re-export
pub use engine::{ChannelSink, EngineConfig, LoggingSink, SignalSink, StrategyEngine};
pub use error::{Result, StrategyError};
pub use features::{Ema, RollingWindow};
pub use garch::{GarchConfig, GarchFilter, GarchOutput};
pub use gbm::{GbmConfig, GbmPath, GbmSimulator, MonteCarloResult};
pub use kalman::{KalmanConfig, KalmanFilter, KalmanOutput};
pub use onnx_inference::{OnnxModelMeta, OnnxPrediction, OnnxPredictor, PriceDirection};
pub use ou_model::{OuConfig, OuModel, OuParams, OuSignal};
pub use signal::{
    AlphaBreakdown, SignalConfig, SignalDirection, SignalGenerator, TradingSignal,
};
