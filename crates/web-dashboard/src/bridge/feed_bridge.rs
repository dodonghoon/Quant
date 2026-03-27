//! # Feed Bridge
//!
//! Data Ingestion Layer ↔ 웹 API 연결.
//! 실시간 시장 데이터를 WebSocket으로 팬아웃합니다.

// 실제 엔진 연동 시 구현할 내용:
// - rtrb::Consumer에서 MarketEvent 수신
// - broadcast::Sender<DashboardEvent>로 팬아웃
// - BBO/Trade를 JSON으로 직렬화하여 WebSocket 클라이언트에 전송

/// Feed → Dashboard 브릿지 (향후 구현)
pub struct FeedBridge;

impl FeedBridge {
    /// 데모 모드에서는 no-op
    pub fn new_demo() -> Self {
        Self
    }
}
