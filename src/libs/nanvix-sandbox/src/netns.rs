// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Network namespace allocation and management.
//!
//! This module provides RAII-based allocation of network namespaces for system VM + user VM
//! groups. Each allocated namespace comes with:
//! - A unique namespace name.
//! - A veth pair (host side + namespace side).
//! - A veth IP address pair (host IP + namespace IP).
//! - A per-namespace TCP port allocator for VM gateway ports.
//!
//! Given that the system VM is deployed inside an L2 guest inside the namespace, this module also
//! adds the necessary routing between the root namespace and the L2 guest.
//!
//! Namespaces are returned to the pool when all RAII handles referring to
//! them are dropped. Gateway TCP ports are allocated and freed via the
//! same RAII semantics as `tcp_port.rs`.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::{
        NETNS_NAME_PREFIX,
        VETH_HOST_PREFIX,
        VETH_NS_PREFIX,
    },
    tcp_port::{
        RawTcpPortNum,
        TcpPort,
        TcpPortAllocator,
    },
};
use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    fmt,
    net::Ipv4Addr,
    process::Command,
    sync::{
        Arc,
        Mutex,
        MutexGuard,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Maximum number of namespaces that we can have at any point in time. This determines the
/// maximum number of concurrent system VMs we can have at any point in time.
///
const MAX_NAMESPACES: u32 = 1024;

///
/// # Description
///
/// Base range of IPs that we assign to the VETH pairs.
///
const BASE_VETH_IP: &str = "10.200.0.0";

///
/// # Description
///
/// Mask that we assign to host-side VETH pair.
///
const VETH_IP_MASK: &str = "/31";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A single namespace configuration, owned by the pool.
///
/// This contains all information needed by the gateway and CLH:
/// - Namespace name.
/// - Veth pair names.
/// - Veth host IP / namespace IP.
///
/// TCP ports for gateways are managed separately via a per-namespace `TcpPortAllocator`.
///
#[derive(Clone, Debug)]
pub struct NetnsInfo {
    ns_name: String,
    veth_host_name: String,
    veth_ns_name: String,
    veth_host_ip: Ipv4Addr,
    veth_ns_ip: Ipv4Addr,
}

///
/// # Description
///
/// Internal state for a namespace entry in the pool.
///
struct NetnsEntry {
    info: NetnsInfo,
    refcount: usize,
    gateway_ports: TcpPortAllocator,
}

///
/// # Description
///
/// Initialization strategy for the pool.
///
pub enum NetnsPoolInitStrategy {
    /// Create namespaces lazily on first allocation.
    Lazy,
    /// Pre-create `count` namespaces at construction time.
    Prefill(usize),
}

///
/// # Description
///
/// Pool configuration parameters.
///
/// IP strategy:
///     For namespace id=N:
///         - veth_host_ip = base_veth_ip + 2*N
///         - veth_ns_ip   = base_veth_ip + 2*N + 1
///
/// Gateway port strategy:
///     Each namespace has its own `TcpPortAllocator` over the range [gateway_port_begin,
///     gateway_port_end]. Ports are unique *within* a namespace; ranges may overlap across
///     namespaces (which is safe because TCP ports are namespaced by netns).
///
#[derive(Clone)]
pub struct NetnsPoolConfig {
    ns_name_prefix: String,
    veth_host_prefix: String,
    veth_ns_prefix: String,
    base_veth_ip: Ipv4Addr,
    gateway_port_begin: RawTcpPortNum,
    gateway_port_end: RawTcpPortNum,
    max_namespaces: u32,
}

///
/// # Description
///
/// Inner pool state guarded by a mutex.
///
struct NetnsPoolState {
    entries: Vec<NetnsEntry>,
    free_indices: Vec<usize>,
    next_id: u32,
}

///
/// # Description
///
/// Cloneable inner handle to the pool.
///
#[derive(Clone)]
struct NetnsPoolInner {
    state: Arc<Mutex<NetnsPoolState>>,
    config: NetnsPoolConfig,
}

///
/// # Description
///
/// RAII handle to a leased network namespace.
///
/// This handle can be cloned (e.g., one handle for the system VM, one for each user VM). When the
/// last handle is dropped, the namespace is returned to the pool for reuse.
///
/// It also provides a per-namespace gateway TCP port allocator via
/// `allocate_gateway_port()`.
///
pub struct NetnsHandle {
    index: usize,
    pool: NetnsPoolInner,
}

///
/// # Description
///
/// Public wrapper around the namespace pool.
///
pub struct NetnsPool {
    inner: NetnsPoolInner,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl NetnsInfo {
    ///
    /// # Description
    ///
    /// Returns the namespace name.
    ///
    /// # Returns
    ///
    /// The namespace name.
    ///
    pub fn ns_name(&self) -> &str {
        &self.ns_name
    }

    ///
    /// # Description
    ///
    /// Returns the host-side veth interface name.
    ///
    /// # Returns
    ///
    /// The host-side veth interface name.
    ///
    pub(crate) fn veth_host_name(&self) -> &str {
        &self.veth_host_name
    }

    ///
    /// # Description
    ///
    /// Returns the namespace-side veth interface name.
    ///
    /// # Returns
    ///
    /// The namespace-side veth interface name.
    ///
    pub(crate) fn veth_ns_name(&self) -> &str {
        &self.veth_ns_name
    }

    ///
    /// # Description
    ///
    /// Returns the host-side veth IP address.
    ///
    /// # Returns
    ///
    /// The host-side veth IP address.
    ///
    pub fn veth_host_ip(&self) -> Ipv4Addr {
        self.veth_host_ip
    }

    ///
    /// # Description
    ///
    /// Returns the namespace-side veth IP address.
    ///
    /// # Returns
    ///
    /// The namespace-side veth IP address.
    ///
    pub fn veth_ns_ip(&self) -> Ipv4Addr {
        self.veth_ns_ip
    }
}

impl NetnsPoolConfig {
    ///
    /// # Description
    ///
    /// Creates a new network namespace pool configuration.
    ///
    /// # Parameters
    ///
    /// - `gateway_port_begin`: The beginning of the gateway TCP port range.
    /// - `gateway_port_end`: The end of the gateway TCP port range.
    ///
    /// # Returns
    ///
    /// A new network namespace pool configuration, or an error if initialization fails.
    ///
    pub fn new(gateway_port_begin: u16, gateway_port_end: u16) -> Result<Self> {
        let base_veth_ip: Ipv4Addr = match BASE_VETH_IP.parse() {
            Ok(ip) => ip,
            Err(error) => {
                let reason: String = format!(
                    "new(): failed to parse BASE_VETH_IP constant (value={}, error={error:?})",
                    BASE_VETH_IP
                );
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };

        Ok(NetnsPoolConfig {
            ns_name_prefix: NETNS_NAME_PREFIX.into(),
            veth_host_prefix: VETH_HOST_PREFIX.into(),
            veth_ns_prefix: VETH_NS_PREFIX.into(),
            base_veth_ip,
            gateway_port_begin,
            gateway_port_end,
            max_namespaces: MAX_NAMESPACES,
        })
    }
}

impl fmt::Debug for NetnsPoolConfig {
    ///
    /// # Description
    ///
    /// Formats the network namespace pool configuration for debugging.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter.
    ///
    /// # Returns
    ///
    /// The result of the formatting operation.
    ///
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetnsPoolConfig")
            .field("ns_name_prefix", &self.ns_name_prefix)
            .field("veth_host_prefix", &self.veth_host_prefix)
            .field("veth_ns_prefix", &self.veth_ns_prefix)
            .field("base_veth_ip", &self.base_veth_ip)
            .field("gateway_port_begin", &self.gateway_port_begin)
            .field("gateway_port_end", &self.gateway_port_end)
            .field("max_namespaces", &self.max_namespaces)
            .finish()
    }
}

impl fmt::Debug for NetnsPoolInitStrategy {
    ///
    /// # Description
    ///
    /// Formats the network namespace pool initialization strategy for debugging.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter.
    ///
    /// # Returns
    ///
    /// The result of the formatting operation.
    ///
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetnsPoolInitStrategy::Lazy => write!(f, "lazy"),
            NetnsPoolInitStrategy::Prefill(size) => write!(f, "prefill-{size}"),
        }
    }
}

impl NetnsHandle {
    ///
    /// # Description
    ///
    /// Returns information about the leased namespace.
    ///
    /// # Returns
    ///
    /// The information about the leased network namespace, or an error otherwise.
    ///
    pub fn netns_info(&self) -> Result<NetnsInfo> {
        match self.pool.state.lock() {
            Ok(guard) => Ok(guard.entries[self.index].info.clone()),
            Err(error) => {
                let reason: String =
                    format!("netns_info(): failed to acquire lock (error={error:?})");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Allocates a gateway TCP port within this namespace.
    ///
    /// The returned `TcpPort` is RAII-managed and will be automatically returned to the
    /// per-namespace pool when dropped. Ports are unique *within* this namespace; different
    /// namespaces may reuse the same port numbers on different veth IPs.
    ///
    /// # Returns
    ///
    /// A TCP port handle, or an error if no ports are available.
    ///
    pub fn allocate_gateway_port(&self) -> Result<TcpPort> {
        match self.pool.state.lock() {
            Ok(mut guard) => {
                if let Some(entry) = guard.entries.get_mut(self.index) {
                    if let Some(port) = entry.gateway_ports.allocate() {
                        Ok(port)
                    } else {
                        let reason: String =
                            "allocate_gateway_port(): ran out of TCP ports".to_string();
                        error!("{reason}");
                        anyhow::bail!(reason);
                    }
                } else {
                    let reason: String =
                        format!("allocate_gateway_port(): invalid index (index={})", self.index);
                    error!("{reason}");
                    anyhow::bail!(reason);
                }
            },
            Err(error) => {
                let reason: String =
                    format!("allocate_gateway_port(): failed to acquire lock (error={error:?})");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        }
    }
}

impl fmt::Debug for NetnsHandle {
    ///
    /// # Description
    ///
    /// Formats the network namespace handle for debugging.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter.
    ///
    /// # Returns
    ///
    /// The result of the formatting operation.
    ///
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.netns_info() {
            Ok(info) => {
                write!(
                    f,
                    "NetnsHandle(ns={}, veth_host={}, veth_ns={}, host_ip={}, ns_ip={})",
                    info.ns_name(),
                    info.veth_host_name(),
                    info.veth_ns_name(),
                    info.veth_host_ip(),
                    info.veth_ns_ip(),
                )
            },
            Err(e) => {
                error!("error getting netns info (error={e:?})");
                write!(f, "NetnsHandle(broken)")
            },
        }
    }
}

impl Clone for NetnsHandle {
    ///
    /// # Description
    ///
    /// Clones the network namespace handle, incrementing the reference count.
    ///
    /// # Returns
    ///
    /// A new handle to the same network namespace.
    ///
    fn clone(&self) -> Self {
        // This is likely a fatal error, but we cannot change the signature of clone.
        if let Err(e) = self.pool.inc_ref(self.index) {
            error!("error incrementing pool ref, likely fatal (index={}, error={e:?})", self.index);
        };

        Self {
            index: self.index,
            pool: self.pool.clone(),
        }
    }
}

impl Drop for NetnsHandle {
    ///
    /// # Description
    ///
    /// Drops the network namespace handle, decrementing the reference count.
    /// When the last handle is dropped, the namespace is returned to the pool.
    ///
    fn drop(&mut self) {
        trace!("drop(): dropping network namespace handle (ns_handle={:?})", self);
        if let Err(e) = self.pool.dec_ref(self.index) {
            error!("error decreasing reference for netns (idx={}, error={e:?})", self.index);
        }
    }
}

impl NetnsPool {
    ///
    /// # Description
    ///
    /// Creates a new namespace pool with the given configuration and initialization strategy:
    /// - In `Lazy` mode, namespaces are created on-demand on first use.
    /// - In `Prefill(count)` mode, `count` namespaces are created up front.
    ///
    /// # Parameters
    ///
    /// - `config`: The network namespace pool configuration.
    /// - `strategy`: The network namespace pool initialization strategy.
    ///
    /// # Returns
    ///
    /// A new network namespace pool, or an error if initialization fails.
    ///
    pub fn new(config: NetnsPoolConfig, strategy: NetnsPoolInitStrategy) -> Result<Self> {
        trace!("new(): creating new netns pool (config={config:?}, strategy={strategy:?})");

        // Initialize the inner pool state.
        let inner: NetnsPoolInner = NetnsPoolInner {
            state: Arc::new(Mutex::new(NetnsPoolState {
                entries: Vec::new(),
                free_indices: Vec::new(),
                next_id: 0,
            })),
            config,
        };

        // Pre-initialize some namespaces if necessary.
        if let NetnsPoolInitStrategy::Prefill(count) = strategy {
            // MAX_NAMESPACES is a u32, so we can safely cast to usize.
            if count > (MAX_NAMESPACES as usize) {
                let reason: String = format!(
                    "requested prefilled netns pool size larger than max (req={count}, \
                     max={MAX_NAMESPACES})"
                );
                error!("new(): {reason}");
                anyhow::bail!(reason);
            }

            // Allocate all namespaces upfront but keep the handles to make sure we create distinct
            // namespaces without reuse.
            let mut handles: Vec<NetnsHandle> = Vec::new();
            for _ in 0..count {
                let handle: NetnsHandle = inner.allocate_impl()?;
                handles.push(handle);
            }

            // Explicitly drop all handles to make sure we return the namespaces to the pool.
            for handle in handles.drain(..) {
                drop(handle);
            }
        }

        Ok(Self { inner })
    }

    ///
    /// # Description
    ///
    /// Allocates a namespace and returns a RAII handle.
    ///
    /// # Returns
    ///
    /// A handle to a network namespace, or an error if allocation fails.
    ///
    pub fn allocate(&self) -> Result<NetnsHandle> {
        self.inner.allocate_impl()
    }
}

impl Drop for NetnsPool {
    ///
    /// # Description
    ///
    /// Drops the network namespace pool, deleting all allocated namespaces.
    ///
    fn drop(&mut self) {
        trace!("drop(): deleting network namespace pool");
        match self.inner.state.lock() {
            Ok(guard) => {
                for entry in &guard.entries {
                    delete_namespace(&entry.info);
                }
            },
            Err(error) => {
                error!("drop(): failed to acquire lock during pool teardown (error={error:?})");
            },
        }
    }
}

impl NetnsPoolInner {
    ///
    /// # Description
    ///
    /// Allocate a new netns handle from the netns pool.
    ///
    /// We first try to use a netns that has been initialized, used, and freed before. If there
    /// are none, we allocate a new one. If we have ran out of slots, or we encounter an error, we
    /// return an error.
    ///
    /// # Returns
    ///
    /// A handle to a network namespace if successful, an error otherwise.
    ///
    fn allocate_impl(&self) -> Result<NetnsHandle> {
        let mut state: MutexGuard<'_, NetnsPoolState> = match self.state.lock() {
            Ok(guard) => guard,
            Err(error) => {
                let reason: String =
                    format!("allocate_impl(): failed to acquire lock (error={error:?})");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };

        // Reuse a free namespace slot if available.
        if let Some(idx) = state.free_indices.pop() {
            let entry: &mut NetnsEntry = &mut state.entries[idx];
            entry.refcount = 1;

            // Reset gateway ports by recreating the allocator.
            entry.gateway_ports =
                TcpPortAllocator::new(self.config.gateway_port_begin, self.config.gateway_port_end);

            drop(state);
            return Ok(NetnsHandle {
                index: idx,
                pool: self.clone(),
            });
        }

        // Create a new namespace if we haven't hit the max.
        if state.next_id >= self.config.max_namespaces {
            let reason: String = format!(
                "allocate_impl(): max_namespaces reached (max_ns={})",
                self.config.max_namespaces
            );
            error!("{reason}");
            anyhow::bail!(reason);
        }

        // This id represents the "slot" of the network namespace pool we are taking up.
        let id: u32 = state.next_id;
        state.next_id += 1;

        let info: NetnsInfo = self.build_info_for_id(id)?;
        init_namespace(&info).map_err(|e| {
            let reason: String = format!(
                "allocate_impl(): failed to initialize namespace (ns_name={}, error={e:?})",
                info.ns_name()
            );
            error!("{reason}");
            anyhow::anyhow!(reason)
        })?;

        let entry: NetnsEntry = NetnsEntry {
            info,
            refcount: 1,
            gateway_ports: TcpPortAllocator::new(
                self.config.gateway_port_begin,
                self.config.gateway_port_end,
            ),
        };

        state.entries.push(entry);
        // This index represents the offset of the network namespace in the initialized network
        // namespace array. Given that we reuse network namespaces and initialize them (by default)
        // lazily, this index may not match the slot in the pool.
        let idx: usize = state.entries.len() - 1;
        drop(state);

        Ok(NetnsHandle {
            index: idx,
            pool: self.clone(),
        })
    }

    ///
    /// # Description
    ///
    /// Increments the reference count on the network namespace handle.
    ///
    /// # Parameters
    ///
    /// - `index`: The offset of the netns handle in the netns pool.
    ///
    /// # Returns
    ///
    /// Ok on success, or an error if the operation fails.
    ///
    fn inc_ref(&self, index: usize) -> Result<()> {
        match self.state.lock() {
            Ok(mut guard) => {
                if let Some(entry) = guard.entries.get_mut(index) {
                    entry.refcount += 1;
                    Ok(())
                } else {
                    let reason: String = format!("inc_ref(): invalid index (idx={index})");
                    error!("{reason}");
                    anyhow::bail!(reason);
                }
            },
            Err(error) => {
                let reason: String = format!("inc_ref(): failed to acquire lock (error={error:?})");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Decrements the reference count on the network namespace handle.
    /// When the reference count reaches zero, the namespace is returned to the pool.
    ///
    /// # Parameters
    ///
    /// - `index`: The offset of the netns handle in the netns pool.
    ///
    /// # Returns
    ///
    /// Ok on success, or an error if the operation fails.
    ///
    fn dec_ref(&self, index: usize) -> Result<()> {
        match self.state.lock() {
            Ok(mut guard) => {
                if let Some(entry) = guard.entries.get_mut(index) {
                    // Not a fatal error, but log warning.
                    if entry.refcount == 0 {
                        warn!("dec_ref(): refcount already zero (ns_name={})", entry.info.ns_name);

                        return Ok(());
                    }
                    entry.refcount -= 1;

                    // If no more references, return netns to the pool.
                    if entry.refcount == 0 {
                        // Cleanup namespace before reuse (may no-op), not a fatal error.
                        trace!(
                            "dec_ref(): returning network namespace to the pool (ns_handle={:?})",
                            entry.info
                        );
                        if let Err(error) = cleanup_namespace(&entry.info) {
                            warn!(
                                "dec_ref(): cleanup failed (ns_name={}, error={error:?})",
                                entry.info.ns_name
                            );
                        }
                        guard.free_indices.push(index);
                    }

                    Ok(())
                } else {
                    let reason: String = format!("dec_ref(): invalid index (idx={index})");
                    error!("{reason}");
                    anyhow::bail!(reason);
                }
            },
            Err(error) => {
                let reason: String = format!("dec_ref(): failed to acquire lock (error={error:?})");
                error!("{reason}");
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Builds NetnsInfo for a given numeric ID.
    ///
    /// # Parameters
    ///
    /// - `id`: The numeric ID of the network namespace.
    ///
    /// # Returns
    ///
    /// A structure describing the network namespace, or an error if the operation fails.
    ///
    fn build_info_for_id(&self, id: u32) -> Result<NetnsInfo> {
        let base_ip_u32: u32 = u32::from(self.config.base_veth_ip);
        let host_ip: Ipv4Addr = Ipv4Addr::from(
            base_ip_u32
                .checked_add(2 * id)
                .ok_or_else(|| anyhow::anyhow!("IP address calculation overflow"))?,
        );
        let ns_ip: Ipv4Addr = Ipv4Addr::from(
            base_ip_u32
                .checked_add(2 * id + 1)
                .ok_or_else(|| anyhow::anyhow!("IP address calculation overflow"))?,
        );

        Ok(NetnsInfo {
            ns_name: format!("{}{}", self.config.ns_name_prefix, id),
            veth_host_name: format!("{}{}", self.config.veth_host_prefix, id),
            veth_ns_name: format!("{}{}", self.config.veth_ns_prefix, id),
            veth_host_ip: host_ip,
            veth_ns_ip: ns_ip,
        })
    }
}

//==================================================================================================
// OS-level namespace/veth management (stubs)
//
// FIXME (#1171): implement these stubs with low-level libc calls + CAP_NET_ADMIN to avoid having to
// call `sudo` on the critical path, as it is known to add upwards of 10 ms of latency.
//==================================================================================================

///
/// # Description
///
/// Runs a system command and returns an error if it fails.
///
/// # Parameters
///
/// - `cmd`: The command to run.
///
/// # Returns
///
/// Ok on success, or an error if the command fails.
///
fn run_cmd(cmd: &mut Command) -> Result<()> {
    let desc: String = format!("{cmd:?}");
    match cmd.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => {
            let reason: String = format!(
                "run_command(): command failed with exist status (cmd=`{desc}`, status={status})"
            );
            error!("{reason}");
            anyhow::bail!(reason);
        },
        Err(e) => {
            let reason: String =
                format!("run_command(): failed to run command (cmd=`{desc}`, error={e:?})");
            error!("{reason}");
            anyhow::bail!(reason);
        },
    }
}

///
/// # Description
///
/// Initializes a new network namespace. The steps are as follows:
///     1. Create the network namespace.
///     2. Create the veth pair in the root namespace.
///     3. Move the namespace side of the veth into the netns.
///     4. Assign IP to host side, bring it up.
///     5. Inside the netns: assign IP to veth_ns, bring veth_ns and lo up.
///
/// Once we are done setting up the namespace, we also need to make sure that all traffic hitting
/// the VETH pair is routed to the L2 VM. This involves:
///     6. Enable IP forwarding inside the netns.
///     7. Route all packets from the host to the VETH halve inside the netns to the TAP IP address
///        of the L2 guest with DNAT.
///     8. Reverse of 7. Route all packets from guest to TAP's host IP to the root halve of the
///        VETH pair.
///
/// # Parameters
///
/// - `info`: The network namespace info.
///
/// # Returns
///
/// Ok on success, or an error if initialization fails.
///
fn init_namespace(info: &NetnsInfo) -> Result<()> {
    trace!("init_namespace(): creating namespace with info: {info:?}");

    // 1. Create the network namespace.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("add")
            .arg(&info.ns_name),
    )?;

    // 2. Create the veth pair in the root namespace.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("add")
            .arg(&info.veth_host_name)
            .arg("type")
            .arg("veth")
            .arg("peer")
            .arg("name")
            .arg(&info.veth_ns_name),
    )?;

    // 3. Move the namespace side of the veth into the netns.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("set")
            .arg(&info.veth_ns_name)
            .arg("netns")
            .arg(&info.ns_name),
    )?;

    // 4. Assign IP to host side, bring it up.
    let host_cidr: String = format!("{}{VETH_IP_MASK}", info.veth_host_ip);
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("addr")
            .arg("add")
            .arg(&host_cidr)
            .arg("dev")
            .arg(&info.veth_host_name),
    )?;
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("set")
            .arg(&info.veth_host_name)
            .arg("up"),
    )?;

    // 5. Inside the netns: assign IP to veth_ns, bring veth_ns and lo up.
    let ns_cidr: String = format!("{}{VETH_IP_MASK}", info.veth_ns_ip);
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(&info.ns_name)
            .arg("ip")
            .arg("addr")
            .arg("add")
            .arg(&ns_cidr)
            .arg("dev")
            .arg(&info.veth_ns_name),
    )?;

    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(&info.ns_name)
            .arg("ip")
            .arg("link")
            .arg("set")
            .arg(&info.veth_ns_name)
            .arg("up"),
    )?;

    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(&info.ns_name)
            .arg("ip")
            .arg("link")
            .arg("set")
            .arg("lo")
            .arg("up"),
    )?;

    // 6. Enable IP forwarding in namespace.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(info.ns_name())
            .arg("sysctl")
            .arg("-q")
            .arg("-w")
            .arg("net.ipv4.ip_forward=1"),
    )?;

    // 7. Forward traffic from the host to the NS VETH pair to the guest TAP IP in the L2 VM by
    //    replacing the destination IP address.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(info.ns_name())
            .arg("iptables")
            .arg("-t")
            .arg("nat")
            .arg("-A")
            .arg("PREROUTING")
            .arg("-d")
            .arg(format!("{}", info.veth_ns_ip()))
            .arg("-p")
            .arg("tcp")
            .arg("-j")
            .arg("DNAT")
            .arg("--to-destination")
            .arg(::config::linuxd::GUEST_TAP_IP_ADDRESS),
    )?;

    // To prevent conntrack confusion, add a MASQUERADE rule that changes the source IP,
    // post-routing, to the IP of the TAP device.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(info.ns_name())
            .arg("iptables")
            .arg("-t")
            .arg("nat")
            .arg("-A")
            .arg("POSTROUTING")
            .arg("-o")
            .arg(::config::linuxd::TAP_NAME)
            .arg("-p")
            .arg("tcp")
            .arg("-d")
            .arg(::config::linuxd::GUEST_TAP_IP_ADDRESS)
            .arg("-j")
            .arg("MASQUERADE"),
    )?;

    // 8. Reverse operation to 7. Forward all traffic from the L2 VM to the host-side of the TAP
    //    device, to the root namespace side of the VETH pair. We do so by replacing the
    //    destination IP address.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(info.ns_name())
            .arg("iptables")
            .arg("-t")
            .arg("nat")
            .arg("-A")
            .arg("PREROUTING")
            .arg("-i")
            .arg(::config::linuxd::TAP_NAME)
            .arg("-p")
            .arg("tcp")
            .arg("-j")
            .arg("DNAT")
            .arg("--to-destination")
            .arg(format!("{}", info.veth_host_ip())),
    )?;

    // To prevent conntrack confusion, add a MASQUERADE rule that changes the source IP,
    // post-routing, from the IP of the TAP device to the netns-side of the VETH pair.
    run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("exec")
            .arg(info.ns_name())
            .arg("iptables")
            .arg("-t")
            .arg("nat")
            .arg("-A")
            .arg("POSTROUTING")
            .arg("-o")
            .arg(info.veth_ns_name())
            .arg("-p")
            .arg("tcp")
            .arg("-d")
            .arg(format!("{}", info.veth_host_ip()))
            .arg("-j")
            .arg("MASQUERADE"),
    )?;

    Ok(())
}

///
///
/// # Description
///
/// Helper method to clean-up a namespace after it has been used by a tenant and get it ready for a
/// new tenant.
///
/// # Parameters
///
/// - `_info`: The network namespace info (currently unused).
///
/// # Returns
///
/// Ok on success, or an error if cleanup fails.
///
fn cleanup_namespace(_info: &NetnsInfo) -> Result<(), String> {
    // Optional: flush routes, conntrack, etc. Currently no-op.

    Ok(())
}

///
/// # Description
///
/// Deletes a network namespace. The clean-up steps are:
///     1. Delete the host-side VETH pair.
///     2. Delete the netns itself.
///
/// Deletion is a best-effort operation. If it fails we just log an error.
///
/// # Parameters
///
/// - `info`: The network namespace info.
///
fn delete_namespace(info: &NetnsInfo) {
    // Best-effort: delete the host-side veth if it still exists.
    if let Err(e) = run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("del")
            .arg(&info.veth_host_name),
    ) {
        error!(
            "delete_namespace(): failed to delete veth (name={}, error={e:?})",
            info.veth_host_name
        );
    }

    // Delete the network namespace itself.
    if let Err(e) = run_cmd(
        Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("del")
            .arg(&info.ns_name),
    ) {
        error!("delete_namespace(): failed to delete netns (name={}, error={e})", info.ns_name);
    }
}
