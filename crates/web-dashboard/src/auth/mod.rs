//! # 인증 모듈
//!
//! JWT 기반 인증 및 권한 관리.

pub mod jwt;
pub mod middleware;

/// 사용자 권한 레벨
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Role {
    /// 읽기 전용
    Viewer = 0,
    /// Kill Switch 활성화, 주문 취소 가능
    Operator = 1,
    /// 전체 제어 (파라미터 변경, Kill Switch 해제, API 키)
    Admin = 2,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Viewer => write!(f, "viewer"),
            Role::Operator => write!(f, "operator"),
            Role::Admin => write!(f, "admin"),
        }
    }
}
