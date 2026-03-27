//! # Signal Generator
//!
//! Kalman Filter, OU Model 등 알파 소스를 결합하여
//! 최종 트레이딩 시그널을 생성합니다.
//!
//! ## 신호 체계
//! ```text
//!   Strong Buy  ←─ [-∞, -2σ]
//!   Buy         ←─ [-2σ, -1σ]
//!   Neutral     ←─ [-1σ, +1σ]
//!   Sell        ←─ [+1σ, +2σ]
//!   Strong Sell ←─ [+2σ, +∞]
//! ```
//!
//! ## 복합 신호 가중
//! 각 알파 소스에 신뢰도 가중치를 부여하여 합산:
//! `composite_z = Σ(w_i * z_i) / Σ(w_i)`

use crate::kalman::KalmanOutput;
use crate::ou_model::OuSignal;

// ────────────────────────────────────────────
// Signal Types
// ────────────────────────────────────────────

/// 트레이딩 방향 신호
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDirection {
    StrongBuy,   // |z| > entry_threshold, 음수 방향 (스프레드 하락)
    Buy,
    Neutral,
    Sell,
    StrongSell,  // |z| > entry_threshold, 양수 방향 (스프레드 상승)
}

impl SignalDirection {
    /// 방향에 따른 부호 (-1, 0, +1)
    pub fn sign(&self) -> f64 {
        match self {
            Self::StrongBuy | Self::Buy => 1.0,
            Self::Neutral => 0.0,
            Self::Sell | Self::StrongSell => -1.0,
        }
    }

    /// 강도 (0.0 ~ 1.0)
    pub fn strength(&self) -> f64 {
        match self {
            Self::StrongBuy | Self::StrongSell => 1.0,
            Self::Buy | Self::Sell => 0.5,
            Self::Neutral => 0.0,
        }
    }
}

/// 최종 생성된 트레이딩 신호
#[derive(Debug, Clone, Copy)]
pub struct TradingSignal {
    /// 방향
    pub direction: SignalDirection,
    /// 복합 Z-Score (가중 평균)
    pub composite_z: f64,
    /// 신호 신뢰도 (0.0 ~ 1.0)
    pub confidence: f64,
    /// 제안 포지션 크기 비율 (0.0 ~ 1.0, Kelly 적용 전 raw)
    pub raw_position_frac: f64,
    /// 타임스탬프 (나노초)
    pub ts_ns: u64,
    /// 디버그: 개별 알파 소스 기여
    pub alpha_breakdown: AlphaBreakdown,
}

/// 개별 알파 소스 기여 (디버깅 및 분석용)
#[derive(Debug, Clone, Copy)]
pub struct AlphaBreakdown {
    pub ou_z: f64,
    pub ou_weight: f64,
    pub ou_mean_reverting: bool,

    pub kalman_innovation: f64,
    pub kalman_gain: f64,
    pub kalman_weight: f64,
}

// ────────────────────────────────────────────
// Signal Generator
// ────────────────────────────────────────────

/// 신호 생성 설정
#[derive(Debug, Clone)]
pub struct SignalConfig {
    /// 진입 임계값 (Z-Score 절대값)
    pub entry_threshold: f64,
    /// 강한 진입 임계값
    pub strong_entry_threshold: f64,
    /// 청산 임계값 (평균 근접 시)
    pub exit_threshold: f64,
    /// OU 모델 가중치
    pub ou_weight: f64,
    /// Kalman innovation 기반 가중치
    pub kalman_weight: f64,
    /// 최소 신뢰도 (이하면 Neutral 강제)
    pub min_confidence: f64,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            entry_threshold: 1.5,
            strong_entry_threshold: 2.5,
            exit_threshold: 0.5,
            ou_weight: 0.7,     // OU가 주력 알파
            kalman_weight: 0.3, // Kalman은 보조
            min_confidence: 0.3,
        }
    }
}

/// 알파 모델들의 출력을 결합하여 최종 신호를 생성합니다.
///
/// Stateless — 각 호출이 독립적이므로 별도의 상태를 유지하지 않습니다.
/// 상태는 개별 알파 모델(Kalman, OU)이 관리합니다.
pub struct SignalGenerator {
    config: SignalConfig,
}

impl SignalGenerator {
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(SignalConfig::default())
    }

    /// OU 신호와 Kalman 출력을 결합하여 최종 신호 생성
    ///
    /// ## 인자
    /// - `ou_signal`: OU 프로세스의 Z-Score 및 파라미터 (Option: 아직 warm-up이면 None)
    /// - `kalman_out`: Kalman Filter 출력 (Option: 미초기화면 None)
    /// - `ts_ns`: 현재 타임스탬프
    pub fn generate(
        &self,
        ou_signal: Option<&OuSignal>,
        kalman_out: Option<&KalmanOutput>,
        ts_ns: u64,
    ) -> TradingSignal {
        let mut weighted_z = 0.0;
        let mut total_weight = 0.0;

        // ── OU 알파 ──
        let (ou_z, ou_w, ou_mr) = if let Some(ou) = ou_signal {
            let w = if ou.is_mean_reverting {
                self.config.ou_weight
            } else {
                self.config.ou_weight * 0.2 // 평균 회귀가 아니면 가중치 대폭 감소
            };
            weighted_z += w * ou.z_score;
            total_weight += w;
            (ou.z_score, w, ou.is_mean_reverting)
        } else {
            (0.0, 0.0, false)
        };

        // ── Kalman 알파 (Innovation 기반) ──
        // Innovation이 크면 "시장이 예상과 다르게 움직임" → 반대 방향 신호
        let (k_innov, k_gain, k_w) = if let Some(kf) = kalman_out {
            if kf.tick_count > 50 {
                // 충분히 수렴한 후에만 Kalman 기반 신호 사용
                let innovation_z = if kf.estimation_error > 1e-15 {
                    -kf.innovation / kf.estimation_error.sqrt() // 정규화된 innovation
                } else {
                    0.0
                };
                let w = self.config.kalman_weight * (1.0 - kf.gain); // gain이 낮을수록 신뢰
                weighted_z += w * innovation_z;
                total_weight += w;
                (kf.innovation, kf.gain, w)
            } else {
                (kf.innovation, kf.gain, 0.0)
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        // ── 복합 Z-Score ──
        let composite_z = if total_weight > 1e-10 {
            weighted_z / total_weight
        } else {
            0.0
        };

        // ── 방향 결정 ──
        let direction = self.classify_direction(composite_z);

        // ── 신뢰도: 가중치 커버리지 × 알파 일치도 ──
        let max_possible_weight = self.config.ou_weight + self.config.kalman_weight;
        let coverage = if max_possible_weight > 0.0 {
            total_weight / max_possible_weight
        } else {
            0.0
        };

        let confidence = (coverage * composite_z.abs().min(3.0) / 3.0).clamp(0.0, 1.0);

        // 신뢰도 미달이면 Neutral로 강제
        let (direction, confidence) = if confidence < self.config.min_confidence {
            (SignalDirection::Neutral, confidence)
        } else {
            (direction, confidence)
        };

        // ── Raw 포지션 비율 (Kelly 적용 전) ──
        let raw_position_frac = direction.sign()
            * direction.strength()
            * confidence
            * (composite_z.abs().min(3.0) / 3.0); // z-score 비례 스케일링

        TradingSignal {
            direction,
            composite_z,
            confidence,
            raw_position_frac,
            ts_ns,
            alpha_breakdown: AlphaBreakdown {
                ou_z,
                ou_weight: ou_w,
                ou_mean_reverting: ou_mr,
                kalman_innovation: k_innov,
                kalman_gain: k_gain,
                kalman_weight: k_w,
            },
        }
    }

    /// Z-Score → 방향 분류
    fn classify_direction(&self, z: f64) -> SignalDirection {
        let abs_z = z.abs();

        if abs_z < self.config.exit_threshold {
            SignalDirection::Neutral
        } else if z < -self.config.strong_entry_threshold {
            SignalDirection::StrongBuy
        } else if z < -self.config.entry_threshold {
            SignalDirection::Buy
        } else if z > self.config.strong_entry_threshold {
            SignalDirection::StrongSell
        } else if z > self.config.entry_threshold {
            SignalDirection::Sell
        } else {
            SignalDirection::Neutral
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kalman::KalmanOutput;
    use crate::ou_model::{OuParams, OuSignal};

    fn make_ou_signal(z_score: f64, mean_reverting: bool) -> OuSignal {
        OuSignal {
            z_score,
            spread: 0.0,
            is_mean_reverting: mean_reverting,
            params: OuParams {
                kappa: 0.1,
                mu: 0.0,
                sigma: 1.0,
                half_life: 7.0,
                r_squared: 0.5,
            },
        }
    }

    fn make_kalman_output(innovation: f64) -> KalmanOutput {
        KalmanOutput {
            estimated_price: 100.0,
            gain: 0.1,
            innovation,
            estimation_error: 0.01,
            tick_count: 200,
        }
    }

    #[test]
    fn test_strong_buy_signal() {
        let gen = SignalGenerator::with_defaults();
        let ou = make_ou_signal(-3.0, true); // 강한 하락 이탈

        let signal = gen.generate(Some(&ou), None, 0);
        assert_eq!(signal.direction, SignalDirection::StrongBuy);
        assert!(signal.composite_z < -2.0);
    }

    #[test]
    fn test_neutral_without_mean_reversion() {
        let gen = SignalGenerator::with_defaults();
        let ou = make_ou_signal(-3.0, false); // 평균 회귀 아님 → 가중치 대폭 하락

        let signal = gen.generate(Some(&ou), None, 0);
        // 가중치가 줄어들어 신뢰도 미달 → Neutral
        assert!(
            signal.confidence < 0.5,
            "confidence should be low without mean reversion"
        );
    }

    #[test]
    fn test_combined_signal_agreement() {
        let gen = SignalGenerator::with_defaults();
        let ou = make_ou_signal(-2.0, true);
        let kf = make_kalman_output(0.5); // 양의 innovation → 가격 예상보다 높음 → 매수 보조

        let signal = gen.generate(Some(&ou), Some(&kf), 0);
        assert!(
            matches!(
                signal.direction,
                SignalDirection::Buy | SignalDirection::StrongBuy
            ),
            "combined should agree on Buy direction"
        );
    }

    #[test]
    fn test_no_data_neutral() {
        let gen = SignalGenerator::with_defaults();
        let signal = gen.generate(None, None, 0);
        assert_eq!(signal.direction, SignalDirection::Neutral);
    }
}
