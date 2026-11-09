// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! Timeouts that back off exponentially.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::tokio::time::{
    Duration,
    Instant,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A timeout that backs off exponentially.
///
/// The timeout first expires after an initial wait. Each time it expires, it may be extended by a
/// wait that is twice as long as the previous one, until a maximum timeout is reached. This lets
/// callers report a slow operation at exponentially growing intervals, rather than at fixed ones,
/// and give up on it once the maximum timeout is reached.
///
#[derive(Debug)]
pub struct BackoffTimeout {
    /// Instant at which the timeout started.
    start: Instant,
    /// Duration of the current wait.
    wait: Duration,
    /// Time since the start at which the current wait expires.
    deadline: Duration,
    /// Time since the start past which the timeout is no longer extended.
    max_timeout: Duration,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl BackoffTimeout {
    ///
    /// # Description
    ///
    /// Starts a timeout that backs off exponentially.
    ///
    /// # Parameters
    ///
    /// - `initial_wait`: Duration of the first wait. It is capped at `max_timeout`.
    /// - `max_timeout`: Maximum time to wait, across all waits.
    ///
    /// # Returns
    ///
    /// A timeout whose first wait starts now.
    ///
    pub fn start(initial_wait: Duration, max_timeout: Duration) -> Self {
        // A zero wait never grows when doubled, so wait for the maximum timeout at once instead.
        let wait: Duration = if initial_wait.is_zero() {
            max_timeout
        } else {
            initial_wait.min(max_timeout)
        };
        Self {
            start: Instant::now(),
            wait,
            deadline: wait,
            max_timeout,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the time elapsed since the timeout started.
    ///
    /// # Returns
    ///
    /// The time elapsed since the timeout started.
    ///
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    ///
    /// # Description
    ///
    /// Returns the time remaining until the current wait expires. Because this is measured against
    /// the start of the timeout, time spent between waits does not extend the timeout.
    ///
    /// # Returns
    ///
    /// The time remaining until the current wait expires, or zero if it has already expired.
    ///
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_sub(self.elapsed())
    }

    ///
    /// # Description
    ///
    /// Extends the timeout by a wait that is twice as long as the current one, without exceeding
    /// the maximum timeout. Callers should invoke this once the current wait expires.
    ///
    /// # Returns
    ///
    /// `true` if the timeout was extended, or `false` if the maximum timeout was already reached, in
    /// which case callers should give up.
    ///
    pub fn extend(&mut self) -> bool {
        if self.deadline >= self.max_timeout {
            return false;
        }
        self.wait = self.wait.saturating_mul(2);
        self.deadline = self
            .deadline
            .saturating_add(self.wait)
            .min(self.max_timeout);
        true
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests;
