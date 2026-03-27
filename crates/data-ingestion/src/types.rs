//! # Normalized Market Data Types
//!
//! 거래소별 원장 데이터를 거래소-독립적(Exchange-Agnostic) 포맷으로 정규화합니다.
//! 다운스트림(Strategy Engine)은 이 타입만 알면 됩니다.
//!
//! ## 설계 원칙
//! - `Copy` 트레이트: 링 버퍼 전달 시 힙 할당 없이 스택 복사
//! - 고정 크기: `String` 대신 고정 길이 배열로 심볼 표현 (할당 제거)
//! - 나노초 타임스탬프: 거래소 원본 + 로컬 수신 시각 이중 기록

use serde::{Deserialize, Serialize};
use std::fmt;

// ────────────────────────────────────────────
// Symbol: 힙 할당 없는 고정 크기 심볼 표현
// ────────────────────────────────────────────

/// 최대 16바이트 고정 크기 심볼 (e.g., "BTC-USDT", "AAPL")
/// `String` 할당을 피하기 위해 스택 기반 고정 배열 사용.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    bytes: [u8; 16],
    len: u8,
}

impl Symbol {
    /// 문자열로부터 Symbol 생성. 16바이트 초과 시 잘림(truncate).
    pub fn from_str(s: &str) -> Self {
        let mut bytes = [0u8; 16];
        let len = s.len().min(16);
        bytes[..len].copy_from_slice(&s.as_bytes()[..len]);
        Self {
            bytes,
            len: len as u8,
        }
    }

    pub fn as_str(&self) -> &str {
        // Safety: Symbol은 항상 유효한 UTF-8에서 생성됨
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("???")
    }
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Symbol(\"{}\")", self.as_str())
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ────────────────────────────────────────────
// Side & MarketEvent Enum
// ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exchange {
    Binance,
    Upbit,
    Bybit,
    Unknown,
}

// ────────────────────────────────────────────
// Core Market Data Types (모두 Copy)
// ────────────────────────────────────────────

/// 호가 변경 (Level 2)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BookUpdate {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    /// 거래소 원본 타임스탬프 (나노초, epoch)
    pub exchange_ts_ns: u64,
    /// 로컬 수신 타임스탬프 (나노초) — 지연 측정용
    pub local_ts_ns: u64,
}

/// 체결 (Trade)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub price: f64,
    pub quantity: f64,
    pub aggressor_side: Side,
    pub exchange_ts_ns: u64,
    pub local_ts_ns: u64,
}

/// BBO (Best Bid/Offer) 스냅샷 — 가장 빈번하게 사용
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BboSnapshot {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub bid_price: f64,
    pub bid_qty: f64,
    pub ask_price: f64,
    pub ask_qty: f64,
    pub exchange_ts_ns: u64,
    pub local_ts_ns: u64,
}

// ────────────────────────────────────────────
// Unified Event Envelope
// ────────────────────────────────────────────

/// 모든 시장 이벤트의 통합 래퍼.
/// 링 버퍼를 통해 이 타입 하나만 전달됩니다.
///
/// 크기: ~128 bytes (스택), Copy 가능 → 링 버퍼 push 시 `memcpy`로 완결.
#[derive(Debug, Clone, Copy)]
pub enum MarketEvent {
    Book(BookUpdate),
    Trade(Trade),
    Bbo(BboSnapshot),
    /// 하트비트 / 연결 상태 신호
    Heartbeat { exchange: Exchange, ts_ns: u64 },
}

impl MarketEvent {
    /// 이벤트의 로컬 수신 타임스탬프 추출 (지연 모니터링)
    pub fn local_ts_ns(&self) -> u64 {
        match self {
            Self::Book(b) => b.local_ts_ns,
            Self::Trade(t) => t.local_ts_ns,
            Self::Bbo(s) => s.local_ts_ns,
            Self::Heartbeat { ts_ns, .. } => *ts_ns,
        }
    }

    /// 이벤트가 속한 거래소
    pub fn exchange(&self) -> Exchange {
        match self {
            Self::Book(b) => b.exchange,
            Self::Trade(t) => t.exchange,
            Self::Bbo(s) => s.exchange,
            Self::Heartbeat { exchange, .. } => *exchange,
        }
    }
}
