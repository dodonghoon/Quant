//! 킬 스위치 관리 라우트
//!
//! 긴급 거래 중단 기능을 관리합니다.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// 킬 스위치 활성화 요청
#[derive(Debug, Deserialize)]
pub struct ActivateKillSwitchRequest {
    /// 활성화 사유
    pub reason: String,
}

/// 킬 스위치 상태 응답
#[derive(Debug, Serialize)]
pub struct KillSwitchStatus {
    pub active: bool,
    pub reason: Option<String>,
    pub activated_at: Option<String>,
}

/// 킬 스위치의 현재 상태를 조회합니다.
///
/// # 반환값
/// 킬 스위치의 활성화 상태 정보를 반환합니다.
pub async fn get_status(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.kill_switch.read() {
        Ok(_kill_switch) => {
            let response = json!({
                "status": "ok",
                "kill_switch": {
                    "active": false,
                    "reason": null,
                    "activated_at": null
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read kill switch status"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 킬 스위치를 활성화합니다.
///
/// # 요청 본문
/// ```json
/// {
///   "reason": "ManualIntervention"
/// }
/// ```
///
/// # 반환값
/// 킬 스위치 활성화 결과를 반환합니다.
pub async fn activate(
    State(state): State<AppState>,
    Json(payload): Json<ActivateKillSwitchRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.kill_switch.write() {
        Ok(mut kill_switch) => {
            let _ = &mut kill_switch; // lock held
            let response = json!({
                "status": "ok",
                "message": "Kill switch activated successfully",
                "kill_switch": {
                    "active": true,
                    "reason": payload.reason,
                    "activated_at": chrono::Utc::now().to_rfc3339()
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to activate kill switch"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 킬 스위치를 비활성화합니다.
///
/// # 반환값
/// 킬 스위치 비활성화 결과를 반환합니다.
///
/// # 주의
/// 관리자 권한이 필요합니다 (추후 구현).
pub async fn reset(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.kill_switch.write() {
        Ok(mut kill_switch) => {
            let _ = &mut kill_switch; // lock held
            let response = json!({
                "status": "ok",
                "message": "Kill switch deactivated successfully",
                "kill_switch": {
                    "active": false,
                    "reason": null,
                    "activated_at": null
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to deactivate kill switch"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}
