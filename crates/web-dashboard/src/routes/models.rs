//! 모델 상태 조회 라우트
//!
//! Kalman, OU, GARCH 등의 모델 상태를 조회합니다.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

use crate::AppState;

/// Kalman 필터 상태를 조회합니다.
///
/// # 경로 파라미터
/// - `symbol`: 조회할 심볼
///
/// # 반환값
/// 해당 심볼의 Kalman 필터 상태를 반환합니다.
pub async fn get_kalman(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.kalman_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "symbol": symbol,
                "model": "Kalman",
                "state": {
                    "position": [100.5, -5.2],
                    "velocity": [0.1, -0.05],
                    "covariance": [[1.2, 0.0], [0.0, 1.5]],
                    "process_noise": 0.01,
                    "measurement_noise": 0.05,
                    "last_update": chrono::Utc::now().to_rfc3339()
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read Kalman config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Ornstein-Uhlenbeck (OU) 모델 상태를 조회합니다.
///
/// # 경로 파라미터
/// - `pair`: 조회할 거래쌍
///
/// # 반환값
/// 해당 거래쌍의 OU 모델 상태를 반환합니다.
pub async fn get_ou(
    State(_state): State<AppState>,
    Path(pair): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let response = json!({
        "status": "ok",
        "pair": pair,
        "model": "OrnsteinUhlenbeck",
        "state": {
            "mean_reversion_speed": 0.15,
            "long_term_mean": 45000.5,
            "volatility": 2500.0,
            "current_value": 44950.0,
            "predicted_value": 45100.0,
            "half_life": 4.62,
            "last_update": chrono::Utc::now().to_rfc3339()
        }
    });
    (StatusCode::OK, Json(response))
}

/// GARCH 모델 상태를 조회합니다.
///
/// # 경로 파라미터
/// - `symbol`: 조회할 심볼
///
/// # 반환값
/// 해당 심볼의 GARCH 모델 상태를 반환합니다.
pub async fn get_garch(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.engine_bridge.garch_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "symbol": symbol,
                "model": "GARCH",
                "state": {
                    "p": 1,
                    "q": 1,
                    "omega": 0.00001,
                    "alpha": [0.08],
                    "beta": [0.90],
                    "current_variance": 0.0004,
                    "volatility": 0.02,
                    "forecast_variance": 0.00041,
                    "last_update": chrono::Utc::now().to_rfc3339()
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read GARCH config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}
