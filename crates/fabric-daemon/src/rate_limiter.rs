//! Token-bucket rate limiter for per-IP request throttling.
//!
//! Each client IP gets its own token bucket that refills at a configurable
//! rate. When a bucket is empty, subsequent requests are rejected with a
//! 429-equivalent error. Stale entries are cleaned up periodically to
//! prevent unbounded memory growth.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Default requests per second.
pub const DEFAULT_RPS: u32 = 100;

/// Interval between stale-entry cleanups.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// Threshold after which an entry is considered stale.
const STALE_THRESHOLD: Duration = Duration::from_secs(120);

/// A single client's token bucket.
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Remaining tokens.
    tokens: f64,
    /// Maximum burst size (= rps).
    capacity: f64,
    /// Tokens added per second.
    refill_rate: f64,
    /// Last time tokens were refilled.
    last_refill: Instant,
}

impl TokenBucket {
    /// Try to consume one token. Returns `true` if allowed.
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// Returns `true` if the bucket hasn't been touched in `STALE_THRESHOLD`.
    fn is_stale(&self) -> bool {
        self.last_refill.elapsed() > STALE_THRESHOLD
    }
}

/// Thread-safe per-IP rate limiter backed by a token bucket.
pub struct RateLimiter {
    /// Per-IP buckets.
    buckets: RwLock<HashMap<IpAddr, TokenBucket>>,
    /// Requests per second allowed per IP.
    rps: u32,
    /// Last cleanup timestamp.
    last_cleanup: RwLock<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given requests-per-second limit.
    pub fn new(rps: u32) -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            rps,
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Create a rate limiter with the default RPS.
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_RPS)
    }

    /// Check whether a request from `ip` is allowed.
    ///
    /// Returns `Ok(())` if allowed, or `Err(RateLimitError)` if the limit
    /// has been exceeded.
    pub fn check(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        // Insert a fresh bucket if this IP is new.
        {
            let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
            if !buckets.contains_key(&ip) {
                buckets.insert(
                    ip,
                    TokenBucket {
                        tokens: self.rps as f64,
                        capacity: self.rps as f64,
                        refill_rate: self.rps as f64,
                        last_refill: Instant::now(),
                    },
                );
            }
        }

        // Try to consume a token.
        {
            let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
            if let Some(bucket) = buckets.get_mut(&ip) {
                if bucket.try_consume() {
                    return Ok(());
                }
            }
        }

        // Periodically clean up stale entries.
        self.maybe_cleanup();

        Err(RateLimitError {
            ip,
            retry_after: Duration::from_secs(1),
        })
    }

    /// Remove stale entries if enough time has passed since the last cleanup.
    fn maybe_cleanup(&self) {
        let mut last = self.last_cleanup.write().unwrap_or_else(|e| e.into_inner());
        if last.elapsed() < CLEANUP_INTERVAL {
            return;
        }
        *last = Instant::now();

        let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        buckets.retain(|_, bucket| !bucket.is_stale());
    }

    /// Get the number of tracked IPs (useful for diagnostics).
    pub fn tracked_ips(&self) -> usize {
        self.buckets.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Get the configured RPS limit.
    pub fn rps(&self) -> u32 {
        self.rps
    }
}

/// Error returned when a request is rate-limited.
#[derive(Debug)]
pub struct RateLimitError {
    /// The client IP that was limited.
    pub ip: IpAddr,
    /// Suggested time to wait before retrying.
    pub retry_after: Duration,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rate limit exceeded for {}, retry after {}s",
            self.ip,
            self.retry_after.as_secs()
        )
    }
}

impl std::error::Error for RateLimitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_under_limit() {
        let limiter = RateLimiter::new(10);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        // Should allow up to rps requests.
        for _ in 0..10 {
            assert!(limiter.check(ip).is_ok());
        }
    }

    #[test]
    fn rejects_over_limit() {
        let limiter = RateLimiter::new(5);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        for _ in 0..5 {
            assert!(limiter.check(ip).is_ok());
        }
        assert!(limiter.check(ip).is_err());
    }

    #[test]
    fn different_ips_are_independent() {
        let limiter = RateLimiter::new(2);
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip1).is_err());

        // ip2 should still be allowed.
        assert!(limiter.check(ip2).is_ok());
    }

    #[test]
    fn recovers_after_time() {
        // Use a tiny bucket: 1 rps, refill almost immediately for test.
        let limiter = RateLimiter::new(1);
        let ip: IpAddr = "127.0.0.2".parse().unwrap();

        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_err());

        // Simulate time passing by creating a new bucket entry.
        // In real code this happens via refill. We'll just check that
        // the bucket refills over time.
        std::thread::sleep(Duration::from_millis(1100));
        assert!(limiter.check(ip).is_ok());
    }

    #[test]
    fn default_rps() {
        let limiter = RateLimiter::with_defaults();
        assert_eq!(limiter.rps(), DEFAULT_RPS);
    }

    #[test]
    fn tracked_ips_count() {
        let limiter = RateLimiter::new(100);
        let ip1: IpAddr = "1.1.1.1".parse().unwrap();
        let ip2: IpAddr = "2.2.2.2".parse().unwrap();

        assert_eq!(limiter.tracked_ips(), 0);
        limiter.check(ip1).ok();
        assert_eq!(limiter.tracked_ips(), 1);
        limiter.check(ip2).ok();
        assert_eq!(limiter.tracked_ips(), 2);
    }

    #[test]
    fn error_contains_ip() {
        let limiter = RateLimiter::new(1);
        let ip: IpAddr = "10.10.10.10".parse().unwrap();
        limiter.check(ip).unwrap();
        let err = limiter.check(ip).unwrap_err();
        assert_eq!(err.ip, ip);
    }
}
