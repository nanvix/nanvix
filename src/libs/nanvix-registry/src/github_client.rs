// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    debug,
    error,
};
use ::reqwest::{
    Client,
    RequestBuilder,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Environment variable name for GitHub token (primary).
const GITHUB_TOKEN_ENV: &str = "GITHUB_TOKEN";

/// Environment variable name for GitHub token (fallback, GitHub CLI convention).
const GH_TOKEN_ENV: &str = "GH_TOKEN";

/// User-Agent header value for all requests.
const USER_AGENT: &str = "nanvix-registry";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Retrieves the GitHub authentication token from environment variables.
///
/// Checks `GITHUB_TOKEN` first, then falls back to `GH_TOKEN` (GitHub CLI convention).
///
/// # Returns
///
/// The token as a `Some(String)` if found, or `None` if neither variable is set.
///
pub(crate) fn get_github_token() -> Option<String> {
    if let Ok(token) = ::std::env::var(GITHUB_TOKEN_ENV) {
        if !token.is_empty() {
            debug!("Using GitHub token from {}", GITHUB_TOKEN_ENV);
            return Some(token);
        }
    }

    if let Ok(token) = ::std::env::var(GH_TOKEN_ENV) {
        if !token.is_empty() {
            debug!("Using GitHub token from {}", GH_TOKEN_ENV);
            return Some(token);
        }
    }

    debug!("No GitHub token found in environment");
    None
}

///
/// # Description
///
/// Returns whether a GitHub authentication token is available.
///
/// # Returns
///
/// `true` if a token is available, `false` otherwise.
///
pub(crate) fn is_authenticated() -> bool {
    get_github_token().is_some()
}

///
/// # Description
///
/// Builds a `reqwest::Client` with a default `User-Agent` header.
///
/// # Returns
///
/// On success, returns a configured `Client`. On failure, returns an error.
///
pub(crate) fn build_client() -> Result<Client> {
    match Client::builder().user_agent(USER_AGENT).build() {
        Ok(client) => Ok(client),
        Err(error) => {
            let reason: String = format!("Failed to build HTTP client: {error}");
            error!("{reason}");
            anyhow::bail!(reason)
        },
    }
}

///
/// # Description
///
/// Creates an authenticated GET request using the provided client.
///
/// If a GitHub token is available in the environment, the `Authorization: Bearer <token>` header
/// is added to the request. Otherwise, the request is sent without authentication.
///
/// # Parameters
///
/// - `client`: The HTTP client to use for the request.
/// - `url`: The URL to send the GET request to.
///
/// # Returns
///
/// A `RequestBuilder` with the appropriate authentication headers set.
///
pub(crate) fn authenticated_get(client: &Client, url: &str) -> RequestBuilder {
    let request: RequestBuilder = client.get(url);

    if let Some(token) = get_github_token() {
        debug!("Adding Bearer token to request");
        request.header("Authorization", format!("Bearer {token}"))
    } else {
        request
    }
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
    /// Tests that build_client creates a client successfully.
    ///
    #[test]
    fn test_build_client() {
        let result: Result<Client> = build_client();
        assert!(result.is_ok());
    }

    ///
    /// # Description
    ///
    /// Tests that authenticated_get returns a request builder.
    ///
    #[test]
    fn test_authenticated_get() {
        let client: Client = build_client().expect("failed to build HTTP client");
        let _request: RequestBuilder = authenticated_get(&client, "https://example.com");
    }

    ///
    /// # Description
    ///
    /// Tests the user agent constant.
    ///
    #[test]
    fn test_user_agent() {
        assert_eq!(USER_AGENT, "nanvix-registry");
    }

    ///
    /// # Description
    ///
    /// Tests the environment variable name constants.
    ///
    #[test]
    fn test_env_var_names() {
        assert_eq!(GITHUB_TOKEN_ENV, "GITHUB_TOKEN");
        assert_eq!(GH_TOKEN_ENV, "GH_TOKEN");
    }
}
