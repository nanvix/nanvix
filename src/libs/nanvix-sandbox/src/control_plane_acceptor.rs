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
    MAX_EARLY_CONTROL_PLANE_CONNECTIONS,
    MAX_WEAK_UPGRADE_RETRIES,
};
use ::anyhow::Result;
use ::control_plane_api::ControlPlaneRegistrationMessage;
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
    /// Peers that arrived before a request to register them.
    early: HashMap<ControlPlanePeer, SocketStream>,
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
    accept_task: AbortHandle,
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
        // We need a new_cyclic here in order to be able to start the acceptor task as part of the
        // call to `new`. The acceptor task requires a reference to self, hence the cyclic
        // dependency.
        Arc::new_cyclic(|weak_self: &Weak<Self>| {
            let weak_self: Weak<Self> = weak_self.clone();
            let accept_task: JoinHandle<()> = spawn(async move {
                // Try to upgrade immediately in case `Arc::new_cyclic` already finished
                // setting the strong reference count before this task was first polled.
                if let Some(acceptor) = weak_self.upgrade() {
                    acceptor.run().await;
                    return;
                }

                // Yield until `Arc::new_cyclic` finishes setting the strong reference count. A
                // single yield is not enough: in a multi-threaded runtime a different worker
                // thread can re-poll this future before the originating thread completes
                // `new_cyclic`, causing `upgrade()` to return `None`.
                for _ in 0..MAX_WEAK_UPGRADE_RETRIES {
                    tokio::task::yield_now().await;
                    if let Some(acceptor) = weak_self.upgrade() {
                        acceptor.run().await;
                        return;
                    }
                }
                error!(
                    "accept loop failed to start: could not upgrade weak reference after \
                     {MAX_WEAK_UPGRADE_RETRIES} yield cycles"
                );
            });

            Self {
                listener,
                _listener_sockaddr: listener_sockaddr,
                _listener_socket_type: listener_socket_type,
                state: Mutex::new(ControlPlaneAcceptorState::default()),
                accept_task: accept_task.abort_handle(),
            }
        })
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
        if let Some(stream) = state.early.remove(&peer) {
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
        if state.early.remove(peer).is_some() {
            debug!("unregister(): dropped buffered early control-plane connection ({peer:?})");
        }
    }

    ///
    /// # Description
    ///
    /// Runs the background accept loop and routes accepted streams to pending waiters.
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
            let mut stream: SocketStream = match accept_result {
                Ok(stream) => stream,
                Err(error) => {
                    error!("run(): failed to accept control-plane connection (error={error:?})");
                    continue;
                },
            };

            // Read peer from connection.
            let peer: ControlPlanePeer =
                match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, Self::read_registration(&mut stream))
                    .await
                {
                    Ok(Ok(peer)) => peer,
                    Ok(Err(error)) => {
                        warn!(
                            "run(): dropping control-plane connection with invalid registration \
                             (error={error:?})"
                        );
                        continue;
                    },
                    Err(error) => {
                        warn!(
                            "run(): dropping control-plane connection after registration timeout \
                             (error={error:?})"
                        );
                        continue;
                    },
                };

            // Notify awaiting peer, or store peer in early arrivals.
            let mut state: MutexGuard<'_, ControlPlaneAcceptorState> = self.state.lock().await;
            if let Some(waiter) = state.pending.remove(&peer) {
                if waiter.send(stream).is_err() {
                    warn!("run(): waiter dropped before control-plane delivery ({peer:?})");
                }
                continue;
            }

            // Evict the oldest buffered connection if the early buffer is at capacity.
            if state.early.len() >= MAX_EARLY_CONTROL_PLANE_CONNECTIONS {
                if let Some(evicted) = state.early.keys().next().cloned() {
                    warn!(
                        "run(): evicting oldest early control-plane connection ({evicted:?}) to \
                         make room"
                    );
                    state.early.remove(&evicted);
                }
            }
            warn!("run(): buffering early control-plane connection ({peer:?})");
            state.early.insert(peer, stream);
        }
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
        self.accept_task.abort();
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::control_plane_api::ControlPlaneRegistrationMessage;
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

        // Small delay to let the acceptor loop buffer the connection.
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

        // Wait for the connection to be buffered.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

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
}
