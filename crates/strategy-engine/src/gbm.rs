//! # Geometric Brownian Motion (GBM) 시뮬레이터
//!
//! 기술문서 §2 다이어그램: "Alpha Models (GBM, OU)"
//!
//! ## 모델 (연속)
//! ```text
//! dS_t = μ S_t dt + σ S_t dW_t
//!
//! μ = 드리프트 (기대 수익률)
//! σ = 변동성
//! W_t = 표준 위너 프로세스
//! ```
//!
//! ## 이산화 (Euler-Maruyama)
//! ```text
//! S_{t+dt} = S_t × exp((μ - σ²/2) × dt + σ × √dt × Z)
//! Z ~ N(0, 1)
//! ```
//!
//! ## 용도
//! - 몬테카를로 시뮬레이션으로 전략 스트레스 테스트
//! - OU 프로세스와 결합하여 Pairs Trading 스프레드 모델링
//! - 옵션 가격 결정 (Black-Scholes 기반)
//! - 백테스팅 합성 데이터 생성

/// GBM 시뮬레이터 설정
#[derive(Debug, Clone)]
pub struct GbmConfig {
    /// 연간 드리프트 (μ)
    pub drift: f64,
    /// 연간 변동성 (σ)
    pub volatility: f64,
    /// 시간 간격 (연 단위, 예: 1/252 = 1 거래일)
    pub dt: f64,
    /// 난수 시드 (재현성)
    pub seed: Option<u64>,
}

impl Default for GbmConfig {
    fn default() -> Self {
        Self {
            drift: 0.05,         // 5% 연간 수익률
            volatility: 0.20,    // 20% 연간 변동성
            dt: 1.0 / 252.0,     // 일봉
            seed: None,
        }
    }
}

/// GBM 시뮬레이션 결과
#[derive(Debug, Clone)]
pub struct GbmPath {
    /// 가격 경로
    pub prices: Vec<f64>,
    /// 로그 수익률
    pub log_returns: Vec<f64>,
    /// 설정
    pub config: GbmConfig,
}

impl GbmPath {
    /// 최종 가격
    pub fn final_price(&self) -> f64 {
        *self.prices.last().unwrap_or(&0.0)
    }

    /// 전체 수익률
    pub fn total_return(&self) -> f64 {
        let first = self.prices.first().unwrap_or(&1.0);
        let last = self.prices.last().unwrap_or(&1.0);
        (last / first) - 1.0
    }

    /// 실현 변동성 (연율화)
    pub fn realized_volatility(&self) -> f64 {
        if self.log_returns.len() < 2 {
            return 0.0;
        }
        let n = self.log_returns.len() as f64;
        let mean: f64 = self.log_returns.iter().sum::<f64>() / n;
        let var: f64 = self.log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
        var.sqrt() / self.config.dt.sqrt() // 연율화
    }
}

/// GBM 시뮬레이터
///
/// 몬테카를로 시뮬레이션, 합성 데이터 생성, 전략 검증에 사용됩니다.
pub struct GbmSimulator {
    config: GbmConfig,
    /// 간단한 LCG 난수 생성기 상태 (외부 크레이트 없이)
    rng_state: u64,
}

impl GbmSimulator {
    pub fn new(config: GbmConfig) -> Self {
        let seed = config.seed.unwrap_or(42);
        Self {
            config,
            rng_state: seed,
        }
    }

    /// 단일 GBM 경로 시뮬레이션
    ///
    /// S_{t+dt} = S_t × exp((μ - σ²/2) × dt + σ × √dt × Z)
    pub fn simulate(&mut self, initial_price: f64, steps: usize) -> GbmPath {
        let mu = self.config.drift;
        let sigma = self.config.volatility;
        let dt = self.config.dt;
        let sqrt_dt = dt.sqrt();

        // 드리프트 보정항: (μ - σ²/2) × dt
        let drift_term = (mu - 0.5 * sigma * sigma) * dt;

        let mut prices = Vec::with_capacity(steps + 1);
        let mut log_returns = Vec::with_capacity(steps);
        prices.push(initial_price);

        for _ in 0..steps {
            let z = self.next_normal();
            let log_ret = drift_term + sigma * sqrt_dt * z;
            let new_price = prices.last().unwrap() * log_ret.exp();

            log_returns.push(log_ret);
            prices.push(new_price);
        }

        GbmPath {
            prices,
            log_returns,
            config: self.config.clone(),
        }
    }

    /// 몬테카를로 시뮬레이션 — 다수 경로 생성
    ///
    /// VaR, CVaR 등 리스크 지표 계산에 사용됩니다.
    pub fn monte_carlo(
        &mut self,
        initial_price: f64,
        steps: usize,
        num_paths: usize,
    ) -> MonteCarloResult {
        let mut final_prices = Vec::with_capacity(num_paths);
        let mut total_returns = Vec::with_capacity(num_paths);

        for _ in 0..num_paths {
            let path = self.simulate(initial_price, steps);
            total_returns.push(path.total_return());
            final_prices.push(path.final_price());
        }

        // 정렬 (VaR 계산용)
        final_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        total_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mean_return = total_returns.iter().sum::<f64>() / num_paths as f64;
        let mean_price = final_prices.iter().sum::<f64>() / num_paths as f64;

        // VaR (5th percentile)
        let var_idx = (num_paths as f64 * 0.05) as usize;
        let var_95 = total_returns[var_idx.min(num_paths - 1)];

        // CVaR (Expected Shortfall below VaR)
        let cvar_95 = if var_idx > 0 {
            total_returns[..var_idx].iter().sum::<f64>() / var_idx as f64
        } else {
            var_95
        };

        MonteCarloResult {
            num_paths,
            steps,
            mean_return,
            mean_final_price: mean_price,
            var_95,
            cvar_95,
            median_return: total_returns[num_paths / 2],
        }
    }

    /// Box-Muller 변환으로 표준 정규 분포 난수 생성
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_uniform();
        let u2 = self.next_uniform();
        // Box-Muller: Z = √(-2 ln U₁) × cos(2π U₂)
        (-2.0 * u1.max(1e-300).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// 간단한 xorshift64 PRNG → [0, 1) 균일 분포
    fn next_uniform(&mut self) -> f64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }
}

/// 몬테카를로 시뮬레이션 결과
#[derive(Debug, Clone)]
pub struct MonteCarloResult {
    pub num_paths: usize,
    pub steps: usize,
    pub mean_return: f64,
    pub mean_final_price: f64,
    /// Value at Risk (5th percentile)
    pub var_95: f64,
    /// Conditional VaR (Expected Shortfall)
    pub cvar_95: f64,
    pub median_return: f64,
}

impl MonteCarloResult {
    pub fn summary(&self) -> String {
        format!(
            "=== Monte Carlo ({} paths × {} steps) ===\n\
             Mean Return:   {:>9.4}%\n\
             Median Return: {:>9.4}%\n\
             VaR (95%):     {:>9.4}%\n\
             CVaR (95%):    {:>9.4}%\n\
             Mean Price:    {:>10.2}",
            self.num_paths,
            self.steps,
            self.mean_return * 100.0,
            self.median_return * 100.0,
            self.var_95 * 100.0,
            self.cvar_95 * 100.0,
            self.mean_final_price,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbm_path_length() {
        let mut sim = GbmSimulator::new(GbmConfig::default());
        let path = sim.simulate(100.0, 252);
        assert_eq!(path.prices.len(), 253); // initial + 252 steps
        assert_eq!(path.log_returns.len(), 252);
    }

    #[test]
    fn test_gbm_prices_positive() {
        let mut sim = GbmSimulator::new(GbmConfig {
            volatility: 0.50, // 높은 변동성
            seed: Some(123),
            ..Default::default()
        });
        let path = sim.simulate(100.0, 1000);
        assert!(path.prices.iter().all(|p| *p > 0.0));
    }

    #[test]
    fn test_monte_carlo_var() {
        let mut sim = GbmSimulator::new(GbmConfig {
            drift: 0.0,
            volatility: 0.20,
            seed: Some(42),
            ..Default::default()
        });
        let result = sim.monte_carlo(100.0, 252, 1000);

        // VaR should be negative (loss)
        assert!(result.var_95 < 0.0);
        // CVaR should be worse than VaR
        assert!(result.cvar_95 <= result.var_95);
    }

    #[test]
    fn test_realized_vol_approx() {
        let mut sim = GbmSimulator::new(GbmConfig {
            drift: 0.0,
            volatility: 0.20,
            seed: Some(99),
            ..Default::default()
        });
        let path = sim.simulate(100.0, 10_000);
        let real_vol = path.realized_volatility();
        // 10,000 step에서 실현 변동성 ≈ 0.20 (오차 ±0.05)
        assert!((real_vol - 0.20).abs() < 0.05);
    }
}
