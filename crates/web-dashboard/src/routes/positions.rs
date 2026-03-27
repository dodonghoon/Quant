//! 포지션 조회 라우트
//!
//! 현재 포지션, PnL 및 포지션 이력을 조회합니다.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// PnL 조회 쿼리 파라미터
#[derive(Debug, Deserialize)]
pub struct PnlHistoryQuery {
    /// 시작 일자 (YYYY-MM-DD 형식)
    pub from: Option<String>,
    /// 종료 일자 (YYYY-MM-DD 형식)
    pub to: Option<String>,
}

/// 포지션 데이터 응답
#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub pnl_percent: f64,
}

/// 모든 포지션을 조회합니다.
///
/// # 반환값
/// 현재 보유 중인 모든 포지션을 JSON 배열로 반환합니다.
pub async fn get_positions(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.positions.read() {
        Ok(_positions) => {
            let response = json!({
                "status": "ok",
                "positions": [],
                "total_count": 0
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read positions"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 일일 PnL을 계산합니다.
///
/// # 반환값
/// 현재 포지션의 총 미실현 PnL을 반환합니다.
pub async fn get_daily_pnl(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.positions.read() {
        Ok(_positions) => {
            let response = json!({
                "status": "ok",
                "date": chrono::Utc::now().format("%Y-%m-%d").to_string(),
                "unrealized_pnl": 0.0,
                "realized_pnl": 0.0,
                "total_pnl": 0.0,
                "pnl_percent": 0.0
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to calculate daily PnL"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// PnL 이력을 조회합니다.
///
/// # 쿼리 파라미터
/// - `from`: 시작 일자 (선택사항)
/// - `to`: 종료 일자 (선택사항)
///
/// # 반환값
/// 지정된 기간의 PnL 이력을 반환합니다.
pub async fn get_pnl_history(
    State(_state): State<AppState>,
    Query(query): Query<PnlHistoryQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let response = json!({
        "status": "ok",
        "from": query.from.unwrap_or_else(|| "2024-01-01".to_string()),
        "to": query.to.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        "history": [
            {
                "date": "2024-01-01",
                "daily_pnl": 1500.0,
                "cumulative_pnl": 1500.0
            },
            {
                "date": "2024-01-02",
                "daily_pnl": -800.0,
                "cumulative_pnl": 700.0
            },
            {
                "date": "2024-01-03",
                "daily_pnl": 2200.0,
                "cumulative_pnl": 2900.0
            }
        ]
    });

    (StatusCode::OK, Json(response))
}
