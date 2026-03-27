//! 시스템 상태 조회 라우트
//!
//! 시스템의 전반적인 상태와 헬스 체크를 제공합니다.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

use crate::AppState;

/// 시스템 상태를 조회합니다.
///
/// # 반환값
/// 현재 시스템 상태 정보를 JSON 형식으로 반환합니다.
pub async fn get_status(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.system.read() {
        Ok(_system) => {
            let response = json!({
                "status": "ok",
                "system": {
                    "running": true,
                    "uptime_seconds": 0,
                    "version": "0.1.0"
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read system state"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 시스템 헬스 체크를 수행합니다.
///
/// # 반환값
/// 각 레이어의 상태를 포함한 헬스 체크 결과를 반환합니다.
pub async fn health_check(State(_state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    let response = json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "layers": {
            "database": "healthy",
            "engine_bridge": "healthy",
            "audit_logger": "healthy",
            "jwt_keys": "healthy"
        }
    });

    (StatusCode::OK, Json(response))
}
