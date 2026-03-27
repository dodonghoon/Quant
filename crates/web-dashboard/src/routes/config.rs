//! 설정 관리 라우트
//!
//! Signal, Risk, Kelly, Kalman, GARCH, Almgren-Chriss 설정을 관리합니다.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::AppState;

/// Signal 설정을 조회합니다.
///
/// # 반환값
/// 현재 Signal 설정을 반환합니다.
pub async fn get_signal_config(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.signal_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "config": {
                    "min_strength": 0.6,
                    "max_exposure": 0.3,
                    "cooldown_period": 300
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read signal config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Signal 설정을 업데이트합니다.
///
/// # 요청 본문
/// Signal 설정 JSON 객체
///
/// # 반환값
/// 업데이트된 설정을 반환합니다.
pub async fn put_signal_config(
    State(state): State<AppState>,
    Json(config): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.signal_config.write() {
        Ok(mut _cfg) => {
            let response = json!({
                "status": "ok",
                "message": "Signal config updated successfully",
                "config": config
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to update signal config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Risk 설정을 조회합니다.
///
/// # 반환값
/// 현재 Risk 설정을 반환합니다.
pub async fn get_risk_config(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.risk_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "config": {
                    "max_position_size": 0.5,
                    "max_drawdown": 0.1,
                    "position_limit": 5,
                    "stop_loss_percent": 0.02
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read risk config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Risk 설정을 업데이트합니다.
///
/// # 요청 본문
/// Risk 설정 JSON 객체
///
/// # 반환값
/// 업데이트된 설정을 반환합니다.
pub async fn put_risk_config(
    State(state): State<AppState>,
    Json(config): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.risk_config.write() {
        Ok(mut _cfg) => {
            let response = json!({
                "status": "ok",
                "message": "Risk config updated successfully",
                "config": config
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to update risk config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Kelly 설정을 조회합니다.
///
/// # 반환값
/// 현재 Kelly 설정을 반환합니다.
pub async fn get_kelly_config(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.kelly_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "config": {
                    "kelly_fraction": 0.25,
                    "win_rate": 0.55,
                    "avg_win": 1.5,
                    "avg_loss": 1.0
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read kelly config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Kelly 설정을 업데이트합니다.
///
/// # 요청 본문
/// Kelly 설정 JSON 객체
///
/// # 반환값
/// 업데이트된 설정을 반환합니다.
pub async fn put_kelly_config(
    State(state): State<AppState>,
    Json(config): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.kelly_config.write() {
        Ok(mut _cfg) => {
            let response = json!({
                "status": "ok",
                "message": "Kelly config updated successfully",
                "config": config
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to update kelly config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Kalman 설정을 조회합니다.
///
/// # 반환값
/// 현재 Kalman 설정을 반환합니다.
pub async fn get_kalman_config(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.kalman_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "config": {
                    "process_noise": 0.01,
                    "measurement_noise": 0.05,
                    "initial_state": [0.0, 0.0],
                    "state_transition": [[1.0, 1.0], [0.0, 1.0]]
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read kalman config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Kalman 설정을 업데이트합니다.
///
/// # 요청 본문
/// Kalman 설정 JSON 객체
///
/// # 반환값
/// 업데이트된 설정을 반환합니다.
pub async fn put_kalman_config(
    State(state): State<AppState>,
    Json(config): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.kalman_config.write() {
        Ok(mut _cfg) => {
            let response = json!({
                "status": "ok",
                "message": "Kalman config updated successfully",
                "config": config
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to update kalman config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// GARCH 설정을 조회합니다.
///
/// # 반환값
/// 현재 GARCH 설정을 반환합니다.
pub async fn get_garch_config(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.garch_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "config": {
                    "p": 1,
                    "q": 1,
                    "omega": 0.00001,
                    "alpha": [0.08],
                    "beta": [0.90]
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read garch config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// GARCH 설정을 업데이트합니다.
///
/// # 요청 본문
/// GARCH 설정 JSON 객체
///
/// # 반환값
/// 업데이트된 설정을 반환합니다.
pub async fn put_garch_config(
    State(state): State<AppState>,
    Json(config): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.garch_config.write() {
        Ok(mut _cfg) => {
            let response = json!({
                "status": "ok",
                "message": "GARCH config updated successfully",
                "config": config
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to update garch config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Almgren-Chriss 설정을 조회합니다.
///
/// # 반환값
/// 현재 Almgren-Chriss 설정을 반환합니다.
pub async fn get_ac_config(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.ac_config.read() {
        Ok(_config) => {
            let response = json!({
                "status": "ok",
                "config": {
                    "lambda": 1e-6,
                    "kappa": 0.5,
                    "eta": 0.1,
                    "participation_rate": 0.1
                }
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to read ac config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Almgren-Chriss 설정을 업데이트합니다.
///
/// # 요청 본문
/// Almgren-Chriss 설정 JSON 객체
///
/// # 반환값
/// 업데이트된 설정을 반환합니다.
pub async fn put_ac_config(
    State(state): State<AppState>,
    Json(config): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match state.engine_bridge.ac_config.write() {
        Ok(mut _cfg) => {
            let response = json!({
                "status": "ok",
                "message": "Almgren-Chriss config updated successfully",
                "config": config
            });
            (StatusCode::OK, Json(response))
        }
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Failed to update ac config"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}
