// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config::GATEWAY_ACCEPT_TIMEOUT;
use ::anyhow::Result;
use ::control_plane_api::{
    LinuxdCommand,
    LinuxdControlMessage,
};
use ::log::{
    error,
    trace,
};
use ::std::sync::Arc;
use ::syscomm::{
    SocketListener,
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::tokio::{
    sync::{
        Mutex,
        MutexGuard,
    },
    task::JoinHandle,
    time::timeout,
};

//==================================================================================================
// Structures
//==================================================================================================

/// State associated with a user VM connected to this linuxd instance.
#[derive(Clone)]
pub struct UserVmHandle {
    /// Writer half used by worker threads to send responses without contending with the reader.
    user_vm_writer: Arc<Mutex<SocketStreamWriter>>,
    /// Identifier of the user VM.
    user_vm_id: u32,
    /// Address of the gateway socket to connect to.
    gateway_sockaddr: String,
    /// Type of gateway socket to connect to.
    gateway_socket_type: SocketType,
    /// Lazy-initialized reader half of the gateway stream.
    gateway_reader: Arc<Mutex<Option<Arc<Mutex<SocketStreamReader>>>>>,
    /// Lazy-initialized writer half of the gateway stream.
    gateway_writer: Arc<Mutex<Option<Arc<Mutex<SocketStreamWriter>>>>>,
    /// Listener for the gateway socket.
    gateway_listener: Arc<Mutex<Option<SocketListener>>>,
    /// Join handle to the task that reads from the user VM stream.
    user_vm_reader_handle: Arc<Mutex<Option<JoinHandle<Result<()>>>>>,
    /// Writer half of the control-plane stream shared with nanvixd.
    control_plane_writer: Arc<Mutex<SocketStreamWriter>>,
}

impl UserVmHandle {
    pub fn new(
        user_vm_writer: SocketStreamWriter,
        user_vm_id: u32,
        gateway_sockaddr: &str,
        gateway_socket_type: &SocketType,
        user_vm_reader_handle: JoinHandle<Result<()>>,
        control_plane_writer: Arc<Mutex<SocketStreamWriter>>,
    ) -> Self {
        trace!("new(): user_vm_id={}, gateway_sockaddr={}", user_vm_id, gateway_sockaddr);
        Self {
            user_vm_writer: Arc::new(Mutex::new(user_vm_writer)),
            user_vm_id,
            gateway_sockaddr: gateway_sockaddr.to_string(),
            gateway_socket_type: *gateway_socket_type,
            gateway_reader: Arc::new(Mutex::new(None)),
            gateway_writer: Arc::new(Mutex::new(None)),
            gateway_listener: Arc::new(Mutex::new(None)),
            user_vm_reader_handle: Arc::new(Mutex::new(Some(user_vm_reader_handle))),
            control_plane_writer,
        }
    }

    /// Get the writer half of the user VM stream.
    pub fn get_user_vm_writer(&self) -> Arc<Mutex<SocketStreamWriter>> {
        self.user_vm_writer.clone()
    }

    /// Lazily establish (or reuse) the gateway connection and return its split reader & writer.
    pub async fn get_gateway_vm_stream(
        &self,
    ) -> Result<(Arc<Mutex<SocketStreamReader>>, Arc<Mutex<SocketStreamWriter>>)> {
        // Acquire reader and writer locks upfront to avoid races between fast and slow paths.
        let mut reader_slot: MutexGuard<'_, Option<Arc<Mutex<SocketStreamReader>>>> =
            self.gateway_reader.lock().await;
        let mut writer_slot: MutexGuard<'_, Option<Arc<Mutex<SocketStreamWriter>>>> =
            self.gateway_writer.lock().await;

        // Fast path: if reader already initialized, attempt to reuse both halves.
        match (reader_slot.as_ref().cloned(), writer_slot.as_ref().cloned()) {
            (Some(reader_arc), Some(writer_arc)) => {
                trace!("reusing existing gateway stream");
                return Ok((reader_arc, writer_arc));
            },
            (Some(_), None) | (None, Some(_)) => {
                unreachable!("gateway reader/writer initialized asymmetrically");
            },
            (None, None) => {
                trace!("gateway stream not initialized; binding new listener");
            },
        };

        // Slow path: need to establish the gateway connection.

        let mut gateway_listener_slot: MutexGuard<'_, Option<SocketListener>> =
            self.gateway_listener.lock().await;

        let unbound_socket: UnboundSocket = UnboundSocket::new(self.gateway_socket_type);
        let gateway_listener: SocketListener =
            match unbound_socket.bind(&self.gateway_sockaddr).await {
                Ok(listener) => listener,
                Err(e) => {
                    let reason: String = format!(
                        "failed to bind to gateway socket address (address={}, error={e:?})",
                        self.gateway_sockaddr
                    );
                    trace!("{}", reason);
                    return Err(anyhow::anyhow!(reason));
                },
            };

        trace!("Listening for gateway connection on: {}", self.gateway_sockaddr);

        // Notify nanvixd that the gateway listener is bound and ready.
        {
            let msg: LinuxdControlMessage =
                LinuxdControlMessage::new(LinuxdCommand::GatewayReady, self.user_vm_id);
            let mut msg_bytes: [u8; LinuxdControlMessage::WIRE_SIZE] =
                [0u8; LinuxdControlMessage::WIRE_SIZE];
            msg.to_bytes(&mut msg_bytes);

            let mut writer: MutexGuard<'_, SocketStreamWriter> =
                self.control_plane_writer.lock().await;
            if let Err(e) = writer.write_all(&msg_bytes).await {
                let reason: String = format!(
                    "failed to send GatewayReady on control-plane (gateway_addr={}, error={e:?})",
                    self.gateway_sockaddr
                );
                error!("get_gateway_vm_stream(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }
            trace!("sent GatewayReady on control-plane (gateway_addr={})", self.gateway_sockaddr);
        }

        // Accept connection from gateway client.
        let stream: SocketStream =
            match timeout(GATEWAY_ACCEPT_TIMEOUT, gateway_listener.accept()).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(e)) => {
                    let reason: String = format!(
                        "failed to accept connection from gateway (gateway_addr={}, error={e:?})",
                        self.gateway_sockaddr
                    );
                    error!("get_gateway_vm_stream(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                },
                Err(_) => {
                    let reason: String = format!(
                        "timed out waiting for gateway connection (gateway_addr={}, \
                         timeout={GATEWAY_ACCEPT_TIMEOUT:?})",
                        self.gateway_sockaddr
                    );
                    error!("get_gateway_vm_stream(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            };

        trace!("Connected to gateway");

        let (reader_half, writer_half): (SocketStreamReader, SocketStreamWriter) = stream.split();
        let reader_arc: Arc<Mutex<SocketStreamReader>> = Arc::new(Mutex::new(reader_half));
        let writer_arc: Arc<Mutex<SocketStreamWriter>> = Arc::new(Mutex::new(writer_half));

        // Store halves.
        *reader_slot = Some(reader_arc.clone());
        *writer_slot = Some(writer_arc.clone());
        *gateway_listener_slot = Some(gateway_listener);

        Ok((reader_arc, writer_arc))
    }

    pub async fn take_user_vm_reader_handle(&self) -> Option<JoinHandle<Result<()>>> {
        let mut guard: MutexGuard<'_, Option<JoinHandle<Result<()>>>> =
            self.user_vm_reader_handle.lock().await;
        guard.take()
    }
}
