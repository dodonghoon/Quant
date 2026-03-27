//! 인증 라우트
//!
//! JWT 토큰 발급 및 갱신 기능을 제공합니다.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// 로그인 요청
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// 사용자명
    pub username: String,
    /// 비밀번호
    pub password: String,
}

/// 토큰 갱신 요청
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    /// 갱신 토큰
    pub refresh_token: String,
}

/// 로그인 응답
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// 토큰 갱신 응답
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// 사용자를 로그인합니다.
///
/// # 요청 본문
/// ```json
/// {
///   "username": "admin",
///   "password": "admin123"
/// }
/// ```
///
/// # 반환값
/// Access Token과 Refresh Token을 반환합니다.
///
/// # 데모 사용자
/// - 사용자명: `admin`
/// - 비밀번호: `admin123`
pub async fn login(
    State(_state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 데모 사용자: admin / admin123
    if payload.username == "admin" && payload.password == "admin123" {
        let response = json!({
            "status": "ok",
            "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiIsImlhdCI6MTUxNjIzOTAyMn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
            "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiIsInR5cCI6InJlZnJlc2giLCJpYXQiOjE1MTYyMzkwMjJ9.8Z8Z_Z8Z_Z8Z_Z8Z_Z8Z_Z8Z_Z8Z_Z8Z_Z8Z_Z8Z",
            "token_type": "Bearer",
            "expires_in": 3600
        });
        (StatusCode::OK, Json(response))
    } else {
        let response = json!({
            "status": "error",
            "message": "Invalid credentials"
        });
        (StatusCode::UNAUTHORIZED, Json(response))
    }
}

/// 토큰을 갱신합니다.
///
/// # 요청 본문
/// ```json
/// {
///   "refresh_token": "..."
/// }
/// ```
///
/// # 반환값
/// 새로운 Access Token을 반환합니다.
pub async fn refresh_token(
    State(_state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 간단한 검증: refresh_token이 존재하면 새 토큰 발급
    if !payload.refresh_token.is_empty() {
        let response = json!({
            "status": "ok",
            "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiIsImlhdCI6MTUxNjIzOTAyMn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
            "token_type": "Bearer",
            "expires_in": 3600
        });
        (StatusCode::OK, Json(response))
    } else {
        let response = json!({
            "status": "error",
            "message": "Invalid refresh token"
        });
        (StatusCode::UNAUTHORIZED, Json(response))
    }
}
