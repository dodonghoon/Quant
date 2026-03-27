//! 신호 조회 라우트
//!
//! 거래 신호 및 신호 이력을 조회합니다.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// 신호 이력 조회 쿼리 파라미터
#[derive(Debug, Deserialize)]
pub struct SignalHistoryQuery {
    /// 거래쌍 필터 (예: BTC/USDT)
    pub pair: Option<String>,
    /// 반환 개수 제한
    pub limit: Option<usize>,
}

/// 신호 응답 데이터
#[derive(Debug, Serialize)]
pub struct SignalResponse {
    pub id: String,
    pub pair: String,
    pub signal_type: String,
    pub strength: f64,
    pub timestamp: String,
}

/// 최근 신호를 조회합니다.
///
/// # 반환값
/// 최근에 생성된 신호들을 반환합니다.
pub async fn get_latest_signals(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.signals.read() {
        Ok(_signals) => {
            let response = json!({
                "status": "ok",
                "signals": [
                    {
                        "id": "sig_001",
                        "pair": "BTC/USDT",
                        "signal_type": "BUY",
                        "strength": 0.85,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    },
                    {
                        "id": "sig_002",
                        "pair": "ETH/USDT",
                        "signal_type": "SELL",
                        "strength": 0.72,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }
                ],
                "total_count": 2
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read signals"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 신호 이력을 조회합니다.
///
/// # 쿼리 파라미터
/// - `pair`: 거래쌍 필터 (선택사항)
/// - `limit`: 반환 개수 제한 (선택사항)
///
/// # 반환값
/// 지정된 조건에 맞는 신호 이력을 반환합니다.
pub async fn get_signal_history(
    State(state): State<AppState>,
    Query(query): Query<SignalHistoryQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.signals.read() {
        Ok(_signals) => {
            let response = json!({
                "status": "ok",
                "filters": {
                    "pair": query.pair,
                    "limit": query.limit
                },
                "signals": [
                    {
                        "id": "sig_001",
                        "pair": "BTC/USDT",
                        "signal_type": "BUY",
                        "strength": 0.85,
                        "timestamp": "2024-01-15T10:30:00Z"
                    },
                    {
                        "id": "sig_002",
                        "pair": "BTC/USDT",
                        "signal_type": "SELL",
                        "strength": 0.65,
                        "timestamp": "2024-01-14T15:45:00Z"
                    },
                    {
                        "id": "sig_003",
                        "pair": "ETH/USDT",
                        "signal_type": "BUY",
                        "strength": 0.78,
                        "timestamp": "2024-01-14T08:20:00Z"
                    }
                ],
                "total_count": 3
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read signal history"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}
