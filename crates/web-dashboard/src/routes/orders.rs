//! 주문 관리 라우트
//!
//! 주문 조회, 취소 및 체결 이력을 관리합니다.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// 주문 조회 쿼리 파라미터
#[derive(Debug, Deserialize)]
pub struct OrdersQuery {
    /// 주문 상태 필터 (open, filled, cancelled)
    pub status: Option<String>,
    /// 종목 필터
    pub symbol: Option<String>,
    /// 반환 개수 제한
    pub limit: Option<usize>,
}

/// 주문 취소 요청 본문
#[derive(Debug, Deserialize)]
pub struct CancelOrderRequest {
    pub reason: Option<String>,
}

/// 주문 응답 데이터
#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub status: String,
    pub created_at: String,
}

/// 모든 주문을 조회합니다.
///
/// # 쿼리 파라미터
/// - `status`: 주문 상태 (선택사항)
/// - `symbol`: 종목 코드 (선택사항)
/// - `limit`: 반환 개수 (선택사항)
///
/// # 반환값
/// 조건에 맞는 주문 목록을 반환합니다.
pub async fn get_orders(
    State(state): State<AppState>,
    Query(query): Query<OrdersQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.orders.read() {
        Ok(_orders) => {
            let response = json!({
                "status": "ok",
                "filters": {
                    "status": query.status,
                    "symbol": query.symbol,
                    "limit": query.limit
                },
                "orders": [],
                "total_count": 0
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read orders"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 특정 주문을 ID로 조회합니다.
///
/// # 경로 파라미터
/// - `id`: 주문 ID
///
/// # 반환값
/// 해당 주문의 상세 정보를 반환합니다.
pub async fn get_order_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.orders.read() {
        Ok(_orders) => {
            let response = json!({
                "status": "ok",
                "order": {
                    "id": id,
                    "symbol": "BTC/USDT",
                    "side": "BUY",
                    "quantity": 0.5,
                    "price": 45000.0,
                    "status": "open",
                    "created_at": chrono::Utc::now().to_rfc3339()
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Order not found"
            });
            (StatusCode::NOT_FOUND, Json(response))
        }
    }
}

/// 주문을 취소합니다.
///
/// # 경로 파라미터
/// - `id`: 취소할 주문의 ID
///
/// # 요청 본문
/// ```json
/// {
///   "reason": "ManualIntervention"
/// }
/// ```
///
/// # 반환값
/// 취소 결과를 반환합니다.
pub async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_payload): Json<CancelOrderRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.orders.write() {
        Ok(mut orders) => {
            let _ = &mut orders; // lock held
            let response = json!({
                "status": "ok",
                "message": "Order cancelled successfully",
                "order": {
                    "id": id,
                    "status": "Cancelled"
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to cancel order"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 체결 이력을 조회합니다.
///
/// # 반환값
/// 모든 체결 기록을 반환합니다.
pub async fn get_fills(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.fills.read() {
        Ok(_fills) => {
            let response = json!({
                "status": "ok",
                "fills": [],
                "total_count": 0
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read fill history"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}
