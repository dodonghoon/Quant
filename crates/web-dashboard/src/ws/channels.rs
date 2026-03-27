//! # WebSocket 채널 관리
//!
//! 구독 모델 관리 및 이벤트 필터링.

use serde::{Deserialize, Serialize};

/// 클라이언트 구독 메시지
#[derive(Debug, Deserialize)]
pub struct SubscribeMessage {
    pub action: String,       // "subscribe" | "unsubscribe"
    pub channels: Vec<String>,
    pub symbols: Option<Vec<String>>,
}

/// 서버 응답 메시지
#[derive(Debug, Serialize)]
pub struct WsMessage {
    pub channel: String,
    pub data: serde_json::Value,
}

/// 지원 채널 목록
pub const CHANNELS: &[&str] = &[
    "market-data",
    "signals",
    "orders",
    "risk",
    "system",
    "models",
];

/// 채널 유효성 검증
pub fn is_valid_channel(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_channels() {
        assert!(is_valid_channel("market-data"));
        assert!(is_valid_channel("signals"));
        assert!(is_valid_channel("orders"));
        assert!(is_valid_channel("risk"));
        assert!(is_valid_channel("system"));
        assert!(is_valid_channel("models"));
    }

    #[test]
    fn test_invalid_channel() {
        assert!(!is_valid_channel("invalid-channel"));
        assert!(!is_valid_channel(""));
    }
}
