//! # Kalman Filter (1D Linear)
//!
//! 시장 가격의 노이즈를 제거하고 "True Price"를 실시간 추정합니다.
//!
//! ## 모델
//! ```text
//! State equation:     x_t = A * x_{t-1} + process_noise
//! Observation eq:     z_t = H * x_t     + measurement_noise
//! ```
//!
//! 1D 단순화 (A=1, H=1):
//! - **State**: 진정한 가격 (숨겨진 상태)
//! - **Observation**: 거래소에서 관측된 가격 (노이즈 포함)
//!
//! ## 성능
//! - 매 틱 O(1) 연산 (상수 시간)
//! - 힙 할당 없음 (모든 상태가 스칼라)
//! - Strategy Engine에서 매 `MarketEvent`마다 호출 가능

use crate::error::{Result, StrategyError};

/// 1D Kalman Filter 설정
#[derive(Debug, Clone)]
pub struct KalmanConfig {
    /// 프로세스 노이즈 분산 (Q): 가격이 얼마나 빠르게 변하는지
    /// 클수록 새 관측값에 빠르게 반응. 작을수록 부드러운 추정.
    pub process_noise: f64,

    /// 측정 노이즈 분산 (R): 관측값의 노이즈 수준
    /// 클수록 관측값을 덜 신뢰. 작을수록 관측값에 가깝게 추정.
    pub measurement_noise: f64,

    /// Innovation 발산 임계값: |innovation| > threshold면 경고
    pub divergence_threshold: f64,
}

impl Default for KalmanConfig {
    fn default() -> Self {
        Self {
            process_noise: 1e-5,        // Q: 가격 변동 분산
            measurement_noise: 1e-3,     // R: 시장 노이즈
            divergence_threshold: 50.0,  // 비정상적 가격 점프 감지
        }
    }
}

/// 1D Kalman Filter 상태
///
/// 전체 크기 ~64바이트, 스택에 유지됩니다.
pub struct KalmanFilter {
    config: KalmanConfig,

    // ── 필터 상태 ──
    /// 추정 상태 (True Price 추정값)
    x: f64,
    /// 추정 오차 공분산
    p: f64,

    // ── 진단 지표 ──
    /// 최근 Kalman Gain (0~1: 0이면 관측 무시, 1이면 관측에 전적 의존)
    gain: f64,
    /// 최근 Innovation (관측값 - 예측값)
    innovation: f64,
    /// 업데이트 횟수
    tick_count: u64,
    /// 초기화 여부
    initialized: bool,
}

impl KalmanFilter {
    pub fn new(config: KalmanConfig) -> Self {
        Self {
            config,
            x: 0.0,
            p: 1.0, // 초기 불확실성을 크게 설정
            gain: 0.0,
            innovation: 0.0,
            tick_count: 0,
            initialized: false,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(KalmanConfig::default())
    }

    /// 새 관측값으로 필터 상태를 업데이트하고, 추정 상태를 반환합니다.
    ///
    /// ## 반환값
    /// `KalmanOutput` — 추정 가격, gain, innovation 등
    ///
    /// ## 복잡도
    /// O(1) — 상수 시간, 힙 할당 없음
    pub fn update(&mut self, observation: f64) -> Result<KalmanOutput> {
        if !self.initialized {
            // 첫 번째 관측값으로 초기화
            self.x = observation;
            self.p = self.config.measurement_noise;
            self.initialized = true;
            self.tick_count = 1;

            return Ok(KalmanOutput {
                estimated_price: observation,
                gain: 1.0,
                innovation: 0.0,
                estimation_error: self.p,
                tick_count: 1,
            });
        }

        self.tick_count += 1;

        // ── Predict Step ──
        // x_pred = A * x (A=1 → 변화 없음)
        let x_pred = self.x;
        // P_pred = A * P * A' + Q
        let p_pred = self.p + self.config.process_noise;

        // ── Update Step ──
        // Innovation: y = z - H * x_pred (H=1)
        self.innovation = observation - x_pred;

        // 발산 감지
        if self.innovation.abs() > self.config.divergence_threshold {
            tracing::warn!(
                innovation = self.innovation,
                threshold = self.config.divergence_threshold,
                tick = self.tick_count,
                "Kalman filter innovation exceeds threshold"
            );
            // 발산 시 필터 리셋 대신 경고만 (전략에서 판단)
        }

        // Innovation covariance: S = H * P_pred * H' + R
        let s = p_pred + self.config.measurement_noise;

        // Kalman Gain: K = P_pred * H' / S
        self.gain = if s.abs() > 1e-15 {
            p_pred / s
        } else {
            0.5 // S가 0에 가까우면 안전한 기본값
        };

        // State update: x = x_pred + K * innovation
        self.x = x_pred + self.gain * self.innovation;

        // Covariance update: P = (1 - K*H) * P_pred
        self.p = (1.0 - self.gain) * p_pred;

        // P가 수치 오차로 음수가 되는 것 방지
        if self.p < 0.0 {
            self.p = 1e-10;
        }

        Ok(KalmanOutput {
            estimated_price: self.x,
            gain: self.gain,
            innovation: self.innovation,
            estimation_error: self.p,
            tick_count: self.tick_count,
        })
    }

    /// 현재 추정 가격
    #[inline]
    pub fn state(&self) -> f64 {
        self.x
    }

    /// 현재 Kalman Gain
    #[inline]
    pub fn gain(&self) -> f64 {
        self.gain
    }

    /// 필터가 초기화되었는지
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 프로세스 노이즈를 동적으로 조정 (변동성 적응)
    ///
    /// GARCH 등으로 추정한 변동성을 반영할 때 사용:
    /// ```ignore
    /// let vol = garch.predict_variance();
    /// kalman.adapt_process_noise(vol * 0.01);
    /// ```
    pub fn adapt_process_noise(&mut self, new_q: f64) {
        self.config.process_noise = new_q.max(1e-10);
    }

    /// 필터를 현재 관측값으로 리셋 (급격한 구조 변화 시)
    pub fn reset(&mut self, observation: f64) {
        self.x = observation;
        self.p = self.config.measurement_noise;
        self.gain = 1.0;
        self.innovation = 0.0;
        // tick_count는 유지 (누적 통계용)
    }
}

/// Kalman Filter 업데이트 결과
#[derive(Debug, Clone, Copy)]
pub struct KalmanOutput {
    /// 노이즈 제거된 추정 가격
    pub estimated_price: f64,
    /// Kalman Gain (0~1): 새 관측에 대한 신뢰도
    pub gain: f64,
    /// Innovation: 예측과 관측의 차이 (이상 탐지에 활용)
    pub innovation: f64,
    /// 추정 오차 공분산
    pub estimation_error: f64,
    /// 누적 업데이트 횟수
    pub tick_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_converges_to_constant() {
        let mut kf = KalmanFilter::with_defaults();
        let true_price = 100.0;

        // 노이즈가 있는 관측값으로 100회 업데이트
        for i in 0..100 {
            let noise = if i % 2 == 0 { 0.5 } else { -0.5 };
            let obs = true_price + noise;
            let out = kf.update(obs).unwrap();

            if i > 50 {
                // 충분히 수렴했으면 오차 < 1.0
                assert!(
                    (out.estimated_price - true_price).abs() < 1.0,
                    "tick {i}: estimated {}, expected ~{}",
                    out.estimated_price,
                    true_price
                );
            }
        }
    }

    #[test]
    fn test_kalman_gain_decreases() {
        let mut kf = KalmanFilter::with_defaults();
        let mut prev_gain = f64::MAX;

        for i in 0..20 {
            let out = kf.update(100.0 + (i as f64) * 0.01).unwrap();
            // Gain은 점점 감소해야 함 (확신 증가)
            if i > 0 {
                assert!(
                    out.gain <= prev_gain + 1e-10,
                    "gain should decrease: {} > {}",
                    out.gain,
                    prev_gain
                );
            }
            prev_gain = out.gain;
        }
    }

    #[test]
    fn test_kalman_reset() {
        let mut kf = KalmanFilter::with_defaults();
        for _ in 0..50 {
            kf.update(100.0).unwrap();
        }
        kf.reset(200.0);
        assert!((kf.state() - 200.0).abs() < 1e-10);
    }
}
