//! # Data Ingestion Layer
//!
//! 퀀트 트레이딩 시스템의 데이터 수집 계층.
//! 거래소 WebSocket에서 실시간 시장 데이터를 수신하고,
//! 정규화된 `MarketEvent`로 변환하여 SPSC 링 버퍼로 전달합니다.
//!
//! ## 모듈 구성
//! - `types`: 거래소-독립적 정규화 데이터 타입 (`MarketEvent`, `Symbol`, `Trade`, `BboSnapshot`)
//! - `error`: 데이터 수집 전용 에러 타입 (`IngestionError`)
//! - `parser`: 거래소별 파서 (`ExchangeParser` 트레이트 + Binance/Upbit 구현)
//! - `feed`: WebSocket Feed Handler (비동기 연결, 재연결, 링 버퍼 push)
//!
//! ## 데이터 흐름
//! ```text
//! Exchange WebSocket
//!     │
//!     ▼
//! FeedHandler.run()          ← tokio async task (I/O bound)
//!     │
//!     ├─ ExchangeParser.parse()  ← 원장 JSON → MarketEvent
//!     │
//!     ▼
//! rtrb::Producer.push()      ← lock-free SPSC ring buffer
//!     │
//!     ▼
//! Consumer (OS thread)       ← Strategy Engine으로 전달
//! ```

pub mod error;
pub mod feed;
pub mod parser;
pub mod questdb;
pub mod redis_store;
pub mod types;

// ── 편의 re-export ──
pub use error::{IngestionError, Result};
pub use feed::start_upbit_websocket;
pub use parser::{BinanceParser, ExchangeParser, UpbitParser};
pub use questdb::{QuestDbReader, QuestDbWriter};
pub use redis_store::RedisTickStore;
pub use types::{BboSnapshot, BookUpdate, Exchange, MarketEvent, Side, Symbol, Trade};
