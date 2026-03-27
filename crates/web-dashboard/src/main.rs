//! # 퀀트 대시보드 서버
//!
//! `cargo run -p web-dashboard --release`

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use web_dashboard::{
    audit::AuditLogger,
    auth,
    bridge::EngineBridge,
    config::ServerConfig,
    routes, ws, AppState,
};

#[tokio::main]
async fn main() {
    // 로깅 초기화
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("web_dashboard=info,tower_http=info")),
        )
        .init();

    // 설정 로드
    let config = ServerConfig::from_env();
    let addr = config.addr;

    tracing::info!("Dashboard server starting on {}", addr);

    // 감사 로그 초기화
    let audit_log = Arc::new(
        AuditLogger::new(&config.audit_db_path)
            .await
            .expect("Failed to initialize audit log"),
    );

    // 엔진 브릿지 초기화 (데모 모드)
    let engine_bridge = Arc::new(EngineBridge::new_demo());

    // JWT 키
    let jwt_keys = auth::jwt::JwtKeys::new(&config.jwt_secret);

    // 앱 상태
    let state = AppState {
        config: config.clone(),
        engine_bridge,
        audit_log,
        jwt_keys,
    };

    // CORS 설정
    let cors = CorsLayer::new()
        .allow_origin(config.cors_origin.parse::<axum::http::HeaderValue>().unwrap())
        .allow_methods(Any)
        .allow_headers(Any);

    // 라우터 구성
    let app = Router::new()
        // 인증
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/refresh", post(routes::auth::refresh_token))
        // 시스템 상태
        .route("/api/v1/status", get(routes::status::get_status))
        .route("/api/v1/health", get(routes::status::health_check))
        // 포지션 & PnL
        .route("/api/v1/positions", get(routes::positions::get_positions))
        .route("/api/v1/pnl/daily", get(routes::positions::get_daily_pnl))
        .route("/api/v1/pnl/history", get(routes::positions::get_pnl_history))
        // 주문
        .route("/api/v1/orders", get(routes::orders::get_orders))
        .route("/api/v1/orders/:id", get(routes::orders::get_order_by_id))
        .route("/api/v1/orders/:id", delete(routes::orders::cancel_order))
        .route("/api/v1/fills", get(routes::orders::get_fills))
        // 시그널
        .route("/api/v1/signals/latest", get(routes::signals::get_latest_signals))
        .route("/api/v1/signals/history", get(routes::signals::get_signal_history))
        // 모델 상태
        .route("/api/v1/models/kalman/:symbol", get(routes::models::get_kalman))
        .route("/api/v1/models/ou/:pair", get(routes::models::get_ou))
        .route("/api/v1/models/garch/:symbol", get(routes::models::get_garch))
        // Kill Switch
        .route("/api/v1/kill-switch", get(routes::kill_switch::get_status))
        .route("/api/v1/kill-switch/activate", post(routes::kill_switch::activate))
        .route("/api/v1/kill-switch/reset", post(routes::kill_switch::reset))
        // 설정
        .route("/api/v1/config/signal", get(routes::config::get_signal_config))
        .route("/api/v1/config/signal", put(routes::config::put_signal_config))
        .route("/api/v1/config/risk", get(routes::config::get_risk_config))
        .route("/api/v1/config/risk", put(routes::config::put_risk_config))
        .route("/api/v1/config/kelly", get(routes::config::get_kelly_config))
        .route("/api/v1/config/kelly", put(routes::config::put_kelly_config))
        .route("/api/v1/config/kalman", get(routes::config::get_kalman_config))
        .route("/api/v1/config/kalman", put(routes::config::put_kalman_config))
        .route("/api/v1/config/garch", get(routes::config::get_garch_config))
        .route("/api/v1/config/garch", put(routes::config::put_garch_config))
        .route("/api/v1/config/almgren-chriss", get(routes::config::get_ac_config))
        .route("/api/v1/config/almgren-chriss", put(routes::config::put_ac_config))
        // 페어 관리
        .route("/api/v1/pairs", get(routes::pairs::get_pairs))
        .route("/api/v1/pairs", post(routes::pairs::add_pair))
        .route("/api/v1/pairs/:id", delete(routes::pairs::remove_pair))
        // 감사 로그
        .route("/api/v1/audit-log", get(routes::audit::get_audit_logs))
        // WebSocket
        .route("/ws/market-data", get(ws::handler::ws_market_data))
        .route("/ws/signals", get(ws::handler::ws_signals))
        .route("/ws/orders", get(ws::handler::ws_orders))
        .route("/ws/risk", get(ws::handler::ws_risk))
        .route("/ws/system", get(ws::handler::ws_system))
        .route("/ws/models", get(ws::handler::ws_models))
        // 미들웨어
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 서버 시작
    tracing::info!("Dashboard ready: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// Ctrl+C 시그널 대기
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");
    tracing::info!("Shutdown signal received");
}
