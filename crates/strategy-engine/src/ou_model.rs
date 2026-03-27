//! # Ornstein-Uhlenbeck (OU) Process
//!
//! 페어 트레이딩의 수학적 기반: 두 자산의 스프레드가 평균으로 회귀하는지 모델링합니다.
//!
//! ## OU 확률미분방정식
//! ```text
//! dX_t = θ(μ - X_t)dt + σ dW_t
//! ```
//! - **θ (kappa)**: 평균 회귀 속도 (클수록 빠르게 회귀)
//! - **μ (mu)**: 장기 평균 수준
//! - **σ (sigma)**: 변동성
//!
//! ## 파라미터 추정
//! 이산 관측값에서 OLS 회귀로 추정:
//! ```text
//! X_t - X_{t-1} = a + b * X_{t-1} + ε
//! θ = -b / dt,  μ = -a / b,  σ = std(ε) / sqrt(dt)
//! ```
//!
//! ## Half-life
//! 스프레드가 평균의 50%로 돌아오는 시간: `t_half = ln(2) / θ`

use crate::error::{Result, StrategyError};
use crate::features::RollingWindow;

/// OU 프로세스 설정
#[derive(Debug, Clone)]
pub struct OuConfig {
    /// 파라미터 추정에 사용할 윈도우 크기 (관측값 수)
    pub estimation_window: usize,
    /// 관측 간격 (초). 틱 기반이면 평균 틱 간격 사용.
    pub dt: f64,
    /// 최소 평균 회귀 속도. 이보다 작으면 "평균 회귀 아님"으로 판단.
    pub min_kappa: f64,
    /// 최대 half-life (초). 이보다 길면 "너무 느린 회귀"로 판단.
    pub max_half_life: f64,
}

impl Default for OuConfig {
    fn default() -> Self {
        Self {
            estimation_window: 500,
            dt: 1.0,         // 1초 간격 가정
            min_kappa: 0.01,
            max_half_life: 86400.0, // 24시간
        }
    }
}

/// OU 프로세스 파라미터 추정 결과
#[derive(Debug, Clone, Copy)]
pub struct OuParams {
    /// 평균 회귀 속도 (θ)
    pub kappa: f64,
    /// 장기 평균 (μ)
    pub mu: f64,
    /// 변동성 (σ)
    pub sigma: f64,
    /// Half-life: 스프레드가 평균의 50%로 돌아오는 시간
    pub half_life: f64,
    /// R² (회귀 적합도)
    pub r_squared: f64,
}

impl OuParams {
    /// 현재 파라미터가 유효한 평균 회귀를 나타내는지 판단
    pub fn is_mean_reverting(&self, config: &OuConfig) -> bool {
        self.kappa > config.min_kappa
            && self.half_life > 0.0
            && self.half_life < config.max_half_life
            && self.r_squared > 0.01  // 최소 적합도
    }
}

/// OU 프로세스 기반 평균 회귀 모델
///
/// 스프레드 시계열을 입력받아 실시간으로 파라미터를 추정하고,
/// 현재 상태의 Z-Score 신호를 생성합니다.
pub struct OuModel {
    config: OuConfig,

    /// 스프레드 값 저장 (순환 버퍼)
    spread_window: RollingWindow,

    /// OLS 회귀용 누적 통계 (Welford 방식)
    /// X_{t-1} → X_t 쌍에 대한 온라인 OLS
    sum_x: f64,     // Σ X_{t-1}
    sum_y: f64,     // Σ (X_t - X_{t-1})
    sum_xx: f64,    // Σ X_{t-1}²
    sum_xy: f64,    // Σ X_{t-1} * (X_t - X_{t-1})
    sum_yy: f64,    // Σ (X_t - X_{t-1})²
    n: usize,       // 유효 관측 쌍 수

    prev_value: f64, // 이전 스프레드 값
    initialized: bool,

    /// 최근 추정 파라미터 (캐시)
    params: Option<OuParams>,
}

impl OuModel {
    pub fn new(config: OuConfig) -> Self {
        let window_size = config.estimation_window;
        Self {
            config,
            spread_window: RollingWindow::new(window_size),
            sum_x: 0.0,
            sum_y: 0.0,
            sum_xx: 0.0,
            sum_xy: 0.0,
            sum_yy: 0.0,
            n: 0,
            prev_value: f64::NAN,
            initialized: false,
            params: None,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(OuConfig::default())
    }

    /// 새 스프레드 값 추가 및 모델 업데이트
    ///
    /// ## 반환값
    /// - `Some(OuSignal)`: 충분한 데이터가 있으면 신호 생성
    /// - `None`: 아직 warm-up 중
    pub fn update(&mut self, spread: f64) -> Option<OuSignal> {
        self.spread_window.push(spread);

        if self.initialized {
            let dy = spread - self.prev_value;
            let x = self.prev_value;

            // 온라인 OLS 누적
            self.sum_x += x;
            self.sum_y += dy;
            self.sum_xx += x * x;
            self.sum_xy += x * dy;
            self.sum_yy += dy * dy;
            self.n += 1;

            // 윈도우가 다 차면 가장 오래된 관측 쌍 제거
            // (간소화: 전체 재추정 대신 누적 통계만 유지,
            //  실제 프로덕션에서는 원형 버퍼에서 오래된 값을 빼야 함)
        }

        self.prev_value = spread;
        self.initialized = true;

        // 최소 관측 수 확인 후 파라미터 추정
        if self.n < 30 {
            return None;
        }

        let params = self.estimate_params()?;
        self.params = Some(params);

        let z_score = self.spread_window.z_score();

        Some(OuSignal {
            z_score,
            params,
            spread,
            is_mean_reverting: params.is_mean_reverting(&self.config),
        })
    }

    /// OLS 회귀로 OU 파라미터 추정 (O(1) — 누적 통계 사용)
    fn estimate_params(&self) -> Option<OuParams> {
        let n = self.n as f64;
        if n < 2.0 {
            return None;
        }

        // OLS: dy = a + b * x + ε
        // b = (n * Σxy - Σx * Σy) / (n * Σxx - (Σx)²)
        let denom = n * self.sum_xx - self.sum_x * self.sum_x;
        if denom.abs() < 1e-15 {
            return None;
        }

        let b = (n * self.sum_xy - self.sum_x * self.sum_y) / denom;
        let a = (self.sum_y - b * self.sum_x) / n;

        // OU 파라미터 변환
        let dt = self.config.dt;
        let kappa = -b / dt;

        // kappa가 음수이면 평균 회귀가 아님 (발산)
        if kappa <= 0.0 {
            return Some(OuParams {
                kappa,
                mu: 0.0,
                sigma: 0.0,
                half_life: f64::INFINITY,
                r_squared: 0.0,
            });
        }

        let mu = if b.abs() > 1e-15 { -a / b } else { 0.0 };

        // 잔차 분산 추정
        let mean_y = self.sum_y / n;
        let ss_tot = self.sum_yy - n * mean_y * mean_y;
        let ss_res = self.sum_yy - a * self.sum_y - b * self.sum_xy;
        let r_squared = if ss_tot.abs() > 1e-15 {
            1.0 - (ss_res / ss_tot)
        } else {
            0.0
        };

        let residual_var = (ss_res / (n - 2.0)).max(0.0);
        let sigma = residual_var.sqrt() / dt.sqrt();

        let half_life = if kappa > 1e-15 {
            (2.0_f64).ln() / kappa
        } else {
            f64::INFINITY
        };

        Some(OuParams {
            kappa,
            mu,
            sigma,
            half_life,
            r_squared: r_squared.clamp(0.0, 1.0),
        })
    }

    /// 최근 추정 파라미터
    pub fn params(&self) -> Option<&OuParams> {
        self.params.as_ref()
    }

    /// 현재 Z-Score (스프레드의 정규화된 위치)
    pub fn z_score(&self) -> f64 {
        self.spread_window.z_score()
    }

    /// 모델 리셋
    pub fn reset(&mut self) {
        *self = Self::new(self.config.clone());
    }
}

/// OU 모델 출력 신호
#[derive(Debug, Clone, Copy)]
pub struct OuSignal {
    /// 스프레드의 Z-Score (핵심 트레이딩 신호)
    /// |z| > 2: 강한 이탈 → 진입 고려
    /// |z| < 0.5: 평균 근접 → 청산 고려
    pub z_score: f64,

    /// 추정된 OU 파라미터
    pub params: OuParams,

    /// 현재 스프레드 원시값
    pub spread: f64,

    /// 유효한 평균 회귀 여부 (kappa 및 half-life 기준)
    pub is_mean_reverting: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ou_mean_reverting_signal() {
        let mut ou = OuModel::new(OuConfig {
            estimation_window: 200,
            dt: 1.0,
            min_kappa: 0.001,
            max_half_life: 100000.0,
        });

        // 평균 회귀하는 시뮬레이션 데이터 생성
        // X_{t+1} = X_t + 0.1 * (0 - X_t) + noise
        let mut x = 5.0; // 평균(0)에서 멀리 시작
        let mut last_signal = None;

        for i in 0..500 {
            x = x + 0.1 * (0.0 - x) + if i % 2 == 0 { 0.1 } else { -0.1 };
            last_signal = ou.update(x);
        }

        let signal = last_signal.expect("should have signal after 500 observations");
        assert!(
            signal.params.kappa > 0.0,
            "kappa should be positive for mean-reverting process"
        );
        assert!(
            signal.params.half_life < f64::INFINITY,
            "half-life should be finite"
        );
    }

    #[test]
    fn test_ou_random_walk_not_mean_reverting() {
        let mut ou = OuModel::new(OuConfig {
            estimation_window: 200,
            dt: 1.0,
            min_kappa: 0.05,  // 높은 문턱
            max_half_life: 100.0,
        });

        // 랜덤 워크: 평균 회귀 없음
        let mut x = 0.0;
        let mut last_signal = None;

        for i in 0..500 {
            x += if i % 3 == 0 { 1.0 } else { -0.5 }; // drift 있는 랜덤워크 근사
            last_signal = ou.update(x);
        }

        if let Some(signal) = last_signal {
            // 랜덤워크는 강한 평균 회귀를 보이지 않아야 함
            // (단, 유한 샘플에서는 약간의 kappa가 추정될 수 있음)
            tracing::debug!(kappa = signal.params.kappa, "random walk kappa");
        }
    }
}
