//! # JWT 인증
//!
//! Access Token (15분) + Refresh Token (7일) 발급/검증.

use super::Role;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT 서명/검증 키 쌍
#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl JwtKeys {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
}

/// JWT Claims (페이로드)
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 사용자 ID
    pub sub: String,
    /// 사용자명
    pub username: String,
    /// 권한 레벨
    pub role: Role,
    /// 만료 시각 (Unix timestamp)
    pub exp: u64,
    /// 발급 시각
    pub iat: u64,
    /// 토큰 유형 ("access" | "refresh")
    pub token_type: String,
}

/// Access Token 생성
pub fn create_access_token(
    keys: &JwtKeys,
    user_id: &str,
    username: &str,
    role: Role,
    ttl_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role,
        exp: now + ttl_secs,
        iat: now,
        token_type: "access".to_string(),
    };
    encode(&Header::default(), &claims, &keys.encoding)
}

/// Refresh Token 생성
pub fn create_refresh_token(
    keys: &JwtKeys,
    user_id: &str,
    username: &str,
    role: Role,
    ttl_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role,
        exp: now + ttl_secs,
        iat: now,
        token_type: "refresh".to_string(),
    };
    encode(&Header::default(), &claims, &keys.encoding)
}

/// 토큰 검증 및 Claims 추출
pub fn verify_token(
    keys: &JwtKeys,
    token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(token, &keys.decoding, &Validation::default())?;
    Ok(token_data.claims)
}
