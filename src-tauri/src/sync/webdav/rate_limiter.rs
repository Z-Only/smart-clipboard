use std::sync::Mutex;
use std::time::Instant;

pub struct TokenBucketLimiter {
    state: Mutex<BucketState>,
    capacity: u32,
    refill_rate_per_sec: f64,
}

struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucketLimiter {
    pub fn new(capacity: u32, refill_period_minutes: u32) -> Self {
        let refill_rate_per_sec = f64::from(capacity) / (f64::from(refill_period_minutes) * 60.0);
        Self {
            state: Mutex::new(BucketState {
                tokens: f64::from(capacity),
                last_refill: Instant::now(),
            }),
            capacity,
            refill_rate_per_sec,
        }
    }

    fn refill(&self, state: &mut BucketState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        let added = elapsed * self.refill_rate_per_sec;
        state.tokens = (state.tokens + added).min(f64::from(self.capacity));
        state.last_refill = now;
    }

    /// Try to consume `count` tokens. Returns true if successful, false if insufficient.
    pub fn try_acquire(&self, count: u32) -> bool {
        let mut state = self.state.lock().unwrap();
        self.refill(&mut state);
        let needed = f64::from(count);
        if state.tokens >= needed {
            state.tokens -= needed;
            true
        } else {
            false
        }
    }

    /// Block until `count` tokens are available, then consume them.
    pub async fn acquire(&self, count: u32) -> f64 {
        let mut total_waited = 0.0;
        loop {
            {
                let mut state = self.state.lock().unwrap();
                self.refill(&mut state);
                let needed = f64::from(count);
                if state.tokens >= needed {
                    state.tokens -= needed;
                    return total_waited;
                }
                let deficit = needed - state.tokens;
                let wait_secs = deficit / self.refill_rate_per_sec;
                let wait_ms = (wait_secs * 1000.0).ceil().max(100.0) as u64;
                drop(state);
                tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
                total_waited += wait_ms as f64 / 1000.0;
            }
        }
    }

    /// Return the current number of available tokens (approximate).
    pub fn available(&self) -> u32 {
        let mut state = self.state.lock().unwrap();
        self.refill(&mut state);
        state.tokens.floor() as u32
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_limiter_starts_full() {
        let limiter = TokenBucketLimiter::new(100, 30);
        assert_eq!(limiter.available(), 100);
        assert_eq!(limiter.capacity(), 100);
    }

    #[test]
    fn test_try_acquire_success() {
        let limiter = TokenBucketLimiter::new(100, 30);
        assert!(limiter.try_acquire(10));
        assert_eq!(limiter.available(), 90);
    }

    #[test]
    fn test_try_acquire_insufficient() {
        let limiter = TokenBucketLimiter::new(5, 30);
        assert!(limiter.try_acquire(3));
        assert!(!limiter.try_acquire(5));
        assert_eq!(limiter.available(), 2);
    }

    #[test]
    fn test_try_acquire_exact() {
        let limiter = TokenBucketLimiter::new(10, 30);
        assert!(limiter.try_acquire(10));
        assert!(!limiter.try_acquire(1));
        assert_eq!(limiter.available(), 0);
    }

    #[tokio::test]
    async fn test_acquire_immediate_when_available() {
        let limiter = TokenBucketLimiter::new(100, 30);
        let waited = limiter.acquire(5).await;
        assert_eq!(waited, 0.0);
        assert_eq!(limiter.available(), 95);
    }
}
