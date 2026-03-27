//! # PyO3 Bridge — Rust ↔ Python 인터페이스
//!
//! 기술문서 §3.3:
//! "PyO3: Rust 함수를 Python 모듈로 컴파일하거나,
//!  Rust 내에서 Python 인터프리터를 호출할 때 사용."
//!
//! Maturin으로 빌드:
//! ```bash
//! cd crates/strategy-engine
//! maturin develop --features python
//! ```
//!
//! Python에서 사용:
//! ```python
//! from quant_engine import PyKalmanFilter, PyOuModel
//! kf = PyKalmanFilter()
//! kf.update(50000.0)
//! print(kf.get_state())
//! ```

#![cfg(feature = "python")]

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

// ============================================================================
// PyKalmanFilter — Kalman Filter를 Python으로 노출
// ============================================================================

/// Python 클래스: Kalman Filter
///
/// 시장 가격에서 노이즈를 제거하고 예상 상태를 추정합니다.
///
/// 메서드:
/// - `new()`: 기본 설정으로 초기화
/// - `update(measurement: f64)`: 새로운 측정값을 필터에 입력
/// - `get_state() -> dict`: 현재 상태를 딕셔너리로 반환
#[pyclass]
pub struct PyKalmanFilter {
    inner: crate::kalman::KalmanFilter,
}

#[pymethods]
impl PyKalmanFilter {
    /// Kalman Filter 생성
    ///
    /// # 예제
    /// ```python
    /// kf = PyKalmanFilter()
    /// ```
    #[new]
    pub fn new() -> Self {
        PyKalmanFilter {
            inner: crate::kalman::KalmanFilter::new(),
        }
    }

    /// 측정값으로 필터 업데이트
    ///
    /// # 인자
    /// - `measurement: f64` - 새로운 시장 가격 관측치
    ///
    /// # 예제
    /// ```python
    /// kf.update(50000.0)  # BTC 가격 50000달러
    /// ```
    pub fn update(&mut self, measurement: f64) {
        self.inner.update(measurement);
    }

    /// 현재 추정 상태 반환
    ///
    /// # 반환
    /// - dict: `{"position": f64, "velocity": f64}`
    ///
    /// # 예제
    /// ```python
    /// state = kf.get_state()
    /// print(state["position"])  # 현재 위치 추정값
    /// ```
    pub fn get_state(&self) -> PyResult<(f64, f64)> {
        Ok((self.inner.position, self.inner.velocity))
    }
}

// ============================================================================
// PyOuModel — Ornstein-Uhlenbeck 모델을 Python으로 노출
// ============================================================================

/// Python 클래스: Ornstein-Uhlenbeck 평균 복귀 모델
///
/// 금리, 통화 페어, 상품 가격 등의 평균 복귀 행동을 모델링합니다.
///
/// 메서드:
/// - `new()`: 기본 파라미터로 초기화
/// - `update(market_price: f64)`: 신규 시장 가격으로 업데이트
/// - `get_signal() -> dict`: 현재 OU 신호 반환
#[pyclass]
pub struct PyOuModel {
    inner: crate::ou_model::OuModel,
}

#[pymethods]
impl PyOuModel {
    /// Ornstein-Uhlenbeck 모델 생성
    ///
    /// # 예제
    /// ```python
    /// ou = PyOuModel()
    /// ```
    #[new]
    pub fn new() -> Self {
        PyOuModel {
            inner: crate::ou_model::OuModel::new(),
        }
    }

    /// 시장 가격으로 모델 업데이트
    ///
    /// # 인자
    /// - `market_price: f64` - 현재 시장 가격
    ///
    /// # 예제
    /// ```python
    /// ou.update(1.0950)  # EUR/USD 1.0950
    /// ```
    pub fn update(&mut self, market_price: f64) {
        self.inner.update(market_price);
    }

    /// OU 신호 반환
    ///
    /// # 반환
    /// - dict: `{"ou_value": f64, "zscore": f64}`
    ///
    /// # 예제
    /// ```python
    /// signal = ou.get_signal()
    /// print(signal["zscore"])  # 표준편차 단위의 이탈도
    /// ```
    pub fn get_signal(&self) -> PyResult<(f64, f64)> {
        Ok((self.inner.ou_value, self.inner.zscore))
    }
}

// ============================================================================
// PySignalGenerator — 신호 생성기를 Python으로 노출
// ============================================================================

/// Python 클래스: 통합 신호 생성기
///
/// 여러 기술적 지표를 조합하여 거래 신호를 생성합니다.
///
/// 메서드:
/// - `new()`: 초기화
/// - `generate(price: f64, volume: f64) -> dict`: 신호 생성
#[pyclass]
pub struct PySignalGenerator {
    inner: crate::signal::SignalGenerator,
}

#[pymethods]
impl PySignalGenerator {
    /// 신호 생성기 생성
    ///
    /// # 예제
    /// ```python
    /// sg = PySignalGenerator()
    /// ```
    #[new]
    pub fn new() -> Self {
        PySignalGenerator {
            inner: crate::signal::SignalGenerator::new(),
        }
    }

    /// 신호 생성
    ///
    /// # 인자
    /// - `price: f64` - 현재 가격
    /// - `volume: f64` - 거래량
    ///
    /// # 반환
    /// - dict: 신호 강도 및 방향 정보
    ///
    /// # 예제
    /// ```python
    /// signal = sg.generate(50000.0, 1000000.0)
    /// print(signal)
    /// ```
    pub fn generate(&mut self, price: f64, volume: f64) -> PyResult<(f64, String)> {
        let signal = self.inner.generate(price, volume);
        let direction = match signal {
            s if s > 0.5 => "BUY".to_string(),
            s if s < -0.5 => "SELL".to_string(),
            _ => "NEUTRAL".to_string(),
        };
        Ok((signal, direction))
    }
}

// ============================================================================
// PyGarchFilter — GARCH 변동성 필터를 Python으로 노출
// ============================================================================

/// Python 클래스: GARCH(1,1) 변동성 필터
///
/// 조건부 이분산(heteroskedasticity)을 모델링하여
/// 시장 변동성을 추적합니다.
///
/// 메서드:
/// - `new()`: 기본 파라미터로 초기화
/// - `update(log_return: f64)`: 로그 수익률로 업데이트
/// - `current_vol() -> f64`: 현재 변동성 반환
#[pyclass]
pub struct PyGarchFilter {
    inner: crate::garch::GarchFilter,
}

#[pymethods]
impl PyGarchFilter {
    /// GARCH 필터 생성
    ///
    /// # 예제
    /// ```python
    /// garch = PyGarchFilter()
    /// ```
    #[new]
    pub fn new() -> Self {
        PyGarchFilter {
            inner: crate::garch::GarchFilter::new(),
        }
    }

    /// 로그 수익률로 필터 업데이트
    ///
    /// # 인자
    /// - `log_return: f64` - 로그 수익률 (예: 가격 변화율의 자연로그)
    ///
    /// # 예제
    /// ```python
    /// # 50000 -> 50500 변화
    /// log_ret = math.log(50500.0 / 50000.0)
    /// garch.update(log_ret)
    /// ```
    pub fn update(&mut self, log_return: f64) {
        self.inner.update(log_return);
    }

    /// 현재 추정 변동성 반환
    ///
    /// # 반환
    /// - f64: 연간화된 변동성 (예: 0.25 = 25%)
    ///
    /// # 예제
    /// ```python
    /// vol = garch.current_vol()
    /// print(f"Current volatility: {vol * 100:.1f}%")
    /// ```
    pub fn current_vol(&self) -> f64 {
        self.inner.current_vol()
    }
}

// ============================================================================
// PyRollingWindow — 이동 윈도우를 Python으로 노출
// ============================================================================

/// Python 클래스: 이동 윈도우 통계량 계산기
///
/// 고정 크기의 슬라이딩 윈도우에서
/// 평균, 표준편차, Z-스코어를 효율적으로 계산합니다.
///
/// 메서드:
/// - `new(window_size: int)`: 지정된 크기로 초기화
/// - `push(value: f64)`: 새로운 값 추가
/// - `mean() -> f64`: 현재 평균 반환
/// - `std() -> f64`: 현재 표준편차 반환
/// - `z_score(value: f64) -> f64`: Z-스코어 계산
#[pyclass]
pub struct PyRollingWindow {
    inner: crate::features::RollingWindow,
}

#[pymethods]
impl PyRollingWindow {
    /// 이동 윈도우 생성
    ///
    /// # 인자
    /// - `window_size: usize` - 윈도우 크기 (예: 20)
    ///
    /// # 예제
    /// ```python
    /// rw = PyRollingWindow(20)  # 20개 요소의 윈도우
    /// ```
    #[new]
    pub fn new(window_size: usize) -> Self {
        PyRollingWindow {
            inner: crate::features::RollingWindow::new(window_size),
        }
    }

    /// 윈도우에 값 추가
    ///
    /// # 인자
    /// - `value: f64` - 추가할 값
    ///
    /// # 예제
    /// ```python
    /// for price in prices:
    ///     rw.push(price)
    /// ```
    pub fn push(&mut self, value: f64) {
        self.inner.push(value);
    }

    /// 현재 윈도우의 평균 반환
    ///
    /// # 반환
    /// - f64: 이동 평균
    ///
    /// # 예제
    /// ```python
    /// ma20 = rw.mean()
    /// ```
    pub fn mean(&self) -> f64 {
        self.inner.mean()
    }

    /// 현재 윈도우의 표준편차 반환
    ///
    /// # 반환
    /// - f64: 이동 표준편차
    ///
    /// # 예제
    /// ```python
    /// vol = rw.std()
    /// ```
    pub fn std(&self) -> f64 {
        self.inner.std()
    }

    /// 주어진 값의 Z-스코어 계산
    ///
    /// # 인자
    /// - `value: f64` - 계산 대상 값
    ///
    /// # 반환
    /// - f64: Z-스코어 ((value - mean) / std)
    ///
    /// # 예제
    /// ```python
    /// z = rw.z_score(price)
    /// if z > 2.0:
    ///     print("이상치 감지")
    /// ```
    pub fn z_score(&self, value: f64) -> f64 {
        self.inner.z_score(value)
    }
}

// ============================================================================
// PyO3 모듈 등록
// ============================================================================

/// Quant Engine Python 모듈
///
/// 모든 Rust 기반 퀀트 거래 엔진 컴포넌트를 Python으로 노출합니다.
///
/// # 사용 예
/// ```python
/// from quant_engine import (
///     PyKalmanFilter,
///     PyOuModel,
///     PySignalGenerator,
///     PyGarchFilter,
///     PyRollingWindow,
/// )
///
/// # Kalman Filter를 이용한 가격 추정
/// kf = PyKalmanFilter()
/// prices = [50000, 50100, 49950, 50200]
/// for price in prices:
///     kf.update(price)
///     pos, vel = kf.get_state()
///     print(f"Position: {pos:.2f}, Velocity: {vel:.4f}")
///
/// # OU 모델을 이용한 평균 복귀 거래
/// ou = PyOuModel()
/// for price in prices:
///     ou.update(price)
///     ou_val, zscore = ou.get_signal()
///     if zscore > 2.0:
///         print("Short signal (과매수)")
///     elif zscore < -2.0:
///         print("Long signal (과매도)")
///
/// # GARCH를 이용한 변동성 추적
/// garch = PyGarchFilter()
/// for i in range(1, len(prices)):
///     log_ret = math.log(prices[i] / prices[i-1])
///     garch.update(log_ret)
///     vol = garch.current_vol()
///     print(f"Volatility: {vol*100:.1f}%")
///
/// # 이동 윈도우를 이용한 통계량
/// rw = PyRollingWindow(3)
/// for price in prices:
///     rw.push(price)
///     print(f"Mean: {rw.mean():.2f}, Std: {rw.std():.2f}")
/// ```
#[pymodule]
pub fn quant_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyKalmanFilter>()?;
    m.add_class::<PyOuModel>()?;
    m.add_class::<PySignalGenerator>()?;
    m.add_class::<PyGarchFilter>()?;
    m.add_class::<PyRollingWindow>()?;

    // 모듈 정보
    m.add(
        "__doc__",
        "Quant Engine: Rust-based quantitative trading engine for Python",
    )?;
    m.add("__version__", "0.1.0")?;

    Ok(())
}

// ============================================================================
// 컴파일 조건부 테스트
// ============================================================================

#[cfg(all(test, feature = "python"))]
mod tests {
    use super::*;

    /// Kalman Filter 파이썬 바인딩 테스트
    #[test]
    fn test_py_kalman_filter() {
        let mut kf = PyKalmanFilter::new();

        // 연속된 측정값 입력
        let measurements = vec![50000.0, 50100.0, 49950.0, 50200.0];
        for &m in &measurements {
            kf.update(m);
        }

        let (pos, vel) = kf.get_state().unwrap();
        assert!(pos > 0.0, "위치는 양수여야 합니다");
        println!("Kalman Filter: position={:.2}, velocity={:.4}", pos, vel);
    }

    /// OU 모델 파이썬 바인딩 테스트
    #[test]
    fn test_py_ou_model() {
        let mut ou = PyOuModel::new();

        // 변동성 있는 가격 입력
        let prices = vec![1.0950, 1.0960, 1.0940, 1.0950];
        for &p in &prices {
            ou.update(p);
        }

        let (ou_val, zscore) = ou.get_signal().unwrap();
        println!("OU Model: ou_value={:.6}, zscore={:.4}", ou_val, zscore);
    }

    /// 신호 생성기 파이썬 바인딩 테스트
    #[test]
    fn test_py_signal_generator() {
        let mut sg = PySignalGenerator::new();

        let (signal, direction) = sg.generate(50000.0, 1000000.0).unwrap();
        println!("Signal Generator: signal={:.4}, direction={}", signal, direction);
        assert!(
            direction == "BUY" || direction == "SELL" || direction == "NEUTRAL",
            "방향은 BUY, SELL, NEUTRAL 중 하나여야 합니다"
        );
    }

    /// GARCH 필터 파이썬 바인딩 테스트
    #[test]
    fn test_py_garch_filter() {
        let mut garch = PyGarchFilter::new();

        // 로그 수익률 입력
        let log_returns = vec![0.002, -0.001, 0.003, -0.002];
        for &lr in &log_returns {
            garch.update(lr);
        }

        let vol = garch.current_vol();
        assert!(vol >= 0.0, "변동성은 음수가 아니어야 합니다");
        println!("GARCH Filter: volatility={:.4} ({:.1}%)", vol, vol * 100.0);
    }

    /// 이동 윈도우 파이썬 바인딩 테스트
    #[test]
    fn test_py_rolling_window() {
        let mut rw = PyRollingWindow::new(3);

        let values = vec![100.0, 102.0, 101.0, 103.0];
        for &v in &values {
            rw.push(v);
        }

        let mean = rw.mean();
        let std = rw.std();
        let zscore = rw.z_score(105.0);

        assert!(mean > 0.0, "평균은 양수여야 합니다");
        assert!(std >= 0.0, "표준편차는 음수가 아니어야 합니다");
        println!(
            "Rolling Window: mean={:.2}, std={:.2}, zscore(105.0)={:.4}",
            mean, std, zscore
        );
    }

    /// 통합 워크플로우 테스트
    #[test]
    fn test_integrated_workflow() {
        // 모든 컴포넌트를 조합하여 거래 신호 생성
        let mut kf = PyKalmanFilter::new();
        let mut ou = PyOuModel::new();
        let mut garch = PyGarchFilter::new();
        let mut rw = PyRollingWindow::new(5);

        let prices = vec![50000.0, 50100.0, 49950.0, 50200.0, 50150.0];

        for (i, &price) in prices.iter().enumerate() {
            // Kalman Filter
            kf.update(price);
            let (kf_pos, _) = kf.get_state().unwrap();

            // OU 모델
            ou.update(price);
            let (_, ou_zscore) = ou.get_signal().unwrap();

            // GARCH (로그 수익률)
            if i > 0 {
                let log_ret = (price / prices[i - 1]).ln();
                garch.update(log_ret);
                let vol = garch.current_vol();
                println!(
                    "Step {}: price={:.2}, kf_pos={:.2}, ou_zscore={:.4}, vol={:.4}",
                    i, price, kf_pos, ou_zscore, vol
                );
            }

            // 이동 윈도우
            rw.push(price);
            println!("  Rolling mean={:.2}, std={:.2}", rw.mean(), rw.std());
        }
    }
}
