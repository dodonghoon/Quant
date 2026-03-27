//! # 인증 미들웨어
//!
//! Authorization 헤더에서 Bearer 토큰을 추출하고 검증합니다.

use super::jwt::{self, Claims, JwtKeys};
use super::Role;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// 인증된 사용자 정보 (라우트 핸들러에서 추출)
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    pub role: Role,
}

impl AuthUser {
    /// 최소 권한 레벨 확인
    pub fn require_role(&self, min_role: Role) -> Result<(), Response> {
        if self.role >= min_role {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Insufficient permissions" })),
            )
                .into_response())
        }
    }
}

/// Axum extractor — Authorization: Bearer {token} 에서 사용자 추출
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Authorization 헤더 추출
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "Missing authorization header" })),
                )
                    .into_response()
            })?;

        // "Bearer " 접두사 확인
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid authorization format" })),
            )
                .into_response()
        })?;

        // JWT 키는 Extensions에서 가져옴 (미들웨어에서 주입)
        let keys = parts
            .extensions
            .get::<JwtKeys>()
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "JWT keys not configured" })),
                )
                    .into_response()
            })?;

        // 토큰 검증
        let claims = jwt::verify_token(keys, token).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": format!("Invalid token: {}", e) })),
            )
                .into_response()
        })?;

        // Access Token인지 확인
        if claims.token_type != "access" {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Expected access token" })),
            )
                .into_response());
        }

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
    }
}
