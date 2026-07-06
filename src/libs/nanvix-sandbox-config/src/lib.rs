// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox configuration structures for Nanvix.
//!
//! This crate provides the configuration types used by the various sandbox deployment modes:
//! single-process (`SimpleSandboxCacheConfig`) and standalone (`StandaloneConfig`).

//==================================================================================================
// Public Modules
//==================================================================================================

#[cfg(feature = "single-process")]
mod single_process;
mod standalone;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Networking mode for deployments.
///
/// Controls whether networking system calls are available to the guest.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkingMode {
    /// Networking is disabled; networking system calls are blocked.
    Disabled,
    /// Networking is enabled; the network daemon handles socket system calls.
    Enabled,
}

impl NetworkingMode {
    /// Returns whether networking is enabled (any mode other than `Disabled`).
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

impl std::fmt::Display for NetworkingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => write!(f, "disabled"),
            Self::Enabled => write!(f, "enabled"),
        }
    }
}

impl std::str::FromStr for NetworkingMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "disabled" => Ok(Self::Disabled),
            "enabled" => Ok(Self::Enabled),
            other => Err(format!(
                "invalid networking mode: '{other}' (expected 'disabled' or 'enabled')"
            )),
        }
    }
}

// Host egress filtering types live in `net-backend` (the network daemon's
// crate) to keep the enforcement type next to the enforcement point and avoid a
// dependency cycle. They are re-exported here so config consumers can refer to
// them via `nanvix::sandbox_config`.
pub use ::net_backend::{
    HostFilter,
    Ipv4Cidr,
};

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "single-process")]
pub use self::single_process::SimpleSandboxCacheConfig;
pub use self::standalone::StandaloneConfig;
