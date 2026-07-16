// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod error;
pub mod filter;
pub mod io;
pub(crate) mod platform;
pub mod query;
pub mod socket;
mod types;

pub use filter::{
    HostFilter,
    Ipv4Cidr,
};

//==================================================================================================
// NetBackend
//==================================================================================================

/// Platform-agnostic networking backend.
///
/// This struct encapsulates all platform-specific networking logic and provides a clean Rust API
/// for socket operations. On Linux, it calls libc directly. On Windows, it uses the Winsock2 API.
///
/// `NetBackend` centralizes host socket operations for the network daemon.
pub struct NetBackend;

impl NetBackend {
    /// Creates a new `NetBackend` instance.
    ///
    /// Initializes the platform networking subsystem (no-op on Unix; calls `WSAStartup` on
    /// Windows). Returns an error if platform initialization fails.
    pub fn new() -> Result<Self, error::NetError> {
        platform::init()?;
        Ok(Self)
    }
}

impl Default for NetBackend {
    fn default() -> Self {
        Self::new().expect("platform networking initialization should succeed")
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test {
    use super::*;

    /// Tests that `NetBackend::default()` creates a valid instance.
    #[test]
    #[allow(clippy::default_constructed_unit_structs)]
    fn default_creates_backend() {
        let _backend: NetBackend = NetBackend::default();
    }

    /// Tests that `NetBackend::new()` creates a valid instance.
    #[test]
    fn new_creates_backend() {
        let _backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
    }
}
