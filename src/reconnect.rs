use std::time::Duration;

/// Bounded exponential backoff for a future E2E session transport. Keeping it
/// independent from relay signalling prevents reconnect loops from silently
/// becoming unattended access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconnectPolicy {
    initial_delay: Duration,
    maximum_delay: Duration,
    maximum_attempts: u8,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            maximum_delay: Duration::from_secs(30),
            maximum_attempts: 5,
        }
    }
}

impl ReconnectPolicy {
    /// Returns the delay before a zero-based retry attempt, or `None` when the
    /// user must explicitly start a new connection attempt.
    pub fn delay_for(&self, attempt: u8) -> Option<Duration> {
        if attempt >= self.maximum_attempts {
            return None;
        }
        let multiplier = 1_u32.checked_shl(u32::from(attempt)).unwrap_or(u32::MAX);
        Some((self.initial_delay * multiplier).min(self.maximum_delay))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_bounded_and_requires_a_new_user_action_after_limit() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.delay_for(0), Some(Duration::from_secs(1)));
        assert_eq!(policy.delay_for(4), Some(Duration::from_secs(16)));
        assert_eq!(policy.delay_for(5), None);
    }
}
