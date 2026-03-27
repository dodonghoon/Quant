//! # Rolling Window & Online Statistics
//!
//! 고정 크기 순환 버퍼(Circular Buffer)로 실시간 스트리밍 통계를 계산합니다.
//!
//! ## 설계 원칙
//! - **할당 없음**: 생성 시 한 번 `Vec` 할당 후 재할당 없음
//! - **O(1) 업데이트**: Welford 알고리즘으로 mean/variance 증분 계산
//! - **Z-Score 즉시 반환**: Strategy Engine이 매 틱마다 호출

/// Welford 온라인 알고리즘 기반 순환 버퍼.
///
/// 매 `push`마다 O(1)로 mean, variance, z-score를 갱신합니다.
/// `capacity`는 생성 시 고정되며 이후 힙 할당이 발생하지 않습니다.
pub struct RollingWindow {
    buf: Vec<f64>,
    capacity: usize,
    head: usize,   // 다음 쓰기 위치
    count: usize,  // 현재 채워진 수 (≤ capacity)

    // Welford 상태
    mean: f64,
    m2: f64,       // 편차 제곱합 (variance = m2 / count)
    sum: f64,

    // 최근 값 캐시
    last_value: f64,
}

impl RollingWindow {
    /// `capacity`개 슬롯의 순환 버퍼 생성
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be > 0");
        Self {
            buf: vec![0.0; capacity],
            capacity,
            head: 0,
            count: 0,
            mean: 0.0,
            m2: 0.0,
            sum: 0.0,
            last_value: f64::NAN,
        }
    }

    /// 새 값 추가. 버퍼가 가득 차면 가장 오래된 값을 밀어냄.
    ///
    /// O(1) 연산: Welford 알고리즘으로 mean/variance 증분 갱신.
    pub fn push(&mut self, value: f64) {
        self.last_value = value;

        if self.count < self.capacity {
            // ── 버퍼 채우는 단계 (warm-up) ──
            self.buf[self.head] = value;
            self.count += 1;
            self.sum += value;
            let delta = value - self.mean;
            self.mean += delta / self.count as f64;
            let delta2 = value - self.mean;
            self.m2 += delta * delta2;
        } else {
            // ── 순환 단계: 오래된 값 제거 + 새 값 추가 ──
            let old = self.buf[self.head];
            self.buf[self.head] = value;
            self.sum += value - old;

            // Welford 업데이트 (제거 + 추가)
            let old_mean = self.mean;
            self.mean += (value - old) / self.count as f64;
            // m2 보정: 새 값과 이전 값의 편차를 이용
            self.m2 += (value - old) * (value - self.mean + old - old_mean);

            // m2가 수치 오차로 음수가 되는 것 방지
            if self.m2 < 0.0 {
                self.m2 = 0.0;
            }
        }

        self.head = (self.head + 1) % self.capacity;
    }

    /// 현재 평균
    #[inline]
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// 현재 분산 (모분산, N으로 나눔)
    #[inline]
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2 / self.count as f64
    }

    /// 현재 표준편차
    #[inline]
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// 최근 값의 Z-Score = (last - mean) / std_dev
    ///
    /// 표준편차가 0이면 0.0 반환 (발산 방지)
    #[inline]
    pub fn z_score(&self) -> f64 {
        let sd = self.std_dev();
        if sd < 1e-15 {
            return 0.0;
        }
        (self.last_value - self.mean) / sd
    }

    /// 합계
    #[inline]
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// 마지막으로 추가된 값
    #[inline]
    pub fn last(&self) -> f64 {
        self.last_value
    }

    /// 버퍼가 완전히 채워졌는지 (warm-up 완료 여부)
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.capacity
    }

    /// 현재 채워진 샘플 수
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// 전체 용량
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 내부 버퍼를 시간 순서로 반환 (백테스팅/디버깅용)
    pub fn as_ordered_slice(&self) -> Vec<f64> {
        let mut result = Vec::with_capacity(self.count);
        if self.count < self.capacity {
            result.extend_from_slice(&self.buf[..self.count]);
        } else {
            // head가 가리키는 곳이 가장 오래된 값
            result.extend_from_slice(&self.buf[self.head..]);
            result.extend_from_slice(&self.buf[..self.head]);
        }
        result
    }
}

// ────────────────────────────────────────────
// EMA (Exponential Moving Average) — 메모리 O(1)
// ────────────────────────────────────────────

/// 지수이동평균. 상태를 단 2개 변수(ema, initialized)로 유지합니다.
pub struct Ema {
    alpha: f64,
    value: f64,
    initialized: bool,
}

impl Ema {
    /// `period`에 해당하는 EMA 생성 (alpha = 2 / (period + 1))
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "EMA period must be > 0");
        Self {
            alpha: 2.0 / (period as f64 + 1.0),
            value: 0.0,
            initialized: false,
        }
    }

    /// 새 값 추가 후 현재 EMA 반환
    pub fn update(&mut self, value: f64) -> f64 {
        if !self.initialized {
            self.value = value;
            self.initialized = true;
        } else {
            self.value = self.alpha * value + (1.0 - self.alpha) * self.value;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_mean_and_zscore() {
        let mut rw = RollingWindow::new(5);
        for v in [10.0, 20.0, 30.0, 40.0, 50.0] {
            rw.push(v);
        }
        assert!(rw.is_ready());
        assert!((rw.mean() - 30.0).abs() < 1e-10);

        // z-score of 50 with mean=30, std ~= 14.14
        let z = rw.z_score();
        assert!(z > 1.0, "z-score should be > 1 for highest value");
    }

    #[test]
    fn test_rolling_circular_overwrite() {
        let mut rw = RollingWindow::new(3);
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            rw.push(v);
        }
        // 버퍼에는 [3, 4, 5]만 남아야 함
        assert!((rw.mean() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_ema_convergence() {
        let mut ema = Ema::new(10);
        for _ in 0..100 {
            ema.update(42.0);
        }
        assert!((ema.value() - 42.0).abs() < 1e-10);
    }
}
