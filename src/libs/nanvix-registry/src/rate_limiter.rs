// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::github_client;
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    warn,
};
use ::reqwest::Response;
use ::std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use ::tokio::sync::Mutex;

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum delay between GitHub API requests for unauthenticated access (in milliseconds).
/// GitHub's rate limit for unauthenticated requests is 60 requests per hour.
/// This 1-second delay helps stay well within limits during bulk operations.
const UNAUTHENTICATED_MIN_REQUEST_DELAY_MS: u64 = 1000;

/// Minimum delay between GitHub API requests for authenticated access (in milliseconds).
/// GitHub's rate limit for authenticated requests is 5000 requests per hour.
/// This 200ms delay provides comfortable margin during bulk operations.
const AUTHENTICATED_MIN_REQUEST_DELAY_MS: u64 = 200;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A simple rate limiter to prevent hitting GitHub API rate limits.
///
/// This limiter enforces a minimum delay between consecutive API requests,
/// helping to avoid rate limit errors during bulk package installations.
///
#[derive(Clone)]
pub(crate) struct RateLimiter {
    /// Timestamp of the last API request.
    last_request: Arc<Mutex<Option<Instant>>>,
    /// Minimum delay between requests.
    min_delay: Duration,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RateLimiter {
    ///
    /// # Description
    ///
    /// Creates a new rate limiter with the default minimum delay for unauthenticated requests.
    ///
    /// # Returns
    ///
    /// A new `RateLimiter` instance.
    ///
    pub(crate) fn new() -> Self {
        Self {
            last_request: Arc::new(Mutex::new(None)),
            min_delay: Duration::from_millis(UNAUTHENTICATED_MIN_REQUEST_DELAY_MS),
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new rate limiter with a custom minimum delay.
    ///
    /// # Parameters
    ///
    /// - `min_delay_ms`: Minimum delay between requests in milliseconds.
    ///
    /// # Returns
    ///
    /// A new `RateLimiter` instance.
    ///
    pub(crate) fn with_delay(min_delay_ms: u64) -> Self {
        Self {
            last_request: Arc::new(Mutex::new(None)),
            min_delay: Duration::from_millis(min_delay_ms),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the minimum delay between requests.
    ///
    /// # Returns
    ///
    /// The minimum delay as a `Duration`.
    ///
    #[allow(dead_code)]
    pub(crate) fn min_delay(&self) -> Duration {
        self.min_delay
    }

    ///
    /// # Description
    ///
    /// Creates a new rate limiter configured for GitHub API access.
    ///
    /// The delay is automatically adjusted based on whether a GitHub authentication token is
    /// available in the environment. Authenticated requests allow 5000 requests per hour (200ms
    /// delay), while unauthenticated requests are limited to 60 per hour (1000ms delay).
    ///
    /// # Returns
    ///
    /// A new `RateLimiter` instance with the appropriate delay.
    ///
    pub(crate) fn for_github() -> Self {
        let delay_ms: u64 = if github_client::is_authenticated() {
            debug!(
                "Rate limiter: using authenticated delay ({}ms)",
                AUTHENTICATED_MIN_REQUEST_DELAY_MS
            );
            AUTHENTICATED_MIN_REQUEST_DELAY_MS
        } else {
            debug!(
                "Rate limiter: using unauthenticated delay ({}ms)",
                UNAUTHENTICATED_MIN_REQUEST_DELAY_MS
            );
            UNAUTHENTICATED_MIN_REQUEST_DELAY_MS
        };

        Self::with_delay(delay_ms)
    }

    ///
    /// # Description
    ///
    /// Waits if necessary to respect rate limits before making an API request.
    ///
    /// If the minimum delay since the last request has not elapsed, this method
    /// will sleep for the remaining time. Otherwise, it returns immediately.
    ///
    pub(crate) async fn wait(&self) {
        let mut last_request: tokio::sync::MutexGuard<'_, Option<Instant>> =
            self.last_request.lock().await;

        if let Some(last) = *last_request {
            let elapsed: Duration = last.elapsed();
            if elapsed < self.min_delay {
                let sleep_duration: Duration = self.min_delay - elapsed;
                debug!(
                    "Rate limiter: waiting {}ms before next request",
                    sleep_duration.as_millis()
                );
                tokio::time::sleep(sleep_duration).await;
            }
        }

        *last_request = Some(Instant::now());
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks an HTTP response for GitHub API rate limit errors and returns an actionable error if
/// detected.
///
/// Inspects the response status code for 403 (Forbidden) and 429 (Too Many Requests), which
/// indicate rate limit exhaustion. When detected, an error is returned with guidance on how to
/// resolve the issue by setting `GITHUB_TOKEN` or `GH_TOKEN` environment variables.
///
/// # Parameters
///
/// - `response`: The HTTP response to check.
///
/// # Returns
///
/// On success (no rate limit issue), returns an empty tuple. On failure, returns an error with
/// an actionable message.
///
pub(crate) fn check_rate_limit(response: &Response) -> Result<()> {
    let status: ::reqwest::StatusCode = response.status();

    // Log remaining rate limit if the header is present.
    if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
        if let Ok(remaining_str) = remaining.to_str() {
            debug!("GitHub API rate limit remaining: {}", remaining_str);

            // Warn if running low on remaining requests.
            if let Ok(remaining_count) = remaining_str.parse::<u64>() {
                if remaining_count <= 10 {
                    warn!(
                        "GitHub API rate limit nearly exhausted: {} requests remaining",
                        remaining_count
                    );
                }
            }
        }
    }

    if status == ::reqwest::StatusCode::FORBIDDEN
        || status == ::reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        let reason: String = if github_client::is_authenticated() {
            "GitHub API rate limit exceeded (authenticated).".to_string()
        } else {
            "GitHub API rate limit exceeded. Set GITHUB_TOKEN (or GH_TOKEN) environment variable \
             to increase limits (60 req/hr -> 5000 req/hr)."
                .to_string()
        };
        error!("{reason}");
        anyhow::bail!(reason)
    }

    Ok(())
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    ///
    /// # Description
    ///
    /// Tests that the rate limiter creates successfully with default settings.
    ///
    #[test]
    fn test_rate_limiter_new() {
        let limiter: RateLimiter = RateLimiter::new();
        assert_eq!(limiter.min_delay().as_millis(), UNAUTHENTICATED_MIN_REQUEST_DELAY_MS as u128);
    }

    ///
    /// # Description
    ///
    /// Tests that the rate limiter creates successfully with custom delay.
    ///
    #[test]
    fn test_rate_limiter_with_delay() {
        let limiter: RateLimiter = RateLimiter::with_delay(500);
        assert_eq!(limiter.min_delay().as_millis(), 500);
    }

    ///
    /// # Description
    ///
    /// Tests that first request does not wait.
    ///
    #[tokio::test]
    async fn test_rate_limiter_first_request_no_wait() {
        let limiter: RateLimiter = RateLimiter::with_delay(100);
        let start: Instant = Instant::now();
        limiter.wait().await;
        // First request should not wait.
        assert!(start.elapsed().as_millis() < 50);
    }

    ///
    /// # Description
    ///
    /// Tests that consecutive requests respect the minimum delay.
    ///
    #[tokio::test]
    async fn test_rate_limiter_enforces_delay() {
        let limiter: RateLimiter = RateLimiter::with_delay(100);

        // First request.
        limiter.wait().await;

        // Second request should wait.
        let start: Instant = Instant::now();
        limiter.wait().await;

        // Should have waited approximately 100ms.
        let elapsed: u128 = start.elapsed().as_millis();
        assert!(elapsed >= 90, "Expected delay of ~100ms, got {}ms", elapsed);
    }

    ///
    /// # Description
    ///
    /// Tests that no wait is needed if enough time has elapsed.
    ///
    #[tokio::test]
    async fn test_rate_limiter_no_wait_after_delay() {
        let limiter: RateLimiter = RateLimiter::with_delay(50);

        // First request.
        limiter.wait().await;

        // Wait longer than the minimum delay.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Second request should not need to wait.
        let start: Instant = Instant::now();
        limiter.wait().await;
        assert!(start.elapsed().as_millis() < 20);
    }

    ///
    /// # Description
    ///
    /// Tests that the for_github constructor creates a rate limiter.
    ///
    #[test]
    fn test_rate_limiter_for_github() {
        let limiter: RateLimiter = RateLimiter::for_github();
        // The delay should be one of the two known values.
        let delay: u128 = limiter.min_delay().as_millis();
        assert!(
            delay == AUTHENTICATED_MIN_REQUEST_DELAY_MS as u128
                || delay == UNAUTHENTICATED_MIN_REQUEST_DELAY_MS as u128,
            "Unexpected delay: {}ms",
            delay
        );
    }

    ///
    /// # Description
    ///
    /// Tests the authenticated delay constant is lower than unauthenticated.
    ///
    #[test]
    fn test_delay_constants() {
        const { assert!(AUTHENTICATED_MIN_REQUEST_DELAY_MS < UNAUTHENTICATED_MIN_REQUEST_DELAY_MS) };
    }
}
