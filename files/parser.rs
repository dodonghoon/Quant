//! # Parser & Normalizer
//!
//! 거래소별 원장 JSON을 `MarketEvent`로 정규화합니다.
//! 각 거래소 어댑터는 `ExchangeParser` 트레이트를 구현합니다.
//!
//! ## 설계 포인트
//! - `serde_json::from_slice`: &[u8]에서 직접 파싱 (String 변환 없이)
//! - 타임스탬프는 즉시 나노초로 변환하여 일관성 유지

use crate::error::{IngestionError, Result};
use crate::types::*;
use serde::Deserialize;

/// 현재 시각을 나노초(epoch) 단위로 반환
#[inline]
pub fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos() as u64
}

// ────────────────────────────────────────────
// ExchangeParser 트레이트
// ────────────────────────────────────────────

/// 거래소별 파서가 구현해야 할 인터페이스.
/// 새 거래소 추가 시 이 트레이트만 구현하면 됩니다.
pub trait ExchangeParser: Send + Sync {
    /// 원장 바이트를 MarketEvent로 변환
    fn parse(&self, raw: &[u8]) -> Result<Option<MarketEvent>>;

    /// 구독 메시지 생성 (연결 직후 전송)
    fn subscription_message(&self, symbols: &[&str]) -> String;

    /// 거래소 식별자
    fn exchange(&self) -> Exchange;
}

// ────────────────────────────────────────────
// Binance 구현 예시
// ────────────────────────────────────────────

/// Binance WebSocket 스트림 파서
///
/// Binance의 combined stream 형식을 처리합니다:
/// `{"stream":"btcusdt@trade","data":{...}}`
pub struct BinanceParser;

/// Binance combined stream envelope
#[derive(Deserialize)]
struct BinanceEnvelope<'a> {
    stream: &'a str,
    data: serde_json::Value,
}

/// Binance Trade event (`@trade` 스트림)
#[derive(Deserialize)]
struct BinanceTradeRaw {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    qty: String,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
    #[serde(rename = "T")]
    trade_time_ms: u64,
}

/// Binance BookTicker event (`@bookTicker` 스트림)
#[derive(Deserialize)]
struct BinanceBboRaw {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "b")]
    bid_price: String,
    #[serde(rename = "B")]
    bid_qty: String,
    #[serde(rename = "a")]
    ask_price: String,
    #[serde(rename = "A")]
    ask_qty: String,
    #[serde(rename = "u")]
    _update_id: Option<u64>,
}

impl BinanceParser {
    /// 문자열 가격을 f64로 파싱 (Binance는 모든 숫자를 문자열로 전송)
    #[inline]
    fn parse_f64(s: &str, field: &'static str) -> Result<f64> {
        s.parse::<f64>().map_err(|_| IngestionError::ParseError {
            context: format!("Binance {field} = \"{s}\""),
            source: serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid float: {s}"),
            )),
        })
    }

    fn parse_trade(&self, data: &serde_json::Value, local_ts: u64) -> Result<MarketEvent> {
        let raw: BinanceTradeRaw =
            serde_json::from_value(data.clone()).map_err(|e| IngestionError::ParseError {
                context: "Binance trade".into(),
                source: e,
            })?;

        Ok(MarketEvent::Trade(Trade {
            symbol: Symbol::from_str(&raw.symbol),
            exchange: Exchange::Binance,
            price: Self::parse_f64(&raw.price, "price")?,
            quantity: Self::parse_f64(&raw.qty, "qty")?,
            aggressor_side: if raw.is_buyer_maker {
                Side::Ask
            } else {
                Side::Bid
            },
            exchange_ts_ns: raw.trade_time_ms * 1_000_000, // ms → ns
            local_ts_ns: local_ts,
        }))
    }

    fn parse_bbo(&self, data: &serde_json::Value, local_ts: u64) -> Result<MarketEvent> {
        let raw: BinanceBboRaw =
            serde_json::from_value(data.clone()).map_err(|e| IngestionError::ParseError {
                context: "Binance bookTicker".into(),
                source: e,
            })?;

        Ok(MarketEvent::Bbo(BboSnapshot {
            symbol: Symbol::from_str(&raw.symbol),
            exchange: Exchange::Binance,
            bid_price: Self::parse_f64(&raw.bid_price, "bid_price")?,
            bid_qty: Self::parse_f64(&raw.bid_qty, "bid_qty")?,
            ask_price: Self::parse_f64(&raw.ask_price, "ask_price")?,
            ask_qty: Self::parse_f64(&raw.ask_qty, "ask_qty")?,
            exchange_ts_ns: local_ts, // bookTicker엔 서버 ts 없음
            local_ts_ns: local_ts,
        }))
    }
}

impl ExchangeParser for BinanceParser {
    fn parse(&self, raw: &[u8]) -> Result<Option<MarketEvent>> {
        let local_ts = now_ns();

        // 빈 메시지 또는 ping/pong 무시
        if raw.is_empty() {
            return Ok(None);
        }

        let envelope: BinanceEnvelope =
            serde_json::from_slice(raw).map_err(|e| IngestionError::ParseError {
                context: "Binance envelope".into(),
                source: e,
            })?;

        // 스트림 이름으로 이벤트 타입 분기
        if envelope.stream.ends_with("@trade") {
            self.parse_trade(&envelope.data, local_ts).map(Some)
        } else if envelope.stream.ends_with("@bookTicker") {
            self.parse_bbo(&envelope.data, local_ts).map(Some)
        } else {
            // 미지원 스트림은 조용히 무시 (로깅만)
            tracing::debug!(stream = envelope.stream, "unhandled stream type");
            Ok(None)
        }
    }

    fn subscription_message(&self, symbols: &[&str]) -> String {
        let streams: Vec<String> = symbols
            .iter()
            .flat_map(|s| {
                let lower = s.to_lowercase();
                vec![format!("{lower}@trade"), format!("{lower}@bookTicker")]
            })
            .collect();

        serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": 1
        })
        .to_string()
    }

    fn exchange(&self) -> Exchange {
        Exchange::Binance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_trade_parse() {
        let parser = BinanceParser;
        let msg = br#"{"stream":"btcusdt@trade","data":{"e":"trade","E":1234567890123,"s":"BTCUSDT","t":12345,"p":"50000.00","q":"0.001","T":1234567890123,"m":false}}"#;

        let event = parser.parse(msg).unwrap().unwrap();
        match event {
            MarketEvent::Trade(t) => {
                assert_eq!(t.symbol.as_str(), "BTCUSDT");
                assert!((t.price - 50000.0).abs() < f64::EPSILON);
                assert_eq!(t.aggressor_side, Side::Bid);
            }
            _ => panic!("expected Trade event"),
        }
    }

    #[test]
    fn test_binance_bbo_parse() {
        let parser = BinanceParser;
        let msg = br#"{"stream":"ethusdt@bookTicker","data":{"s":"ETHUSDT","b":"3000.00","B":"1.5","a":"3001.00","A":"2.0","u":12345}}"#;

        let event = parser.parse(msg).unwrap().unwrap();
        match event {
            MarketEvent::Bbo(bbo) => {
                assert!((bbo.bid_price - 3000.0).abs() < f64::EPSILON);
                assert!((bbo.ask_price - 3001.0).abs() < f64::EPSILON);
            }
            _ => panic!("expected Bbo event"),
        }
    }

    #[test]
    fn test_subscription_message() {
        let parser = BinanceParser;
        let msg = parser.subscription_message(&["btcusdt", "ethusdt"]);
        assert!(msg.contains("btcusdt@trade"));
        assert!(msg.contains("ethusdt@bookTicker"));
    }
}
