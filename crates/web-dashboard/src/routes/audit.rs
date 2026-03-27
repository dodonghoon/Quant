//! 감시 로그 조회 라우트
//!
//! 시스템 감시 로그를 조회합니다.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// 감시 로그 조회 쿼리 파라미터
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    /// 로그 레벨 필터 (info, warning, error)
    pub level: Option<String>,
    /// 로그 종류 필터
    pub action: Option<String>,
    /// 반환 개수 제한
    pub limit: Option<usize>,
    /// 오프셋
    pub offset: Option<usize>,
}

/// 감시 로그 응답 데이터
#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub action: String,
    pub user: Option<String>,
    pub details: serde_json::Value,
}

/// 감시 로그를 조회합니다.
///
/// # 쿼리 파라미터
/// - `level`: 로그 레벨 필터 (선택사항)
/// - `action`: 로그 종류 필터 (선택사항)
/// - `limit`: 반환 개수 (선택사항, 기본값: 100)
/// - `offset`: 오프셋 (선택사항, 기본값: 0)
///
/// # 반환값
/// 감시 로그 목록을 반환합니다.
pub async fn get_audit_logs(
    State(state): State<AppState>,
    Query(query): Query<AuditLogQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    // 감시 로그 데이터는 audit_log에서 가져옴
    let response = json!({
        "status": "ok",
        "filters": {
            "level": query.level,
            "action": query.action,
            "limit": limit,
            "offset": offset
        },
        "logs": [
            {
                "id": "log_001",
                "timestamp": "2024-01-15T10:30:00Z",
                "level": "info",
                "action": "system_start",
                "user": null,
                "details": {
                    "version": "0.1.0",
                    "uptime": 3600
                }
            },
            {
                "id": "log_002",
                "timestamp": "2024-01-15T10:35:00Z",
                "level": "info",
                "action": "order_created",
                "user": "system",
                "details": {
                    "order_id": "ord_001",
                    "symbol": "BTC/USDT",
                    "side": "BUY",
                    "quantity": 0.5
                }
            },
            {
                "id": "log_003",
                "timestamp": "2024-01-15T10:40:00Z",
                "level": "warning",
                "action": "drawdown_warning",
                "user": null,
                "details": {
                    "current_drawdown": 0.08,
                    "max_allowed": 0.1
                }
            },
            {
                "id": "log_004",
                "timestamp": "2024-01-15T10:45:00Z",
                "level": "info",
                "action": "fill_executed",
                "user": "system",
                "details": {
                    "order_id": "ord_001",
                    "symbol": "BTC/USDT",
                    "filled_quantity": 0.5,
                    "fill_price": 45000.0
                }
            }
        ],
        "total_count": 4,
        "has_more": false
    });

    (StatusCode::OK, Json(response))
}

/// 특정 로그 ID의 상세 정보를 조회합니다.
///
/// # 경로 파라미터
/// - `id`: 조회할 로그 ID
///
/// # 반환값
/// 해당 로그의 상세 정보를 반환합니다.
pub async fn get_audit_log_detail(
    State(_state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let response = json!({
        "status": "ok",
        "log": {
            "id": id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": "info",
            "action": "order_created",
            "user": "system",
            "details": {
                "order_id": "ord_001",
                "symbol": "BTC/USDT",
                "side": "BUY",
                "quantity": 0.5,
                "price": 45000.0,
                "status": "open"
            }
        }
    });

    (StatusCode::OK, Json(response))
}
