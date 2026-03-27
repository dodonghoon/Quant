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

// ────────────────────────────────────────────
// Upbit 구현
// ────────────────────────────────────────────

/// Upbit WebSocket 스트림 파서
///
/// Upbit은 JSON 형식으로 체결/호가 데이터를 전송합니다.
/// 연결 후 `[{"ticket":"..."}, {"type":"trade","codes":["KRW-BTC"]}, ...]`
/// 형식의 구독 메시지를 보내야 합니다.
///
/// ## 메시지 포맷
/// Trade: `{"type":"trade","code":"KRW-BTC","trade_price":50000.0,...}`
/// Orderbook: `{"type":"orderbook","code":"KRW-BTC","orderbook_units":[...],...}`
pub struct UpbitParser;

/// Upbit 공통 메시지 (type 필드로 분기)
#[derive(Deserialize)]
struct UpbitMessage<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    code: Option<String>,
}

/// Upbit Trade 이벤트
#[derive(Deserialize)]
struct UpbitTradeRaw {
    code: String,
    trade_price: f64,
    trade_volume: f64,
    /// "ASK" 또는 "BID"
    ask_bid: String,
    /// 체결 타임스탬프 (밀리초)
    timestamp: u64,
}

/// Upbit Orderbook 이벤트 (최우선 호가만 사용)
#[derive(Deserialize)]
struct UpbitOrderbookRaw {
    code: String,
    timestamp: u64,
    orderbook_units: Vec<UpbitOrderbookUnit>,
}

#[derive(Deserialize)]
struct UpbitOrderbookUnit {
    ask_price: f64,
    bid_price: f64,
    ask_size: f64,
    bid_size: f64,
}

impl ExchangeParser for UpbitParser {
    fn parse(&self, raw: &[u8]) -> Result<Option<MarketEvent>> {
        let local_ts = now_ns();

        if raw.is_empty() {
            return Ok(None);
        }

        // Upbit은 구독 응답으로 `{"status":"UP"}` 등을 보낼 수 있음
        let msg: UpbitMessage =
            serde_json::from_slice(raw).map_err(|e| IngestionError::ParseError {
                context: "Upbit message".into(),
                source: e,
            })?;

        match msg.msg_type {
            "trade" => {
                let trade: UpbitTradeRaw = serde_json::from_slice(raw)
                    .map_err(|e| IngestionError::ParseError {
                        context: "Upbit trade".into(),
                        source: e,
                    })?;

                // Upbit 코드 "KRW-BTC" → 심볼 변환
                let symbol_str = trade.code.replace('-', "");

                Ok(Some(MarketEvent::Trade(Trade {
                    symbol: Symbol::from_str(&symbol_str),
                    exchange: Exchange::Upbit,
                    price: trade.trade_price,
                    quantity: trade.trade_volume,
                    aggressor_side: if trade.ask_bid == "ASK" {
                        Side::Ask
                    } else {
                        Side::Bid
                    },
                    exchange_ts_ns: trade.timestamp * 1_000_000, // ms → ns
                    local_ts_ns: local_ts,
                })))
            }
            "orderbook" => {
                let ob: UpbitOrderbookRaw = serde_json::from_slice(raw)
                    .map_err(|e| IngestionError::ParseError {
                        context: "Upbit orderbook".into(),
                        source: e,
                    })?;

                // 최우선 호가 (첫 번째 유닛)만 BBO로 변환
                let unit = ob.orderbook_units.first().ok_or_else(|| {
                    IngestionError::ParseError {
                        context: "Upbit orderbook: empty units".into(),
                        source: serde_json::Error::io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "empty orderbook_units",
                        )),
                    }
                })?;

                let symbol_str = ob.code.replace('-', "");

                Ok(Some(MarketEvent::Bbo(BboSnapshot {
                    symbol: Symbol::from_str(&symbol_str),
                    exchange: Exchange::Upbit,
                    bid_price: unit.bid_price,
                    bid_qty: unit.bid_size,
                    ask_price: unit.ask_price,
                    ask_qty: unit.ask_size,
                    exchange_ts_ns: ob.timestamp * 1_000_000,
                    local_ts_ns: local_ts,
                })))
            }
            _ => {
                // 미지원 타입 (status, ping 등) 무시
                tracing::debug!(msg_type = msg.msg_type, "unhandled Upbit message type");
                Ok(None)
            }
        }
    }

    fn subscription_message(&self, symbols: &[&str]) -> String {
        // Upbit 구독 형식: [{"ticket":"uuid"},{"type":"trade","codes":["KRW-BTC"]},{"type":"orderbook","codes":["KRW-BTC"]}]
        let codes: Vec<String> = symbols
            .iter()
            .map(|s| {
                // "btckrw" → "KRW-BTC" 변환 시도, 이미 올바른 형식이면 그대로
                let upper = s.to_uppercase();
                if upper.contains('-') {
                    upper
                } else {
                    // "KRWBTC" → "KRW-BTC" 형태로 변환
                    format!("KRW-{}", upper.trim_start_matches("KRW"))
                }
            })
            .collect();

        serde_json::json!([
            {"ticket": "data-ingestion-001"},
            {"type": "trade", "codes": codes},
            {"type": "orderbook", "codes": codes}
        ])
        .to_string()
    }

    fn exchange(&self) -> Exchange {
        Exchange::Upbit
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

    // ── Upbit 테스트 ──

    #[test]
    fn test_upbit_trade_parse() {
        let parser = UpbitParser;
        let msg = br#"{"type":"trade","code":"KRW-BTC","trade_price":95000000.0,"trade_volume":0.005,"ask_bid":"BID","timestamp":1700000000123}"#;

        let event = parser.parse(msg).unwrap().unwrap();
        match event {
            MarketEvent::Trade(t) => {
                assert_eq!(t.symbol.as_str(), "KRWBTC");
                assert!((t.price - 95_000_000.0).abs() < f64::EPSILON);
                assert_eq!(t.aggressor_side, Side::Bid);
                assert_eq!(t.exchange, Exchange::Upbit);
            }
            _ => panic!("expected Trade event"),
        }
    }

    #[test]
    fn test_upbit_orderbook_parse() {
        let parser = UpbitParser;
        let msg = br#"{"type":"orderbook","code":"KRW-ETH","timestamp":1700000000456,"orderbook_units":[{"ask_price":4500000.0,"bid_price":4499000.0,"ask_size":1.2,"bid_size":3.5},{"ask_price":4501000.0,"bid_price":4498000.0,"ask_size":0.5,"bid_size":2.0}]}"#;

        let event = parser.parse(msg).unwrap().unwrap();
        match event {
            MarketEvent::Bbo(bbo) => {
                assert!((bbo.bid_price - 4_499_000.0).abs() < f64::EPSILON);
                assert!((bbo.ask_price - 4_500_000.0).abs() < f64::EPSILON);
                assert_eq!(bbo.exchange, Exchange::Upbit);
            }
            _ => panic!("expected Bbo event"),
        }
    }

    #[test]
    fn test_upbit_unknown_type_ignored() {
        let parser = UpbitParser;
        let msg = br#"{"type":"status","code":null}"#;
        let result = parser.parse(msg).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_upbit_subscription_message() {
        let parser = UpbitParser;
        let msg = parser.subscription_message(&["KRW-BTC", "KRW-ETH"]);
        assert!(msg.contains("KRW-BTC"));
        assert!(msg.contains("trade"));
        assert!(msg.contains("orderbook"));
    }
}
