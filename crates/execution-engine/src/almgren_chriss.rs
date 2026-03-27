//! # Almgren-Chriss 최적 집행 모델
//!
//! 기술문서 §4.3:
//! "Almgren-Chriss 모델을 적용하여 시장 충격(Market Impact)과
//!  타이밍 리스크 간의 최적 경로 계산."
//!
//! ## 모델 개요
//! 대량 주문을 실행할 때 시장 충격(Market Impact)을 최소화하면서
//! 타이밍 리스크를 관리하는 최적의 주문 분할 전략을 계산합니다.
//!
//! ## 비용 구성
//! ```text
//! Total Cost = Temporary Impact + Permanent Impact + Timing Risk
//!
//! E[Cost] = ½γΣ(n_j)² + Σ(g(v_j)) + λVar[Cost]
//!
//! 여기서:
//!   γ = permanent impact coefficient
//!   η = temporary impact coefficient
//!   σ = 자산 변동성 (daily)
//!   λ = risk aversion parameter
//!   X = 총 주문 수량
//!   T = 집행 기간 (타임 슬라이스 수)
//! ```
//!
//! ## 최적 궤적 (Optimal Trajectory)
//! ```text
//! x_j = X × sinh(κ(T-t_j)) / sinh(κT)
//!
//! κ = arccosh( (½τ²σ²) / (η × (1 - ½τγ/η)) + 1 )^(1/2)
//!
//! 여기서 τ = T/N (슬라이스 간 시간 간격)
//! ```

/// Almgren-Chriss 모델 설정
#[derive(Debug, Clone)]
pub struct AlmgrenChrissConfig {
    /// 영구적 시장 충격 계수 (γ, gamma)
    /// 거래량 대비 가격의 영구 변동 크기
    pub permanent_impact: f64,

    /// 일시적 시장 충격 계수 (η, eta)
    /// 주문 속도에 비례하는 일시적 가격 영향
    pub temporary_impact: f64,

    /// 일간 변동성 (σ)
    pub daily_volatility: f64,

    /// 리스크 회피 계수 (λ, lambda)
    /// 높을수록 빠르게 실행 (리스크 감소, 충격 증가)
    pub risk_aversion: f64,
}

impl Default for AlmgrenChrissConfig {
    fn default() -> Self {
        Self {
            permanent_impact: 2.5e-7,  // γ
            temporary_impact: 2.5e-6,  // η
            daily_volatility: 0.02,    // 2% 일일 변동성
            risk_aversion: 1e-6,       // λ
        }
    }
}

/// 집행 스케줄 — 각 타임 슬라이스의 주문량
#[derive(Debug, Clone)]
pub struct ExecutionSchedule {
    /// 각 슬라이스에서 집행할 수량 (부호 포함)
    pub slices: Vec<f64>,
    /// 각 슬라이스까지의 잔여 포지션
    pub trajectory: Vec<f64>,
    /// 예상 총 비용 (기대값)
    pub expected_cost: f64,
    /// 비용의 분산
    pub cost_variance: f64,
    /// 타임 슬라이스 간격 (시간 단위)
    pub interval_secs: f64,
}

impl ExecutionSchedule {
    /// 텍스트 요약
    pub fn summary(&self) -> String {
        format!(
            "=== Almgren-Chriss Schedule ===\n\
             Slices:         {}\n\
             Interval:       {:.1}s\n\
             Expected Cost:  {:.6}\n\
             Cost StdDev:    {:.6}\n\
             Total Qty:      {:.4}",
            self.slices.len(),
            self.interval_secs,
            self.expected_cost,
            self.cost_variance.sqrt(),
            self.slices.iter().sum::<f64>(),
        )
    }
}

/// Almgren-Chriss 최적 집행 모델
pub struct AlmgrenChrissModel {
    config: AlmgrenChrissConfig,
}

impl AlmgrenChrissModel {
    pub fn new(config: AlmgrenChrissConfig) -> Self {
        Self { config }
    }

    /// 최적 집행 스케줄 계산
    ///
    /// # 인자
    /// - `total_quantity`: 총 집행 수량 (양수=매수, 음수=매도)
    /// - `num_slices`: 타임 슬라이스 수 (N)
    /// - `total_time_secs`: 총 집행 시간 (초)
    ///
    /// # 반환
    /// 각 슬라이스의 최적 주문 수량
    pub fn optimal_schedule(
        &self,
        total_quantity: f64,
        num_slices: usize,
        total_time_secs: f64,
    ) -> ExecutionSchedule {
        let n = num_slices.max(1);
        let x = total_quantity;
        let tau = total_time_secs / n as f64; // 슬라이스 간격 (초)
        let c = &self.config;

        // 초 → 일 변환 (변동성은 일간 기준)
        let tau_days = tau / 86400.0;

        // κ 계산 — 최적 궤적의 감쇠 파라미터
        let kappa = self.compute_kappa(tau_days);

        // 최적 궤적: x_j = X × sinh(κ(N-j)) / sinh(κN)
        let kappa_n = kappa * n as f64;
        let sinh_kn = kappa_n.sinh();

        let mut trajectory = Vec::with_capacity(n + 1);
        for j in 0..=n {
            let remaining_frac = (kappa * (n - j) as f64).sinh() / sinh_kn;
            trajectory.push(x * remaining_frac);
        }

        // 슬라이스별 주문량 = trajectory[j] - trajectory[j+1]
        let mut slices = Vec::with_capacity(n);
        for j in 0..n {
            slices.push(trajectory[j] - trajectory[j + 1]);
        }

        // 기대 비용 계산
        // E[Cost] = ½γX² + η × Σ(n_j² / τ)
        let permanent_cost = 0.5 * c.permanent_impact * x * x;
        let temporary_cost: f64 = slices
            .iter()
            .map(|nj| c.temporary_impact * nj * nj / tau_days)
            .sum();
        let expected_cost = permanent_cost + temporary_cost;

        // 비용 분산: Var = σ² × τ × Σ(x_j²)
        let cost_variance = c.daily_volatility * c.daily_volatility
            * tau_days
            * trajectory.iter().map(|xj| xj * xj).sum::<f64>();

        ExecutionSchedule {
            slices,
            trajectory,
            expected_cost,
            cost_variance,
            interval_secs: tau,
        }
    }

    /// κ (kappa) 계산 — 궤적 감쇠 속도
    ///
    /// κ = arccosh(α/η + 1)^(1/2)
    /// 여기서 α = ½τ²σ²λ
    fn compute_kappa(&self, tau_days: f64) -> f64 {
        let c = &self.config;
        let sigma_sq = c.daily_volatility * c.daily_volatility;

        // α = risk aversion × variance × time²
        let alpha = c.risk_aversion * sigma_sq * tau_days;

        // η_tilde = η × (1 - 0.5×τ×γ/η)
        let eta_tilde =
            c.temporary_impact * (1.0 - 0.5 * tau_days * c.permanent_impact / c.temporary_impact);

        // 발산 방지: eta_tilde > 0 보장
        let eta_safe = eta_tilde.max(1e-15);

        // kappa = acosh(α / (2 × eta_safe) + 1)
        let arg = alpha / (2.0 * eta_safe) + 1.0;
        arg.acosh().max(1e-10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_schedule() {
        let model = AlmgrenChrissModel::new(AlmgrenChrissConfig::default());
        let schedule = model.optimal_schedule(1000.0, 10, 3600.0);

        // 슬라이스 수량 합계 ≈ 총 수량
        let total: f64 = schedule.slices.iter().sum();
        assert!((total - 1000.0).abs() < 1e-6);
        assert_eq!(schedule.slices.len(), 10);

        // 최적 궤적: 앞쪽이 뒤쪽보다 큼 (urgency → front-loaded)
        assert!(schedule.slices[0] >= schedule.slices[9]);
    }

    #[test]
    fn test_high_risk_aversion_frontloads() {
        let aggressive = AlmgrenChrissModel::new(AlmgrenChrissConfig {
            risk_aversion: 1e-3, // 높은 리스크 회피 → 빠르게 실행
            ..Default::default()
        });
        let patient = AlmgrenChrissModel::new(AlmgrenChrissConfig {
            risk_aversion: 1e-9, // 낮은 리스크 회피 → 균등 분산
            ..Default::default()
        });

        let agg_sched = aggressive.optimal_schedule(1000.0, 10, 3600.0);
        let pat_sched = patient.optimal_schedule(1000.0, 10, 3600.0);

        // 공격적: 첫 슬라이스 비중 > 균등
        let agg_first_frac = agg_sched.slices[0] / 1000.0;
        let pat_first_frac = pat_sched.slices[0] / 1000.0;
        assert!(agg_first_frac > pat_first_frac);
    }

    #[test]
    fn test_sell_order_negative() {
        let model = AlmgrenChrissModel::new(AlmgrenChrissConfig::default());
        let schedule = model.optimal_schedule(-500.0, 5, 1800.0);

        // 매도: 모든 슬라이스가 음수
        for s in &schedule.slices {
            assert!(*s < 0.0);
        }
        let total: f64 = schedule.slices.iter().sum();
        assert!((total + 500.0).abs() < 1e-6);
    }

    #[test]
    fn test_expected_cost_positive() {
        let model = AlmgrenChrissModel::new(AlmgrenChrissConfig::default());
        let schedule = model.optimal_schedule(1000.0, 20, 7200.0);
        assert!(schedule.expected_cost > 0.0);
        assert!(schedule.cost_variance > 0.0);
    }
}
