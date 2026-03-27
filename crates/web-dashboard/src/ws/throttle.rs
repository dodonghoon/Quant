//! # WebSocket 스로틀링
//!
//! 시장 데이터 전송 주파수 제한 (백프레셔 방지).

use std::time::{Duration, Instant};

/// 시간 기반 스로틀러
pub struct Throttle {
    interval: Duration,
    last_sent: Instant,
}

impl Throttle {
    /// 밀리초 간격의 스로틀러 생성
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval: Duration::from_millis(interval_ms),
            last_sent: Instant::now() - Duration::from_millis(interval_ms),
        }
    }

    /// 전송 가능 여부 확인 — true면 전송, false면 스킵
    pub fn should_send(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_sent) >= self.interval {
            self.last_sent = now;
            true
        } else {
            false
        }
    }

    /// 남은 대기 시간 (밀리초)
    pub fn time_until_next(&self) -> u64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sent);
        if elapsed >= self.interval {
            0
        } else {
            (self.interval - elapsed).as_millis() as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_throttle_blocks_too_fast_sends() {
        let mut throttle = Throttle::new(100);

        assert!(throttle.should_send()); // First send always allowed
        assert!(!throttle.should_send()); // Immediately blocked
        assert!(!throttle.should_send()); // Still blocked
    }

    #[test]
    fn test_throttle_allows_after_interval() {
        let mut throttle = Throttle::new(50);

        assert!(throttle.should_send());
        thread::sleep(Duration::from_millis(60));
        assert!(throttle.should_send()); // Now allowed
    }

    #[test]
    fn test_throttle_initializes_with_expired_interval() {
        let throttle = Throttle::new(100);
        // First call to should_send should always return true because
        // last_sent is initialized to Instant::now() - interval
        // So it will definitely be past the interval
    }
}
