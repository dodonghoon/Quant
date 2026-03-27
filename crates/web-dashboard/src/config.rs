//! # 서버 설정
//!
//! 환경 변수 또는 기본값으로 웹 대시보드 서버를 설정합니다.

use std::net::SocketAddr;

/// 웹 대시보드 서버 설정
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 바인딩 주소
    pub addr: SocketAddr,
    /// JWT 서명 시크릿
    pub jwt_secret: String,
    /// JWT Access Token 만료 시간 (초)
    pub jwt_access_ttl_secs: u64,
    /// JWT Refresh Token 만료 시간 (초)
    pub jwt_refresh_ttl_secs: u64,
    /// CORS 허용 오리진
    pub cors_origin: String,
    /// SQLite 감사 로그 DB 경로
    pub audit_db_path: String,
    /// 최대 동시 WebSocket 연결 수
    pub max_ws_connections: usize,
    /// 시장 데이터 WebSocket 스로틀 (밀리초)
    pub ws_market_throttle_ms: u64,
}

impl ServerConfig {
    /// 환경 변수에서 설정 로드 (없으면 기본값)
    pub fn from_env() -> Self {
        Self {
            addr: std::env::var("DASHBOARD_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
                .parse()
                .expect("Invalid DASHBOARD_ADDR"),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "quant-dashboard-dev-secret-change-me".to_string()),
            jwt_access_ttl_secs: std::env::var("JWT_ACCESS_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900), // 15분
            jwt_refresh_ttl_secs: std::env::var("JWT_REFRESH_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(604800), // 7일
            cors_origin: std::env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            audit_db_path: std::env::var("AUDIT_DB_PATH")
                .unwrap_or_else(|_| "audit.db".to_string()),
            max_ws_connections: std::env::var("MAX_WS_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            ws_market_throttle_ms: std::env::var("WS_MARKET_THROTTLE_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100), // 10Hz
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
