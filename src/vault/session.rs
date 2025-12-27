//! Session management with timeout and rate limiting

use std::time::{Duration, Instant};
use zeroize::Zeroize;

/// Session manager - handles vault unlock state
pub struct SessionManager {
    vault_key: Option<[u8; 32]>,
    created_at: Option<Instant>,
    last_activity: Option<Instant>,
    timeout_secs: u64,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(300) // 5 minute default timeout
    }
}

impl SessionManager {
    /// Create a new session manager with the given timeout in seconds
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            vault_key: None,
            created_at: None,
            last_activity: None,
            timeout_secs,
        }
    }

    /// Start a session with the given vault key
    pub fn start(&mut self, key: [u8; 32]) {
        self.vault_key = Some(key);
        self.created_at = Some(Instant::now());
        self.last_activity = Some(Instant::now());
    }

    /// Check if session is active and valid
    pub fn is_active(&self) -> bool {
        if self.vault_key.is_none() {
            return false;
        }

        if let Some(last) = self.last_activity {
            last.elapsed() < Duration::from_secs(self.timeout_secs)
        } else {
            false
        }
    }

    /// Touch the session (update last activity)
    pub fn touch(&mut self) {
        if self.vault_key.is_some() {
            self.last_activity = Some(Instant::now());
        }
    }

    /// Get the vault key if session is active
    pub fn get_key(&mut self) -> Option<&[u8; 32]> {
        if self.is_active() {
            self.touch();
            self.vault_key.as_ref()
        } else {
            self.end();
            None
        }
    }

    /// End the session and zeroize the key
    pub fn end(&mut self) {
        if let Some(ref mut key) = self.vault_key {
            key.zeroize();
        }
        self.vault_key = None;
        self.created_at = None;
        self.last_activity = None;
    }

    /// Get remaining session time in seconds
    pub fn remaining_secs(&self) -> u64 {
        if let Some(last) = self.last_activity {
            let elapsed = last.elapsed().as_secs();
            if elapsed < self.timeout_secs {
                self.timeout_secs - elapsed
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Set the timeout duration
    pub fn set_timeout(&mut self, secs: u64) {
        self.timeout_secs = secs;
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        self.end();
    }
}

/// Rate limiter for brute-force protection
pub struct RateLimiter {
    failed_attempts: u32,
    last_attempt: Option<Instant>,
    locked_until: Option<Instant>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    /// Maximum failed attempts before lockout (stricter for 6-digit PIN)
    const MAX_ATTEMPTS: u32 = 3;
    /// Base lockout duration in seconds (longer for PIN)
    const BASE_LOCKOUT_SECS: u64 = 60;
    /// Maximum lockout duration (30 minutes)
    const MAX_LOCKOUT_SECS: u64 = 1800;
    /// Reset attempts after this many seconds of inactivity
    const RESET_AFTER_SECS: u64 = 3600;

    pub fn new() -> Self {
        Self {
            failed_attempts: 0,
            last_attempt: None,
            locked_until: None,
        }
    }

    /// Check if currently locked out. Returns error message if locked.
    pub fn check_locked(&mut self) -> Result<(), String> {
        // Reset attempts if enough time has passed
        if let Some(last) = self.last_attempt {
            if last.elapsed() > Duration::from_secs(Self::RESET_AFTER_SECS) {
                self.failed_attempts = 0;
                self.locked_until = None;
            }
        }

        // Check if still in lockout period
        if let Some(until) = self.locked_until {
            if Instant::now() < until {
                let remaining = until.duration_since(Instant::now());
                return Err(format!(
                    "Too many failed attempts. Try again in {} seconds",
                    remaining.as_secs()
                ));
            }
            self.locked_until = None;
        }

        Ok(())
    }

    /// Record a failed attempt
    pub fn record_failure(&mut self) {
        self.failed_attempts += 1;
        self.last_attempt = Some(Instant::now());

        if self.failed_attempts >= Self::MAX_ATTEMPTS {
            // Exponential backoff
            let multiplier = (self.failed_attempts - Self::MAX_ATTEMPTS) / Self::MAX_ATTEMPTS;
            let lockout_secs = Self::BASE_LOCKOUT_SECS * 2u64.pow(multiplier);
            let lockout_secs = lockout_secs.min(Self::MAX_LOCKOUT_SECS);

            self.locked_until = Some(Instant::now() + Duration::from_secs(lockout_secs));
        }
    }

    /// Record a successful attempt (resets counter)
    pub fn record_success(&mut self) {
        self.failed_attempts = 0;
        self.last_attempt = None;
        self.locked_until = None;
    }

    /// Get remaining lockout time in seconds
    pub fn lockout_remaining(&self) -> Option<u64> {
        self.locked_until.map(|until| {
            if Instant::now() < until {
                until.duration_since(Instant::now()).as_secs()
            } else {
                0
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_lifecycle() {
        let mut session = SessionManager::new(300);
        assert!(!session.is_active());

        session.start([42u8; 32]);
        assert!(session.is_active());

        assert!(session.get_key().is_some());

        session.end();
        assert!(!session.is_active());
        assert!(session.get_key().is_none());
    }

    #[test]
    fn rate_limiter_allows_initial_attempts() {
        let mut limiter = RateLimiter::new();
        assert!(limiter.check_locked().is_ok());
    }

    #[test]
    fn rate_limiter_locks_after_max_attempts() {
        let mut limiter = RateLimiter::new();

        for _ in 0..RateLimiter::MAX_ATTEMPTS {
            limiter.record_failure();
        }

        assert!(limiter.check_locked().is_err());
    }

    #[test]
    fn rate_limiter_resets_on_success() {
        let mut limiter = RateLimiter::new();

        for _ in 0..3 {
            limiter.record_failure();
        }

        limiter.record_success();
        assert_eq!(limiter.failed_attempts, 0);
    }
}
