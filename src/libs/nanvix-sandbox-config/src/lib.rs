// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox configuration structures for Nanvix.
//!
//! This crate provides the configuration types used by the various sandbox deployment modes:
//! multi-process (`SandboxCacheConfig`), single-process (`SimpleSandboxCacheConfig`), and
//! standalone (`StandaloneConfig`).

//==================================================================================================
// Public Modules
//==================================================================================================

mod multi_process;
#[cfg(feature = "single-process")]
mod single_process;
mod standalone;

//==================================================================================================
// Imports
//==================================================================================================

use ::syscomm::SocketType;

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

///
/// # Description
///
/// Endpoint of a decoupled `networkd` process.
///
/// When set on a [`StandaloneConfig`], the user VM forwards its socket system calls to an external
/// `networkd` process listening at this address instead of running the network daemon in-process.
/// This keeps `networkd`'s state fully separate from the user VM it serves, so `networkd` can
/// eventually run on a different machine.
///
#[derive(Clone, Debug)]
pub struct NetworkdEndpoint {
    /// Socket address `networkd` is listening on (a Unix-domain socket path or a `host:port` pair).
    sockaddr: String,
    /// Socket address family used to reach `networkd`.
    socket_type: SocketType,
}

impl NetworkdEndpoint {
    ///
    /// # Description
    ///
    /// Creates a new decoupled `networkd` endpoint.
    ///
    /// # Parameters
    ///
    /// - `sockaddr`: Socket address `networkd` is listening on.
    /// - `socket_type`: Socket address family used to reach `networkd`.
    ///
    pub fn new(sockaddr: String, socket_type: SocketType) -> Self {
        Self {
            sockaddr,
            socket_type,
        }
    }

    /// Returns the socket address `networkd` is listening on.
    pub fn sockaddr(&self) -> &str {
        &self.sockaddr
    }

    /// Returns the socket address family used to reach `networkd`.
    pub fn socket_type(&self) -> SocketType {
        self.socket_type
    }
}

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "single-process")]
pub use self::single_process::SimpleSandboxCacheConfig;
pub use self::{
    multi_process::SandboxCacheConfig,
    standalone::StandaloneConfig,
};
