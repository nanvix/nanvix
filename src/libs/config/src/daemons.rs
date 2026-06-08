// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Name of the host filesystem daemon.
pub const HOSTFSD_NAME: &str = "hostfsd";

/// Name of the memory daemon.
pub const MEMD_NAME: &str = "memd";

/// Name of the network daemon.
pub const NETWORKD_NAME: &str = "networkd";

/// Name of the VFS daemon.
pub const VFSD_NAME: &str = "vfsd";

/// Names of system daemons that should not trigger shutdown when they terminate.
/// The process daemon itself ("procd") is excluded because it is never registered in the
/// process table (procd is the registrar, not a registrant).
///
/// NOTE: update this list when adding a new guest daemon to the system.
pub const DAEMON_NAMES: &[&str] = &[MEMD_NAME, NETWORKD_NAME, VFSD_NAME];
