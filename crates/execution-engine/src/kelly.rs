//! # Kelly Criterion — Optimal Position Sizing
//!
//! 기술문서 4.3: 승률과 배당률에 따른 최적 베팅 비율 산출.
//!
//! ## 연속 모형 (Continuous Kelly)
//! ```text
//! f* = (μ - r) / σ²
//! ```
//! - **f***: 최적 자본 배분 비율
//! - **μ**: 전략의 기대 수익률
//! - **r**: 무위험 이자율
//! - **σ**: 수익률의 표준편차
//!
//! ## 이산 모형 (Discrete Kelly)
//! ```text
//! f* = (p * b - q) / b
//! ```
//! - **p**: 승률, **q = 1-p**: 패율
//! - **b**: 승리 시 배당률 (win/loss ratio)
//!
//! ## 실전 적용: Fractional Kelly
//! Full Kelly는 변동성이 극단적 → 보통 **Half Kelly (f/2)** 또는
//! 더 보수적인 비율 사용. `kelly_fraction` 파라미터로 조절.

/// Kelly 포지션 사이징 설정
#[derive(Debug, Clone)]
pub struct KellyConfig {
    /// Kelly 비율 (0.0 ~ 1.0). 0.5 = Half Kelly (권장).
    /// Full Kelly는 이론적 최적이지만 실전에서는 과도한 변동성 유발.
    pub kelly_fraction: f64,

    /// 최대 단일 포지션 비율 (자본 대비). 안전장치.
    pub max_position_fraction: f64,

    /// 최소 포지션 비율. 이하면 무시 (거래 비용 대비 무의미).
    pub min_position_fraction: f64,

    /// 무위험 이자율 (연율). 연속 모형에서 사용.
    pub risk_free_rate: f64,

    /// 최소 승률. 이하면 포지션 0 (edge 없음).
    pub min_win_rate: f64,
}

impl Default for KellyConfig {
    fn default() -> Self {
        Self {
            kelly_fraction: 0.25,            // Quarter Kelly (보수적)
            max_position_fraction: 0.10,     // 자본의 최대 10%
            min_position_fraction: 0.001,    // 0.1% 이하는 무시
            risk_free_rate: 0.05,            // 5% 연율
            min_win_rate: 0.50,              // 최소 50% 승률
        }
    }
}

/// Kelly Criterion 계산기
pub struct KellySizer {
    config: KellyConfig,
}

impl KellySizer {
    pub fn new(config: KellyConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(KellyConfig::default())
    }

    /// **연속 모형**: 기대수익률과 변동성으로 최적 비율 산출
    ///
    /// ## 인자
    /// - `expected_return`: 전략의 기대 수익률 (per period)
    /// - `volatility`: 수익률의 표준편차 (per period)
    ///
    /// ## 반환
    /// `KellyOutput` — 최적 비율, 클리핑 적용 여부 등
    pub fn continuous_kelly(
        &self,
        expected_return: f64,
        volatility: f64,
    ) -> KellyOutput {
        // Edge가 없으면 → 포지션 0
        let excess_return = expected_return - self.config.risk_free_rate;
        if excess_return <= 0.0 {
            return KellyOutput::zero("no positive edge (μ ≤ r)");
        }

        // σ가 0에 가까우면 → 수치 발산 방지
        if volatility < 1e-10 {
            return KellyOutput::zero("volatility too small");
        }

        // f* = (μ - r) / σ²
        let raw_kelly = excess_return / (volatility * volatility);

        self.apply_constraints(raw_kelly)
    }

    /// **이산 모형**: 승률과 배당률로 최적 비율 산출
    ///
    /// ## 인자
    /// - `win_rate`: 승률 (0.0 ~ 1.0)
    /// - `win_loss_ratio`: 평균 수익 / 평균 손실 (b)
    ///
    /// ## 반환
    /// `KellyOutput` — 최적 비율
    pub fn discrete_kelly(
        &self,
        win_rate: f64,
        win_loss_ratio: f64,
    ) -> KellyOutput {
        // 승률 검증
        if win_rate < self.config.min_win_rate {
            return KellyOutput::zero("win rate below minimum threshold");
        }

        if win_rate <= 0.0 || win_rate >= 1.0 {
            return KellyOutput::zero("invalid win rate (must be 0 < p < 1)");
        }

        if win_loss_ratio <= 0.0 {
            return KellyOutput::zero("invalid win/loss ratio (must be > 0)");
        }

        let p = win_rate;
        let q = 1.0 - p;
        let b = win_loss_ratio;

        // f* = (p * b - q) / b
        let raw_kelly = (p * b - q) / b;

        if raw_kelly <= 0.0 {
            return KellyOutput::zero("no positive edge (p*b ≤ q)");
        }

        self.apply_constraints(raw_kelly)
    }

    /// Signal Generator의 raw_position_frac을 Kelly로 조정
    ///
    /// 간편 인터페이스: 신호 강도와 전략 통계를 결합.
    ///
    /// ## 인자
    /// - `signal_frac`: SignalGenerator의 raw_position_frac (-1.0 ~ 1.0)
    /// - `win_rate`: 해당 전략/심볼의 과거 승률
    /// - `win_loss_ratio`: 평균 수익/손실 비율
    pub fn size_from_signal(
        &self,
        signal_frac: f64,
        win_rate: f64,
        win_loss_ratio: f64,
    ) -> KellyOutput {
        let kelly = self.discrete_kelly(win_rate, win_loss_ratio);

        // 신호 방향 × Kelly 크기
        let direction = signal_frac.signum();
        let signal_magnitude = signal_frac.abs();

        // 최종 크기 = direction × min(signal_magnitude, kelly_fraction)
        let final_frac = direction * signal_magnitude.min(kelly.final_fraction);

        KellyOutput {
            raw_kelly: kelly.raw_kelly,
            fractional_kelly: kelly.fractional_kelly,
            final_fraction: final_frac.abs(),
            direction: if final_frac >= 0.0 { 1.0 } else { -1.0 },
            was_clipped: kelly.was_clipped || signal_magnitude > kelly.final_fraction,
            reject_reason: kelly.reject_reason,
        }
    }

    /// Raw Kelly에 안전장치 적용 (Fractional Kelly + 한도 클리핑)
    fn apply_constraints(&self, raw_kelly: f64) -> KellyOutput {
        // Fractional Kelly 적용
        let fractional = raw_kelly * self.config.kelly_fraction;

        // 한도 클리핑
        let mut clipped = false;
        let final_frac = if fractional > self.config.max_position_fraction {
            clipped = true;
            self.config.max_position_fraction
        } else if fractional < self.config.min_position_fraction {
            // 너무 작으면 거래 비용 대비 무의미 → 0
            return KellyOutput::zero("position too small after fractional kelly");
        } else {
            fractional
        };

        KellyOutput {
            raw_kelly,
            fractional_kelly: fractional,
            final_fraction: final_frac,
            direction: 1.0,
            was_clipped: clipped,
            reject_reason: None,
        }
    }
}

/// Kelly 계산 결과
#[derive(Debug, Clone)]
pub struct KellyOutput {
    /// Full Kelly 비율 (제약 적용 전)
    pub raw_kelly: f64,
    /// Fractional Kelly (kelly_fraction 적용 후)
    pub fractional_kelly: f64,
    /// 최종 포지션 비율 (모든 제약 적용 후, 항상 ≥ 0)
    pub final_fraction: f64,
    /// 방향 (+1.0 long, -1.0 short)
    pub direction: f64,
    /// 한도에 의해 클리핑되었는지
    pub was_clipped: bool,
    /// 포지션 0인 경우 사유
    pub reject_reason: Option<String>,
}

impl KellyOutput {
    fn zero(reason: &str) -> Self {
        Self {
            raw_kelly: 0.0,
            fractional_kelly: 0.0,
            final_fraction: 0.0,
            direction: 0.0,
            was_clipped: false,
            reject_reason: Some(reason.to_string()),
        }
    }

    /// 포지션을 취해야 하는지 (final_fraction > 0)
    pub fn should_trade(&self) -> bool {
        self.final_fraction > 0.0 && self.reject_reason.is_none()
    }

    /// 부호가 있는 최종 비율 (long: +, short: -)
    pub fn signed_fraction(&self) -> f64 {
        self.direction * self.final_fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continuous_kelly() {
        let sizer = KellySizer::new(KellyConfig {
            kelly_fraction: 0.5,
            max_position_fraction: 0.20,
            min_position_fraction: 0.001,
            risk_free_rate: 0.0,
            min_win_rate: 0.50,
        });

        // μ = 0.10, σ = 0.20 → f* = 0.10 / 0.04 = 2.5
        // Half Kelly → 1.25 → clipped to 0.20
        let out = sizer.continuous_kelly(0.10, 0.20);
        assert!((out.raw_kelly - 2.5).abs() < 1e-10);
        assert!(out.was_clipped);
        assert!((out.final_fraction - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_discrete_kelly_coin_flip() {
        let sizer = KellySizer::new(KellyConfig {
            kelly_fraction: 1.0, // Full Kelly for test
            max_position_fraction: 1.0,
            min_position_fraction: 0.001,
            risk_free_rate: 0.0,
            min_win_rate: 0.50,
        });

        // 공정 동전 (p=0.5, b=1.0) → f* = (0.5 * 1 - 0.5) / 1 = 0 → no edge
        let out = sizer.discrete_kelly(0.50, 1.0);
        assert!(!out.should_trade());

        // 유리한 동전 (p=0.6, b=1.0) → f* = (0.6 - 0.4) / 1 = 0.2
        let out = sizer.discrete_kelly(0.60, 1.0);
        assert!(out.should_trade());
        assert!((out.raw_kelly - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_no_edge_returns_zero() {
        let sizer = KellySizer::with_defaults();

        // 기대수익률이 무위험이자율 이하
        let out = sizer.continuous_kelly(0.03, 0.20); // μ=3% < r=5%
        assert!(!out.should_trade());
        assert!(out.reject_reason.is_some());
    }

    #[test]
    fn test_size_from_signal() {
        let sizer = KellySizer::new(KellyConfig {
            kelly_fraction: 0.5,
            max_position_fraction: 0.10,
            min_position_fraction: 0.001,
            risk_free_rate: 0.0,
            min_win_rate: 0.45,
        });

        let out = sizer.size_from_signal(
            -0.8, // Strong Sell signal
            0.55, // 55% win rate
            1.2,  // 1.2:1 win/loss
        );

        assert!(out.should_trade());
        assert!(out.direction < 0.0, "should be short");
        assert!(out.final_fraction <= 0.10, "should respect max limit");
    }
}
