//! # GARCH(1,1) 온라인 변동성 추정
//!
//! 기술문서 §4.2:
//! "GARCH 모델: 변동성 예측을 위해 사용.
//!  변동성 클러스터링(Volatility Clustering) 반영."
//!
//! ## 모델
//! ```text
//! σ²_t = ω + α × ε²_{t-1} + β × σ²_{t-1}
//!
//! ω = 장기 분산 기여분 (omega)
//! α = 직전 충격(뉴스) 반응 계수 (ARCH effect)
//! β = 이전 분산 지속 계수 (GARCH effect)
//! ```
//!
//! ## 정상성 조건
//! α + β < 1 (persistence < 1)

use crate::error::{Result, StrategyError};

/// GARCH(1,1) 설정
#[derive(Debug, Clone)]
pub struct GarchConfig {
    /// ω (omega): 장기 분산 기여분
    pub omega: f64,
    /// α (alpha): ARCH 효과 — 직전 충격 반응 (기본: 0.06)
    pub alpha: f64,
    /// β (beta): GARCH 효과 — 분산 지속 (기본: 0.90)
    pub beta: f64,
    /// 초기 분산 (warm-up 전 사용)
    pub initial_variance: f64,
    /// 최소 워밍업 샘플 수
    pub min_samples: usize,
}

impl Default for GarchConfig {
    fn default() -> Self {
        let alpha = 0.06;
        let beta = 0.90;
        // ω = long_run_var × (1 - α - β), 여기서 long_run_var ≈ 0.0004 (2% daily)
        let long_run_variance = 0.0004;
        Self {
            omega: long_run_variance * (1.0 - alpha - beta),
            alpha,
            beta,
            initial_variance: long_run_variance,
            min_samples: 20,
        }
    }
}

impl GarchConfig {
    pub fn validate(&self) -> Result<()> {
        if self.alpha < 0.0 || self.beta < 0.0 || self.omega < 0.0 {
            return Err(StrategyError::ConfigError(
                "GARCH parameters must be non-negative".into(),
            ));
        }
        if self.alpha + self.beta >= 1.0 {
            return Err(StrategyError::ConfigError(format!(
                "GARCH persistence α+β = {:.4} >= 1.0 (non-stationary)",
                self.alpha + self.beta
            )));
        }
        Ok(())
    }
}

/// GARCH(1,1) 온라인 필터 출력
#[derive(Debug, Clone, Copy)]
pub struct GarchOutput {
    /// 현재 조건부 분산 σ²_t
    pub variance: f64,
    /// 현재 조건부 변동성 σ_t
    pub volatility: f64,
    /// 장기 무조건부 변동성
    pub long_run_volatility: f64,
    /// α + β (변동성 지속성)
    pub persistence: f64,
    /// 처리된 샘플 수
    pub sample_count: u64,
}

/// GARCH(1,1) 온라인 변동성 필터
///
/// 매 틱/수익률 관측마다 O(1)로 조건부 분산을 갱신합니다.
/// Rust 실시간 파이프라인에 적합한 스트리밍 구현입니다.
pub struct GarchFilter {
    config: GarchConfig,
    /// 현재 조건부 분산 σ²_t
    current_variance: f64,
    /// 이전 관측의 잔차 제곱 ε²_{t-1}
    prev_shock_sq: f64,
    /// 이전 수익률 (잔차 계산용)
    prev_return: f64,
    /// 누적 샘플 수
    sample_count: u64,
    /// 수익률 이동 평균 (잔차 = return - mean)
    return_mean: f64,
}

impl GarchFilter {
    pub fn new(config: GarchConfig) -> Result<Self> {
        config.validate()?;
        let init_var = config.initial_variance;
        Ok(Self {
            config,
            current_variance: init_var,
            prev_shock_sq: init_var, // 초기값으로 장기 분산 사용
            prev_return: 0.0,
            sample_count: 0,
            return_mean: 0.0,
        })
    }

    /// 새 수익률 관측 업데이트
    ///
    /// σ²_t = ω + α × ε²_{t-1} + β × σ²_{t-1}
    pub fn update(&mut self, ret: f64) -> Result<GarchOutput> {
        self.sample_count += 1;

        // 온라인 수익률 평균 (Welford 스타일)
        let n = self.sample_count as f64;
        self.return_mean += (ret - self.return_mean) / n;

        // 잔차 = 수익률 - 평균
        let epsilon = ret - self.return_mean;
        let shock_sq = epsilon * epsilon;

        // GARCH(1,1) 업데이트
        let new_variance = self.config.omega
            + self.config.alpha * self.prev_shock_sq
            + self.config.beta * self.current_variance;

        // 수치 안정성: 분산은 항상 양수
        self.current_variance = new_variance.max(1e-20);
        self.prev_shock_sq = shock_sq;
        self.prev_return = ret;

        Ok(self.output())
    }

    /// 현재 상태 출력
    pub fn output(&self) -> GarchOutput {
        let persistence = self.config.alpha + self.config.beta;
        let long_run_var = if persistence < 1.0 {
            self.config.omega / (1.0 - persistence)
        } else {
            f64::INFINITY
        };

        GarchOutput {
            variance: self.current_variance,
            volatility: self.current_variance.sqrt(),
            long_run_volatility: long_run_var.sqrt(),
            persistence,
            sample_count: self.sample_count,
        }
    }

    /// 모델 초기화 여부
    pub fn is_warm(&self) -> bool {
        self.sample_count >= self.config.min_samples as u64
    }

    /// h-step ahead 변동성 예측
    ///
    /// σ²_{t+h} = ω/(1-α-β) + (α+β)^h × (σ²_t - ω/(1-α-β))
    pub fn forecast(&self, horizon: usize) -> Vec<f64> {
        let p = self.config.alpha + self.config.beta;
        let long_run_var = if p < 1.0 {
            self.config.omega / (1.0 - p)
        } else {
            self.current_variance
        };

        (1..=horizon)
            .map(|h| {
                let var_h = long_run_var + p.powi(h as i32) * (self.current_variance - long_run_var);
                var_h.max(1e-20).sqrt()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garch_convergence() {
        let mut filter = GarchFilter::new(GarchConfig::default()).unwrap();

        // 100개 정상 수익률 입력
        for i in 0..100 {
            let ret = 0.001 * ((i as f64 * 0.1).sin());
            let out = filter.update(ret).unwrap();
            assert!(out.variance > 0.0);
            assert!(out.volatility > 0.0);
        }

        let out = filter.output();
        assert!(out.persistence < 1.0);
        assert!(out.sample_count == 100);
    }

    #[test]
    fn test_garch_shock_response() {
        let mut filter = GarchFilter::new(GarchConfig::default()).unwrap();

        // 평온한 기간
        for _ in 0..50 {
            filter.update(0.001).unwrap();
        }
        let calm_vol = filter.output().volatility;

        // 큰 충격
        filter.update(0.10).unwrap(); // 10% 수익률
        let shocked_vol = filter.output().volatility;

        // 충격 후 변동성 증가
        assert!(shocked_vol > calm_vol);
    }

    #[test]
    fn test_garch_forecast_converges() {
        let mut filter = GarchFilter::new(GarchConfig::default()).unwrap();
        for _ in 0..50 {
            filter.update(0.002).unwrap();
        }

        let forecast = filter.forecast(100);
        // 장기 예측은 장기 변동성에 수렴
        let long_run = filter.output().long_run_volatility;
        let last_forecast = forecast.last().unwrap();
        assert!((last_forecast - long_run).abs() < 0.001);
    }

    #[test]
    fn test_invalid_params() {
        let result = GarchFilter::new(GarchConfig {
            alpha: 0.5,
            beta: 0.6,
            ..Default::default()
        });
        assert!(result.is_err()); // persistence = 1.1 >= 1.0
    }
}
