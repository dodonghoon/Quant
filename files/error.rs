//! # Strategy Engine Error Types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StrategyError {
    #[error("수치 연산 오류: {context} — {detail}")]
    NumericalError {
        context: &'static str,
        detail: String,
    },

    #[error("모델 미초기화: {model} (최소 {required_samples}개 샘플 필요, 현재 {current})")]
    InsufficientData {
        model: &'static str,
        required_samples: usize,
        current: usize,
    },

    #[error("Kalman Filter 발산: innovation = {innovation:.6}, threshold = {threshold:.6}")]
    KalmanDivergence { innovation: f64, threshold: f64 },

    #[error("설정 오류: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, StrategyError>;
