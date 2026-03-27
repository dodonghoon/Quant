//! # Quant Trading Dashboard — API Gateway
//!
//! 퀀트 트레이딩 시스템의 웹 대시보드 백엔드.
//! Axum 기반 REST API + WebSocket 서버.
//!
//! ## 모듈 구성
//! - `config`: 서버 설정 (포트, JWT, CORS)
//! - `auth`: JWT 인증 & 권한 미들웨어
//! - `routes`: REST API 엔드포인트
//! - `ws`: WebSocket 실시간 스트리밍
//! - `bridge`: Rust 트레이딩 엔진 ↔ API 브릿지
//! - `audit`: 감사 로그

pub mod audit;
pub mod auth;
pub mod bridge;
pub mod config;
pub mod routes;
pub mod ws;

use std::sync::Arc;

/// 모든 라우트에서 공유되는 애플리케이션 상태
#[derive(Clone)]
pub struct AppState {
    pub config: config::ServerConfig,
    pub engine_bridge: Arc<bridge::EngineBridge>,
    pub audit_log: Arc<audit::AuditLogger>,
    pub jwt_keys: auth::jwt::JwtKeys,
}
