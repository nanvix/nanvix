// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Name of the host filesystem daemon.
pub const HOSTFSD_NAME: &str = "hostfsd";

/// Name of the process daemon.
pub const PROCD_NAME: &str = "procd";

/// Name of the memory daemon.
pub const MEMD_NAME: &str = "memd";

/// Name of the network daemon.
pub const NETWORKD_NAME: &str = "networkd";

/// Name of the VFS daemon.
pub const VFSD_NAME: &str = "vfsd";

/// Names of the host-side system daemons. These run on the host alongside the user-VM monitor
/// rather than as guest processes, so the guest kernel never spawns them and `procd` never manages
/// them.
pub const HOST_DAEMON_NAMES: &[&str] = &[HOSTFSD_NAME, NETWORKD_NAME];

/// Names of the guest-side system daemons that the kernel spawns directly, including the process
/// daemon (`procd`). The kernel uses these to classify a process's role authoritatively at spawn
/// time, distinguishing a daemon from the init workload even when a deployment omits a daemon and
/// lets the init workload take that daemon's conventional process identifier.
///
/// NOTE: update this list when adding a new guest daemon to the system.
pub const GUEST_DAEMON_NAMES: &[&str] = &[PROCD_NAME, MEMD_NAME, VFSD_NAME];

/// Returns `true` if `name` is a guest-side system daemon that the kernel spawns directly. The
/// kernel uses this to classify a process's role authoritatively at spawn time, distinguishing a
/// daemon from the init workload even in a deployment that omits a daemon and lets the init
/// workload take that daemon's conventional process identifier.
pub fn is_system_daemon(name: &str) -> bool {
    GUEST_DAEMON_NAMES.contains(&name)
}
