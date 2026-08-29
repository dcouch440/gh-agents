//! A token bucket for pacing outbound requests.
//!
//! Shared by the LLM rate limiter, which paces by requests-per-minute, and the
//! `brave_search` tool, which paces by requests-per-second. The two differ only
//! in capacity and refill rate, so they use one implementation rather than two
//! that drift apart.

use std::time::Duration;

// `tokio::time::Instant`, not `std`'s: it is the same clock in production, but
// it is the one `sleep` advances, so tests can pause time instead of waiting.
use tokio::time::Instant;

#[cfg(test)]
mod tests;

/// A classic token bucket: `capacity` tokens, refilled at `refill_rate` per
/// second, one consumed per request.
///
/// Capacity is the burst allowance, and it is what separates the two callers:
/// 60 requests per minute with capacity 60 lets all 60 leave at once, whereas
/// 1 per second with capacity 1 genuinely serializes them.
#[derive(Debug)]
pub struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    /// A bucket that allows `requests_per_minute`, bursting the whole minute's
    /// allowance at once.
    pub fn per_minute(requests_per_minute: usize) -> Self {
        let capacity = requests_per_minute as f64;
        Self::new(capacity, capacity / 60.0)
    }

    /// A bucket that allows `requests_per_second`, bursting at most one
    /// second's worth.
    ///
    /// Capacity is floored at one token: a rate below 1/s still has to let a
    /// single request through, or nothing would ever be admitted.
    pub fn per_second(requests_per_second: f64) -> Self {
        Self::new(requests_per_second.max(1.0), requests_per_second)
    }

    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// Wait until a token is available, then consume it.
    pub async fn acquire(&mut self) {
        loop {
            self.refill();
            if self.tokens >= 1.0 {
                self.tokens -= 1.0;
                return;
            }
            // Sleeping for exactly the shortfall would race the clock and spin;
            // the loop re-checks, so an early wake costs one extra pass.
            let wait = (1.0 - self.tokens) / self.refill_rate;
            tokio::time::sleep(Duration::from_secs_f64(wait)).await;
        }
    }
}
