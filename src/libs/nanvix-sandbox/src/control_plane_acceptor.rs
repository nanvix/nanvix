// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Shared control-plane acceptor and connection router.
//!
//! This module implements a daemon task that owns the control-plane listener socket and accepts
//! incoming connections from new linuxd and uservm instances. It then notifies the tasks that
//! spawned them and are blocked waiting for the spawn to be complete and the instance to be ready.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config::{
    CONTROL_PLANE_ACCEPT_TIMEOUT,
    MAX_CONCURRENT_CONTROL_PLANE_HANDLERS,
    MAX_EARLY_CONTROL_PLANE_CONNECTIONS,
};
use ::anyhow::Result;
use ::control_plane_api::ControlPlaneRegistrationMessage;
use ::indexmap::IndexMap;
use ::log::{
    debug,
    error,
    warn,
};
use ::std::{
    collections::HashMap,
    sync::{
        Arc,
        Weak,
    },
};
use ::syscomm::{
    ReadExact,
    SocketListener,
    SocketStream,
    SocketType,
};
use ::tokio::{
    spawn,
    sync::{
        oneshot::{
            channel,
            Receiver,
            Sender,
        },
        Mutex,
        MutexGuard,
        Semaphore,
    },
    task::{
        AbortHandle,
        JoinHandle,
    },
    time::timeout,
};
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Logical identity of a peer connecting to the shared control-plane listener.
///
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum ControlPlanePeer {
    LinuxDaemon(String),
    UserVm(UserVmIdentifier),
}

///
/// # Description
///
/// Internal routing state for the control-plane acceptor.
///
#[derive(Default)]
struct ControlPlaneAcceptorState {
    /// Registered peers pending to connect to the sandbox cache.
    pending: HashMap<ControlPlanePeer, Sender<SocketStream>>,
    /// Peers that arrived before a request to register them (insertion-ordered for FIFO eviction).
    early: IndexMap<ControlPlanePeer, SocketStream>,
}

///
/// # Description
///
/// Background acceptor that owns control-plane accepts and routes streams to waiting peers.
///
pub struct ControlPlaneAcceptor {
    listener: SocketListener,
    _listener_sockaddr: String,
    _listener_socket_type: SocketType,
    state: Mutex<ControlPlaneAcceptorState>,
    accept_task: std::sync::OnceLock<AbortHandle>,
    /// Limits the number of concurrent in-flight connection handler tasks.
    handler_semaphore: Arc<Semaphore>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ControlPlaneAcceptor {
    ///
    /// # Description
    ///
    /// Creates a new control-plane acceptor and spawns its background accept loop.
    ///
    /// # Arguments
    ///
    /// - `listener`: Bound listener socket.
    /// - `listener_sockaddr`: Listener socket address used for diagnostics and resource ownership.
    /// - `listener_socket_type`: Listener socket type metadata.
    ///
    /// # Returns
    ///
    /// Returns a shared handle to the newly created acceptor.
    ///
    pub fn new(
        listener: SocketListener,
        listener_sockaddr: String,
        listener_socket_type: SocketType,
    ) -> Arc<Self> {
        // Build the Arc first so that the strong reference count is non-zero before spawning the
        // accept loop. The previous `Arc::new_cyclic` approach had a race: the spawned task could
        // be polled on another worker thread before `new_cyclic` finished setting the strong count,
        // causing `Weak::upgrade()` to return `None` and the accept loop to never start.
        let acceptor: Arc<Self> = Arc::new(Self {
            listener,
            _listener_sockaddr: listener_sockaddr,
            _listener_socket_type: listener_socket_type,
            state: Mutex::new(ControlPlaneAcceptorState::default()),
            accept_task: std::sync::OnceLock::new(),
            handler_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONTROL_PLANE_HANDLERS)),
        });

        // Spawn the accept loop now that the Arc is fully constructed.
        let weak_self: Weak<Self> = Arc::downgrade(&acceptor);
        let accept_task: JoinHandle<()> = spawn(async move {
            if let Some(acceptor) = weak_self.upgrade() {
                acceptor.run().await;
            } else {
                debug!("accept loop did not start: acceptor was dropped before task was polled");
            }
        });
        acceptor
            .accept_task
            .set(accept_task.abort_handle())
            .expect("control-plane acceptor abort handle must be initialized exactly once");

        acceptor
    }

    ///
    /// # Description
    ///
    /// Registers interest in the next control-plane connection from a specific User VM.
    ///
    /// # Arguments
    ///
    /// - `user_vm_id`: Identifier of the User VM that is expected to connect.
    ///
    /// # Returns
    ///
    /// Returns a oneshot receiver that resolves to the routed control-plane stream.
    ///
    pub async fn register_uservm(
        &self,
        user_vm_id: UserVmIdentifier,
    ) -> Result<Receiver<SocketStream>> {
        self.register(ControlPlanePeer::UserVm(user_vm_id)).await
    }

    ///
    /// # Description
    ///
    /// Removes any pending control-plane registration for a specific User VM.
    ///
    /// # Arguments
    ///
    /// - `user_vm_id`: Identifier of the User VM whose registration should be removed.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    ///
    pub async fn unregister_uservm(&self, user_vm_id: UserVmIdentifier) {
        self.unregister(&ControlPlanePeer::UserVm(user_vm_id)).await;
    }

    ///
    /// # Description
    ///
    /// Registers interest in the next control-plane connection from a specific Linux daemon.
    ///
    /// # Arguments
    ///
    /// - `tenant_id`: Tenant identifier associated with the Linux daemon instance.
    ///
    /// # Returns
    ///
    /// Returns a oneshot receiver that resolves to the routed control-plane stream.
    ///
    pub async fn register_linuxd(&self, tenant_id: &str) -> Result<Receiver<SocketStream>> {
        self.register(ControlPlanePeer::LinuxDaemon(tenant_id.to_string()))
            .await
    }

    ///
    /// # Description
    ///
    /// Removes any pending control-plane registration for a tenant's Linux daemon.
    ///
    /// # Arguments
    ///
    /// - `tenant_id`: Tenant identifier associated with the Linux daemon instance.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    ///
    pub async fn unregister_linuxd(&self, tenant_id: &str) {
        self.unregister(&ControlPlanePeer::LinuxDaemon(tenant_id.to_string()))
            .await;
    }

    ///
    /// # Description
    ///
    /// Installs a pending waiter or immediately delivers an early-arriving buffered stream.
    ///
    /// # Arguments
    ///
    /// - `peer`: Logical identity of the peer whose control-plane stream is being awaited.
    ///
    /// # Returns
    ///
    /// Returns a oneshot receiver for the matching control-plane stream.
    ///
    async fn register(&self, peer: ControlPlanePeer) -> Result<Receiver<SocketStream>> {
        let (tx, rx): (Sender<SocketStream>, Receiver<SocketStream>) = channel();
        let mut state = self.state.lock().await;

        if state.pending.contains_key(&peer) {
            let reason: String = format!("duplicate control-plane registration ({peer:?})");
            error!("register(): {reason}");
            anyhow::bail!(reason);
        }

        // If the notification from the peer arrived early, still return a oneshot channel but
        // immediately send the result.
        if let Some(stream) = state.early.shift_remove(&peer) {
            if tx.send(stream).is_err() {
                let reason: String =
                    format!("failed to deliver buffered control-plane connection ({peer:?})");
                error!("register(): {reason}");
                anyhow::bail!(reason);
            }
            debug!("register(): delivered buffered control-plane connection ({peer:?})");
            return Ok(rx);
        }

        state.pending.insert(peer.clone(), tx);
        debug!("register(): installed control-plane waiter ({peer:?})");
        Ok(rx)
    }

    ///
    /// # Description
    ///
    /// Removes a pending waiter from the acceptor state.
    ///
    /// # Arguments
    ///
    /// - `peer`: Logical identity of the peer to unregister.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    ///
    async fn unregister(&self, peer: &ControlPlanePeer) {
        let mut state = self.state.lock().await;
        if state.pending.remove(peer).is_some() {
            debug!("unregister(): removed pending control-plane waiter ({peer:?})");
        }
        if state.early.shift_remove(peer).is_some() {
            debug!("unregister(): dropped buffered early control-plane connection ({peer:?})");
        }
    }

    ///
    /// # Description
    ///
    /// Runs the background accept loop and routes accepted streams to pending waiters.
    ///
    /// Each accepted connection is handled in its own spawned task so that a slow or
    /// half-open connection (one that completes the transport handshake but never sends a
    /// registration message) cannot block subsequent accepts.
    ///
    /// # Arguments
    ///
    /// This function does not return under normal operation.
    ///
    async fn run(self: Arc<Self>) {
        loop {
            // Accept new connection.
            let accept_result: Result<SocketStream, ::std::io::Error> =
                self.listener.accept().await;
            let stream: SocketStream = match accept_result {
                Ok(stream) => stream,
                Err(error) => {
                    error!("run(): failed to accept control-plane connection (error={error:?})");
                    continue;
                },
            };

            // Spawn a task to read the registration and route the stream. This prevents a
            // single slow connection from blocking the entire accept loop.
            let permit = match self.handler_semaphore.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    warn!("run(): dropping connection, handler concurrency limit reached");
                    drop(stream);
                    continue;
                },
            };
            let acceptor: Weak<Self> = Arc::downgrade(&self);
            spawn(async move {
                if let Some(acceptor) = acceptor.upgrade() {
                    acceptor.handle_connection(stream).await;
                }
                drop(permit);
            });
        }
    }

    ///
    /// # Description
    ///
    /// Reads the registration from an accepted connection and routes it to the appropriate
    /// pending waiter or the early-arrival buffer.
    ///
    /// # Arguments
    ///
    /// - `stream`: Newly accepted control-plane stream.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    ///
    async fn handle_connection(self: Arc<Self>, mut stream: SocketStream) {
        // Read peer from connection.
        let peer: ControlPlanePeer =
            match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, Self::read_registration(&mut stream)).await
            {
                Ok(Ok(peer)) => peer,
                Ok(Err(error)) => {
                    warn!(
                        "handle_connection(): dropping control-plane connection with invalid \
                         registration (error={error:?})"
                    );
                    return;
                },
                Err(error) => {
                    warn!(
                        "handle_connection(): dropping control-plane connection after \
                         registration timeout (error={error:?})"
                    );
                    return;
                },
            };

        // Notify awaiting peer, or store peer in early arrivals.
        let mut state: MutexGuard<'_, ControlPlaneAcceptorState> = self.state.lock().await;
        if let Some(waiter) = state.pending.remove(&peer) {
            if waiter.send(stream).is_err() {
                warn!(
                    "handle_connection(): waiter dropped before control-plane delivery ({peer:?})"
                );
            }
            return;
        }

        // Evict the oldest buffered connection if the early buffer is at capacity and the
        // incoming peer is not already present.
        if state.early.len() >= MAX_EARLY_CONTROL_PLANE_CONNECTIONS
            && !state.early.contains_key(&peer)
        {
            if let Some((evicted, _)) = state.early.shift_remove_index(0) {
                warn!(
                    "handle_connection(): evicting oldest early control-plane connection \
                     ({evicted:?}) to make room"
                );
            }
        }
        warn!("handle_connection(): buffering early control-plane connection ({peer:?})");
        state.early.insert(peer, stream);
    }

    ///
    /// # Description
    ///
    /// Reads and decodes the registration handshake from an accepted control-plane stream.
    ///
    /// # Arguments
    ///
    /// - `stream`: Newly accepted control-plane stream.
    ///
    /// # Returns
    ///
    /// Returns the logical identity of the connecting peer.
    ///
    async fn read_registration(stream: &mut SocketStream) -> Result<ControlPlanePeer> {
        let mut header: [u8; ControlPlaneRegistrationMessage::HEADER_SIZE] =
            [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        stream.read_exact(&mut header).await?;
        let tenant_id_len: usize = usize::from(u16::from_le_bytes([
            header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET],
            header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET + 1],
        ]));
        if tenant_id_len > ControlPlaneRegistrationMessage::MAX_TENANT_ID_LEN {
            anyhow::bail!(
                "tenant_id length {tenant_id_len} exceeds maximum {}",
                ControlPlaneRegistrationMessage::MAX_TENANT_ID_LEN
            );
        }
        let mut tenant_id_bytes: Vec<u8> = vec![0u8; tenant_id_len];
        if tenant_id_len > 0 {
            stream.read_exact(&mut tenant_id_bytes).await?;
        }

        let registration: ControlPlaneRegistrationMessage =
            ControlPlaneRegistrationMessage::try_from_parts(&header, &tenant_id_bytes)?;

        match registration.user_vm_id() {
            Some(user_vm_id) => Ok(ControlPlanePeer::UserVm(user_vm_id)),
            None => Ok(ControlPlanePeer::LinuxDaemon(
                registration.tenant_id().unwrap_or_default().to_string(),
            )),
        }
    }
}

impl Drop for ControlPlaneAcceptor {
    ///
    /// # Description
    ///
    /// Aborts the background accept task when the acceptor is dropped.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    ///
    fn drop(&mut self) {
        if let Some(handle) = self.accept_task.get() {
            handle.abort();
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::control_plane_api::{
        ControlPlanePeerKind,
        ControlPlaneRegistrationMessage,
    };
    use ::syscomm::{
        SocketType,
        UnboundSocket,
        WriteAll,
    };

    /// Helper: bind a TCP listener on localhost with an OS-assigned port and return the acceptor
    /// together with the connect address.
    async fn setup_acceptor() -> (Arc<ControlPlaneAcceptor>, String) {
        let listener: SocketListener = UnboundSocket::new(SocketType::Tcp)
            .bind("127.0.0.1:0")
            .await
            .expect("failed to bind test listener");

        let addr: String = match &listener {
            SocketListener::Tcp { listener, .. } => {
                let bound = listener.local_addr().expect("local_addr failed");
                format!("127.0.0.1:{}", bound.port())
            },
            #[cfg(unix)]
            _ => unreachable!(),
        };

        let acceptor: Arc<ControlPlaneAcceptor> =
            ControlPlaneAcceptor::new(listener, addr.clone(), SocketType::Tcp);

        (acceptor, addr)
    }

    /// Helper: connect to the acceptor and send a registration message.
    async fn connect_and_register(addr: &str, msg: &[u8]) -> SocketStream {
        let mut stream: SocketStream = UnboundSocket::new(SocketType::Tcp)
            .connect(addr)
            .await
            .expect("failed to connect to test acceptor");
        stream
            .write_all(msg)
            .await
            .expect("failed to write registration");
        stream
    }

    #[tokio::test]
    async fn register_then_connect_delivers_stream() {
        let (acceptor, addr) = setup_acceptor().await;

        let user_vm_id: UserVmIdentifier = UserVmIdentifier::new(42);
        let rx: Receiver<SocketStream> = acceptor
            .register_uservm(user_vm_id)
            .await
            .expect("register failed");

        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_uservm(user_vm_id)
            .to_bytes()
            .expect("to_bytes failed");
        let _client: SocketStream = connect_and_register(&addr, &registration).await;

        let delivered: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx)
            .await
            .expect("timed out waiting for delivery")
            .expect("channel error");
        drop(delivered);
    }

    #[tokio::test]
    async fn early_arrival_buffered_and_delivered() {
        let (acceptor, addr) = setup_acceptor().await;

        let tenant_id: &str = "tenant-early";
        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_linuxd(tenant_id)
            .expect("for_linuxd failed")
            .to_bytes()
            .expect("to_bytes failed");
        let _client: SocketStream = connect_and_register(&addr, &registration).await;

        // Wait until the accept loop actually buffers the early connection.
        timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, async {
            loop {
                let buffered: bool = {
                    let state = acceptor.state.lock().await;
                    state
                        .early
                        .contains_key(&ControlPlanePeer::LinuxDaemon(tenant_id.to_string()))
                };
                if buffered {
                    break;
                }
                tokio::task::yield_now().await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("timed out waiting for early connection to be buffered");

        let rx: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("register failed");

        let delivered: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx)
            .await
            .expect("timed out waiting for delivery")
            .expect("channel error");
        drop(delivered);
    }

    #[tokio::test]
    async fn duplicate_registration_returns_error() {
        let (acceptor, _addr) = setup_acceptor().await;

        let user_vm_id: UserVmIdentifier = UserVmIdentifier::new(99);

        acceptor
            .register_uservm(user_vm_id)
            .await
            .expect("first register should succeed");

        let result = acceptor.register_uservm(user_vm_id).await;
        assert!(result.is_err(), "duplicate registration should fail");
    }

    #[tokio::test]
    async fn unregister_cleans_early_buffer() {
        let (acceptor, addr) = setup_acceptor().await;

        let tenant_id: &str = "tenant-unregister";
        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_linuxd(tenant_id)
            .expect("for_linuxd failed")
            .to_bytes()
            .expect("to_bytes failed");
        let _client: SocketStream = connect_and_register(&addr, &registration).await;

        // Wait until the accept loop actually buffers the early connection.
        timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, async {
            loop {
                let buffered: bool = {
                    let state = acceptor.state.lock().await;
                    state
                        .early
                        .contains_key(&ControlPlanePeer::LinuxDaemon(tenant_id.to_string()))
                };
                if buffered {
                    break;
                }
                tokio::task::yield_now().await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("timed out waiting for early connection to be buffered");

        // Unregister should drop the buffered entry even though nothing was pending.
        acceptor.unregister_linuxd(tenant_id).await;

        // A subsequent register should NOT see a buffered stream (it was cleaned up).
        let rx: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("register after unregister should succeed");

        // The channel should not resolve immediately since the early buffer was cleaned.
        let result = tokio::time::timeout(std::time::Duration::from_millis(300), rx).await;
        assert!(result.is_err(), "should timeout because early entry was cleaned");
    }

    /// Reproduction test for issue #1996: after a successful linuxd register+deliver cycle, a
    /// second register+deliver for the **same** tenant_id must succeed without timeout.
    #[tokio::test]
    async fn reregister_same_tenant_after_delivery_succeeds() {
        let (acceptor, addr) = setup_acceptor().await;
        let tenant_id: &str = "runner";
        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_linuxd(tenant_id)
            .expect("for_linuxd failed")
            .to_bytes()
            .expect("to_bytes failed");

        // --- First cycle (simulates first nanvixd invocation) ---
        let rx1: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("first register failed");

        let _client1: SocketStream = connect_and_register(&addr, &registration).await;

        let stream1: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx1)
            .await
            .expect("timed out waiting for first delivery")
            .expect("channel error on first delivery");
        // Drop the delivered stream to simulate cleanup / linuxd shutdown.
        drop(stream1);

        // --- Second cycle (simulates second nanvixd invocation for same tenant) ---
        let rx2: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("second register failed");

        let _client2: SocketStream = connect_and_register(&addr, &registration).await;

        let _stream2: SocketStream = timeout(std::time::Duration::from_secs(5), rx2)
            .await
            .expect("timed out waiting for second delivery (issue #1996)")
            .expect("channel error on second delivery");
    }

    /// Reproduction test for issue #1996 variant: a stale early-buffer entry from the first run
    /// must not be served to the second register call after the first linuxd was cleaned up.
    #[tokio::test]
    async fn stale_early_entry_not_served_after_cleanup() {
        let (acceptor, addr) = setup_acceptor().await;
        let tenant_id: &str = "runner";
        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_linuxd(tenant_id)
            .expect("for_linuxd failed")
            .to_bytes()
            .expect("to_bytes failed");

        // --- First cycle: normal register + deliver ---
        let rx1: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("first register failed");

        let _client1: SocketStream = connect_and_register(&addr, &registration).await;

        let stream1: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx1)
            .await
            .expect("timed out waiting for first delivery")
            .expect("channel error on first delivery");

        // Simulate cleanup: drop the old stream but do NOT call unregister (matches real
        // SimpleSandboxCache::cleanup which does not call unregister_linuxd).
        drop(stream1);

        // Simulate the old linuxd reconnecting (or a stale connection arriving) before the second
        // register. This connection should land in the early buffer.
        let _stale_client: SocketStream = connect_and_register(&addr, &registration).await;

        // Wait until the accept loop actually buffers the early connection.
        timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, async {
            loop {
                let buffered: bool = {
                    let state = acceptor.state.lock().await;
                    state
                        .early
                        .contains_key(&ControlPlanePeer::LinuxDaemon(tenant_id.to_string()))
                };
                if buffered {
                    break;
                }
                tokio::task::yield_now().await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("timed out waiting for early connection to be buffered");

        // Unregister simulates what a corrected cleanup path should do.
        acceptor.unregister_linuxd(tenant_id).await;

        // --- Second cycle ---
        let rx2: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("second register failed");

        // Connect the REAL second linuxd.
        let _client2: SocketStream = connect_and_register(&addr, &registration).await;

        let _stream2: SocketStream = timeout(std::time::Duration::from_secs(5), rx2)
            .await
            .expect("timed out waiting for second delivery")
            .expect("channel error on second delivery");
    }

    /// Reproduction test for issue #1996 blocking variant: a half-open stale connection that
    /// blocks the single-threaded accept loop can prevent subsequent registrations from being
    /// served.
    #[tokio::test]
    async fn half_open_stale_connection_does_not_block_new_delivery() {
        let (acceptor, addr) = setup_acceptor().await;
        let tenant_id: &str = "runner";
        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_linuxd(tenant_id)
            .expect("for_linuxd failed")
            .to_bytes()
            .expect("to_bytes failed");

        // --- First cycle: normal register + deliver ---
        let rx1: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("first register failed");

        let _client1: SocketStream = connect_and_register(&addr, &registration).await;

        let stream1: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx1)
            .await
            .expect("timed out waiting for first delivery")
            .expect("channel error on first delivery");
        drop(stream1);

        // Inject a half-open connection: connect to the acceptor but do NOT send any registration
        // data. This simulates a dying process that completed the TCP handshake but never sent
        // its registration message.
        let _half_open: SocketStream = UnboundSocket::new(SocketType::Tcp)
            .connect(&addr)
            .await
            .expect("half-open connect failed");

        // --- Second cycle: must not be blocked by the half-open connection ---
        let rx2: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("second register failed");

        let _client2: SocketStream = connect_and_register(&addr, &registration).await;

        // Use a 5-second timeout — well under the 60-second read_registration timeout.
        // If the accept loop is single-threaded and blocked by the half-open connection,
        // this will fail.
        let _stream2: SocketStream = timeout(std::time::Duration::from_secs(5), rx2)
            .await
            .expect(
                "timed out waiting for second delivery — accept loop likely blocked by half-open \
                 connection (issue #1996)",
            )
            .expect("channel error on second delivery");
    }

    #[tokio::test]
    async fn correct_delivery_per_peer() {
        let (acceptor, addr) = setup_acceptor().await;

        let vm_a: UserVmIdentifier = UserVmIdentifier::new(1);
        let vm_b: UserVmIdentifier = UserVmIdentifier::new(2);

        let rx_a: Receiver<SocketStream> = acceptor
            .register_uservm(vm_a)
            .await
            .expect("register A failed");
        let rx_b: Receiver<SocketStream> = acceptor
            .register_uservm(vm_b)
            .await
            .expect("register B failed");

        // Connect B first, then A (reverse order).
        let reg_b: Vec<u8> = ControlPlaneRegistrationMessage::for_uservm(vm_b)
            .to_bytes()
            .expect("to_bytes failed");
        let _client_b: SocketStream = connect_and_register(&addr, &reg_b).await;

        let reg_a: Vec<u8> = ControlPlaneRegistrationMessage::for_uservm(vm_a)
            .to_bytes()
            .expect("to_bytes failed");
        let _client_a: SocketStream = connect_and_register(&addr, &reg_a).await;

        // Both should be delivered to their respective receivers regardless of order.
        let _stream_a: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx_a)
            .await
            .expect("timed out A")
            .expect("channel error A");
        let _stream_b: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx_b)
            .await
            .expect("timed out B")
            .expect("channel error B");
    }

    #[tokio::test]
    async fn oversized_tenant_id_rejected() {
        let (acceptor, addr) = setup_acceptor().await;

        // Build a raw registration message with tenant_id_len exceeding MAX_TENANT_ID_LEN.
        let oversized_len: u16 =
            (ControlPlaneRegistrationMessage::MAX_TENANT_ID_LEN as u16).saturating_add(1);
        let mut header: [u8; ControlPlaneRegistrationMessage::HEADER_SIZE] =
            [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        // Peer kind = LinuxDaemon.
        header[ControlPlaneRegistrationMessage::PEER_KIND_OFFSET] =
            ControlPlanePeerKind::LinuxDaemon.into();
        // tenant_id_len in little-endian.
        let len_bytes: [u8; 2] = oversized_len.to_le_bytes();
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET] = len_bytes[0];
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET + 1] = len_bytes[1];

        let tenant_id_payload: Vec<u8> = vec![b'x'; oversized_len as usize];
        let mut wire: Vec<u8> = Vec::with_capacity(header.len() + tenant_id_payload.len());
        wire.extend_from_slice(&header);
        wire.extend_from_slice(&tenant_id_payload);

        let _client: SocketStream = connect_and_register(&addr, &wire).await;

        // Small delay so the acceptor has time to process (and reject) the connection.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // A subsequent valid registration must still succeed, proving the acceptor was not
        // disrupted by the oversized message.
        let tenant_id: &str = "tenant-valid";
        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_linuxd(tenant_id)
            .expect("for_linuxd failed")
            .to_bytes()
            .expect("to_bytes failed");
        let _client2: SocketStream = connect_and_register(&addr, &registration).await;

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let rx: Receiver<SocketStream> = acceptor
            .register_linuxd(tenant_id)
            .await
            .expect("register failed");

        let delivered: SocketStream = timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, rx)
            .await
            .expect("timed out waiting for delivery")
            .expect("channel error");
        drop(delivered);
    }

    #[tokio::test]
    async fn early_buffer_full_evicts_oldest_peer() {
        let (acceptor, addr) = setup_acceptor().await;

        // Pre-fill the early buffer to capacity with synthetic peers.  We use the internal state
        // directly so we do not need MAX_EARLY_CONTROL_PLANE_CONNECTIONS real TCP connections.
        {
            let mut state = acceptor.state.lock().await;
            for i in 0..MAX_EARLY_CONTROL_PLANE_CONNECTIONS {
                let peer = ControlPlanePeer::UserVm(UserVmIdentifier::new(i as u32));
                // We need a dummy SocketStream; create a loopback pair and use one end.
                let listener = UnboundSocket::new(SocketType::Tcp)
                    .bind("127.0.0.1:0")
                    .await
                    .expect("bind failed");
                let dummy_addr: String = match &listener {
                    SocketListener::Tcp { listener, .. } => {
                        let bound = listener.local_addr().expect("local_addr failed");
                        format!("127.0.0.1:{}", bound.port())
                    },
                    #[cfg(unix)]
                    _ => unreachable!(),
                };
                let connect_fut = UnboundSocket::new(SocketType::Tcp).connect(&dummy_addr);
                let accept_fut = listener.accept();
                let (client, server) = tokio::join!(connect_fut, accept_fut);
                let _client = client.expect("connect failed");
                let server_stream = server.expect("accept failed");
                state.early.insert(peer, server_stream);
            }
            assert_eq!(state.early.len(), MAX_EARLY_CONTROL_PLANE_CONNECTIONS);
        }

        // Connect one more peer through the real accept loop — this should evict peer 0 (the
        // oldest insertion).
        let new_tenant: &str = "tenant-overflow";
        let registration: Vec<u8> = ControlPlaneRegistrationMessage::for_linuxd(new_tenant)
            .expect("for_linuxd failed")
            .to_bytes()
            .expect("to_bytes failed");
        let _client = connect_and_register(&addr, &registration).await;

        // Wait for the acceptor loop to process the connection.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify state: the oldest peer (UserVm 0) should have been evicted, and the new peer
        // should be present.
        let state = acceptor.state.lock().await;
        assert_eq!(
            state.early.len(),
            MAX_EARLY_CONTROL_PLANE_CONNECTIONS,
            "buffer should still be at capacity after eviction + insertion"
        );
        assert!(
            !state
                .early
                .contains_key(&ControlPlanePeer::UserVm(UserVmIdentifier::new(0))),
            "oldest peer (UserVm 0) should have been evicted"
        );
        // The second-oldest should still be present.
        assert!(
            state
                .early
                .contains_key(&ControlPlanePeer::UserVm(UserVmIdentifier::new(1))),
            "second peer (UserVm 1) should still be buffered"
        );
        assert!(
            state
                .early
                .contains_key(&ControlPlanePeer::LinuxDaemon(new_tenant.to_string())),
            "newly arrived peer should be buffered"
        );
    }
}
