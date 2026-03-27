//! # Data Ingestion Error Types
//!
//! 데이터 수집 계층 전용 에러 타입.
//! WebSocket 연결, 파싱, 스트림 관련 오류를 모두 포괄합니다.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IngestionError {
    /// WebSocket 연결 실패
    #[error("WebSocket 연결 실패 — url: {url}")]
    ConnectionFailed {
        url: String,
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    /// 메시지 수신 중 오류 (네트워크 끊김 등)
    #[error("메시지 수신 오류: {0}")]
    ReceiveError(#[from] tokio_tungstenite::tungstenite::Error),

    /// JSON 파싱 실패 (거래소 포맷 변경, 예상 밖 메시지 등)
    #[error("파싱 오류 ({context}): {source}")]
    ParseError {
        context: String,
        source: serde_json::Error,
    },

    /// 스트림 비정상 종료 (서버 close, 네트워크 단절)
    #[error("스트림 연결 끊김 — 재연결 시도 중")]
    StreamDisconnected,

    /// 설정 오류 (잘못된 URL, 최대 재연결 초과 등)
    #[error("설정 오류: {0}")]
    ConfigError(String),

    /// 저장소 오류 (Redis 쓰기/읽기 실패 등)
    #[error("저장소 오류: {0}")]
    StorageError(String),

    /// 연결 오류 (Redis 연결 실패 등)
    #[error("연결 오류: {0}")]
    ConnectionError(String),
}

pub type Result<T> = std::result::Result<T, IngestionError>;
