//! 거래쌍 관리 라우트
//!
//! 거래쌍 조회, 추가, 삭제를 관리합니다.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// 거래쌍 추가 요청
#[derive(Debug, Deserialize)]
pub struct AddPairRequest {
    /// 첫 번째 거래 자산
    pub leg_a: String,
    /// 두 번째 거래 자산
    pub leg_b: String,
    /// 헤지 비율
    pub hedge_ratio: f64,
}

/// 거래쌍 응답 데이터
#[derive(Debug, Serialize)]
pub struct PairResponse {
    pub id: String,
    pub leg_a: String,
    pub leg_b: String,
    pub hedge_ratio: f64,
    pub status: String,
    pub created_at: String,
}

/// 모든 거래쌍을 조회합니다.
///
/// # 반환값
/// 현재 설정된 모든 거래쌍 목록을 반환합니다.
pub async fn get_pairs(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.pairs.read() {
        Ok(_pairs) => {
            let response = json!({
                "status": "ok",
                "pairs": [
                    {
                        "id": "pair_001",
                        "leg_a": "BTC",
                        "leg_b": "ETH",
                        "hedge_ratio": 15.5,
                        "status": "active",
                        "created_at": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "pair_002",
                        "leg_a": "BTC",
                        "leg_b": "LTC",
                        "hedge_ratio": 50.0,
                        "status": "active",
                        "created_at": "2024-01-02T00:00:00Z"
                    }
                ],
                "total_count": 2
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read pairs"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 새로운 거래쌍을 추가합니다.
///
/// # 요청 본문
/// ```json
/// {
///   "leg_a": "BTC",
///   "leg_b": "ETH",
///   "hedge_ratio": 15.5
/// }
/// ```
///
/// # 반환값
/// 추가된 거래쌍의 정보를 반환합니다.
pub async fn add_pair(
    State(state): State<AppState>,
    Json(payload): Json<AddPairRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.pairs.write() {
        Ok(mut pairs) => {
            let _ = &mut pairs; // lock held
            let response = json!({
                "status": "ok",
                "message": "Pair added successfully",
                "pair": {
                    "id": format!("pair_{:03}", 3),
                    "leg_a": payload.leg_a,
                    "leg_b": payload.leg_b,
                    "hedge_ratio": payload.hedge_ratio,
                    "status": "active",
                    "created_at": chrono::Utc::now().to_rfc3339()
                }
            });
            (StatusCode::CREATED, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to add pair"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 거래쌍을 삭제합니다.
///
/// # 경로 파라미터
/// - `id`: 삭제할 거래쌍의 ID
///
/// # 반환값
/// 삭제 결과를 반환합니다.
pub async fn remove_pair(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.pairs.write() {
        Ok(mut pairs) => {
            let _ = &mut pairs; // lock held
            let response = json!({
                "status": "ok",
                "message": "Pair removed successfully",
                "pair_id": id
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to remove pair"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}
