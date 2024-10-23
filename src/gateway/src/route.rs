// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};
use ::sys::{
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::tokio::sync::{
    mpsc::UnboundedSender,
    RwLock,
    RwLockReadGuard,
    RwLockWriteGuard,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// Gateway Lookup Table
///
#[derive(Clone)]
pub struct GatewayLookupTable {
    /// Clients connected to the gateway.
    clients: Arc<RwLock<LookupTable>>,
    /// Hash table that resolves client addresses to process identifiers.
    pids_to_clients: Arc<RwLock<HashMap<u32, SocketAddr>>>,
}

///
/// Gateway Peer
///
#[derive(Clone)]
pub enum GatewayPeer {
    Client(UnboundedSender<Result<Message, anyhow::Error>>),
}

//==================================================================================================
// Implementations
//==================================================================================================

// Type aliases to make clippy happy.
type LookupTable = HashMap<SocketAddr, GatewayPeer>;
type PidTable = HashMap<u32, SocketAddr>;
type LookupTableReadGuard<'a> = RwLockReadGuard<'a, LookupTable>;
type LookupTableWriteGuard<'a> = RwLockWriteGuard<'a, LookupTable>;
type PidTableReadGuard<'a> = RwLockReadGuard<'a, PidTable>;
type PidTableWriteGuard<'a> = RwLockWriteGuard<'a, PidTable>;

impl GatewayLookupTable {
    ///
    /// # Description
    ///
    /// Creates a new gateway lookup tables.
    ///
    /// # Returns
    ///
    /// A new gateway lookup tables.
    ///
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            pids_to_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    ///
    /// # Description
    ///
    /// Registers a client address.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address of the client.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    pub async fn register_addr(&mut self, addr: SocketAddr, client_tx: GatewayPeer) -> Result<()> {
        trace!("register_addr(): addr={:?}", addr);

        // Lock tables.
        let (_pids_to_clients, mut clients): (PidTableWriteGuard<'_>, LookupTableWriteGuard<'_>) =
            Self::lock_tables_write(&self.pids_to_clients, &self.clients).await;

        // Check if address is already in use.
        if clients.contains_key(&addr) {
            let reason: String = format!("address already in use (addr={:?})", addr);
            error!("register_addr(): {}", reason);
            anyhow::bail!(reason);
        }

        // Register client in lookup tables.
        clients.insert(addr, client_tx);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Registers a process identifier.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    pub async fn register_pid(&mut self, pid: ProcessIdentifier, addr: SocketAddr) -> Result<()> {
        trace!("register_pid(): pid={:?}, addr={:?}", pid, addr);

        // Lock tables.
        let (mut pids_to_clients, clients): (PidTableWriteGuard<'_>, LookupTableWriteGuard<'_>) =
            Self::lock_tables_write(&self.pids_to_clients, &self.clients).await;

        // Check if address is not registered.
        if !clients.contains_key(&addr) {
            let reason: String = format!("address is not registered (addr={:?})", addr);
            error!("register_pid(): {}", reason);
            anyhow::bail!(reason);
        }

        // Check if process identifier is already registered.
        if let Some(registerd_addr) = pids_to_clients.get(&pid.into()) {
            // Check if address does not match the expected one.
            if *registerd_addr != addr {
                let reason: String = format!(
                    "pid already registered (pid={:?}, addr={:?}, registered_addr={:?})",
                    pid, addr, registerd_addr
                );
                error!("register_pid(): {}", reason);
                anyhow::bail!(reason);
            }

            // Process identifier is already registered.
            return Ok(());
        }

        // Register client in lookup tables.
        pids_to_clients.insert(pid.into(), addr);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Removes a client.
    ///
    /// # Parameters
    ///
    /// - `pids_to_clients`: Process identifiers to clients lookup table.
    ///
    /// - `clients`: Clients lookup table.
    ///
    /// - `addr`: Address of the client.
    ///
    pub async fn remove(lookup_tables: &GatewayLookupTable, addr: SocketAddr) -> Result<()> {
        trace!("remove(): addr={:?}", addr);

        // Lock tables.
        let (mut pids_to_clients, mut clients): (
            PidTableWriteGuard<'_>,
            LookupTableWriteGuard<'_>,
        ) = Self::lock_tables_write(&lookup_tables.pids_to_clients, &lookup_tables.clients).await;

        // Remove address from lookup table.
        if clients.remove(&addr).is_none() {
            let reason: String = format!("unknown address (addr={:?})", addr);
            unreachable!("remove(): {}", reason);
        }

        // Remove process identifier from lookup table.
        if let Some(pid) = pids_to_clients.iter().find_map(|(query_pid, query_addr)| {
            if *query_addr == addr {
                Some(*query_pid)
            } else {
                None
            }
        }) {
            pids_to_clients.remove(&pid);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Looks up a client based on its process identifier.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the client.
    ///
    /// # Returns
    ///
    /// The client associated with the process identifier.
    ///
    pub async fn lookup_pid(&self, pid: ProcessIdentifier) -> Result<GatewayPeer> {
        trace!("lookup(): pid={:?}", pid);

        // Lock tables for reading.
        let (pids_to_clients, clients): (PidTableReadGuard<'_>, LookupTableReadGuard<'_>) =
            Self::lock_tables_read(&self.pids_to_clients, &self.clients).await;

        // Lookup process identifier.
        match pids_to_clients.get(&pid.into()) {
            // Found matching address.
            Some(addr) => {
                // Lookup address.
                match clients.get(addr) {
                    // Found matching address.
                    Some(client) => Ok(client.clone()),
                    // Address not found.
                    None => {
                        let reason: String =
                            format!("unknown address (pid={:?}, addr={:?})", pid, addr);
                        error!("lookup(): {}", reason);
                        anyhow::bail!(reason);
                    },
                }
            },
            // Process identifier not found.
            None => {
                let reason: String = format!("unknown pid (pid={:?})", pid);
                error!("lookup(): {}", reason);
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Looks up a client based on its address.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address of the client.
    ///
    /// # Returns
    ///
    /// The client associated with the address.
    ///
    pub async fn lookup_addr(&self, addr: SocketAddr) -> Result<GatewayPeer> {
        trace!("lookup_addr(): addr={:?}", addr);

        // Lock tables for reading.
        let (_pids_to_clients, clients): (PidTableReadGuard<'_>, LookupTableReadGuard<'_>) =
            Self::lock_tables_read(&self.pids_to_clients, &self.clients).await;

        // Lookup address.
        match clients.get(&addr) {
            // Found matching address.
            Some(client) => Ok(client.clone()),
            // Address not found.
            None => {
                let reason: String = format!("unknown address (addr={:?})", addr);
                error!("lookup_addr(): {}", reason);
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Helper function that locks lookup tables for reading.
    ///
    /// # Parameters
    ///
    /// - `pids_to_clients`: Process identifiers to clients lookup table.
    /// - `clients`: Clients lookup table.
    ///
    /// # Returns
    ///
    /// A tuple containing the read guards for the lookup tables.
    ///
    async fn lock_tables_read<'a>(
        pids_to_clients: &'a Arc<RwLock<HashMap<u32, SocketAddr>>>,
        clients: &'a Arc<RwLock<LookupTable>>,
    ) -> (RwLockReadGuard<'a, HashMap<u32, SocketAddr>>, RwLockReadGuard<'a, LookupTable>) {
        let pids_to_clients: RwLockReadGuard<'a, HashMap<u32, SocketAddr>> =
            pids_to_clients.read().await;
        let clients: RwLockReadGuard<'a, LookupTable> = clients.read().await;
        (pids_to_clients, clients)
    }

    ///
    /// # Description
    ///
    /// Helper function that locks lookup tables for writing.
    ///
    /// # Parameters
    ///
    /// - `pids_to_clients`: Process identifiers to clients lookup table.
    /// - `clients`: Clients lookup table.
    ///
    /// # Returns
    ///
    /// A tuple containing the write guards for the lookup tables.
    ///
    async fn lock_tables_write<'a>(
        pids_to_clients: &'a Arc<RwLock<HashMap<u32, SocketAddr>>>,
        clients: &'a Arc<RwLock<LookupTable>>,
    ) -> (RwLockWriteGuard<'a, HashMap<u32, SocketAddr>>, RwLockWriteGuard<'a, LookupTable>) {
        let pids_to_clients: RwLockWriteGuard<'a, HashMap<u32, SocketAddr>> =
            pids_to_clients.write().await;
        let clients: RwLockWriteGuard<'a, LookupTable> = clients.write().await;
        (pids_to_clients, clients)
    }
}
