// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::route::{
    GatewayLookupTable,
    GatewayPeer,
};
use ::anyhow::Result;
use ::std::{
    future::Future,
    net::SocketAddr,
    pin::Pin,
};
use ::sys::{
    config,
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    sync::{
        mpsc,
        mpsc::{
            UnboundedReceiver,
            UnboundedSender,
        },
    },
};

//==================================================================================================
// Traits
//==================================================================================================

///
/// Gateway Client
///
pub trait GatewayClient: Sized + Send {
    ///
    /// # Description
    ///
    /// Creates a new gateway client.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address of the client.
    /// - `tx`: Transmit endpoint for messages to clients.
    /// - `rx`: Receive endpoint for messages from clients.
    ///
    /// # Returns
    ///
    /// A new gateway client.
    ///
    fn new(
        addr: SocketAddr,
        tx: UnboundedSender<(SocketAddr, Message)>,
        rx: UnboundedReceiver<Result<Message, anyhow::Error>>,
    ) -> Self;

    ///
    /// # Description
    ///
    /// Runs the gateway client.
    ///
    /// # Parameters
    ///
    /// - `client`: Gateway client.
    /// - `stream`: TCP stream associated with the client.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    fn run(
        client: Self,
        stream: TcpStream,
    ) -> Pin<Box<(dyn Future<Output = Result<(), anyhow::Error>> + std::marker::Send)>>;
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// Gateway
///
pub struct Gateway<T: GatewayClient> {
    /// Address of system daemon.
    systemd_conn: Option<::std::net::TcpStream>,
    /// Address of the gateway for HTTP traffic.
    http_addr: SocketAddr,
    /// Transmit endpoint for messages to clients.
    gateway_client_tx: UnboundedSender<(SocketAddr, Message)>,
    /// Receive endpoint for messages from clients.
    gateway_client_rx: UnboundedReceiver<(SocketAddr, Message)>,
    /// Transmit endpoint for messages to the VM.
    gateway_vm_tx: UnboundedSender<Message>,
    /// Receive endpoint for messages from the VM.
    gateway_vm_rx: UnboundedReceiver<Message>,
    /// Lookup tables.
    lookup_tables: GatewayLookupTable,
    /// System daemon TX endpoint.
    systemd_tx: Option<SysdTx>,
    /// Marker to force ownership over [`GatewayClient`].
    _phantom: std::marker::PhantomData<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

// Type aliases to make clippy happy.
type ClientGatewayRx = UnboundedReceiver<(SocketAddr, Message)>;
type ClientGatewayTx = UnboundedSender<(SocketAddr, Message)>;
type ClientRx = UnboundedReceiver<Result<Message, anyhow::Error>>;
type ClientTx = UnboundedSender<Result<Message, anyhow::Error>>;
type SysdRx = UnboundedReceiver<Message>;
type SysdTx = UnboundedSender<Message>;

impl<T: GatewayClient> Gateway<T> {
    ///
    /// # Description
    ///
    /// Creates a new gateway.
    ///
    /// # Parameters
    ///
    /// - `http_addr`: Address of the gateway for HTTP traffic.
    /// - `systemd_addr`: Address of the system daemon.
    ///
    /// # Returns
    ///
    /// A new gateway.
    ///
    pub fn new(
        http_addr: SocketAddr,
        systemd_addr: Option<String>,
    ) -> Result<(Gateway<T>, UnboundedSender<Message>, UnboundedReceiver<Message>)> {
        trace!("new(): http_addr={:?}, systemd_addr={:?}", http_addr, systemd_addr);

        // Create an asynchronous channel to enable communication from the gateway to the VM.
        let (gateway_vm_tx, vm_rx): (UnboundedSender<Message>, UnboundedReceiver<Message>) =
            mpsc::unbounded_channel();

        // Create an asynchronous channel to enable communication from the VM to the gateway.
        let (vm_tx, gateway_vm_rx): (UnboundedSender<Message>, UnboundedReceiver<Message>) =
            mpsc::unbounded_channel();

        // Create an asynchronous channel to enable communication from the client to the gateway.
        let (gateway_client_tx, gateway_client_rx): (ClientGatewayTx, ClientGatewayRx) =
            mpsc::unbounded_channel();

        // Connect to the system daemon.
        let systemd_conn: Option<::std::net::TcpStream> = if let Some(systemd_addr) = systemd_addr {
            debug!("new(): connecting to system daemon at {:?}", systemd_addr);
            let systemd_addr: SocketAddr = systemd_addr.parse()?;
            Some(std::net::TcpStream::connect(systemd_addr)?)
        } else {
            None
        };

        Ok((
            Self {
                systemd_conn,
                systemd_tx: None,
                http_addr,
                gateway_client_rx,
                gateway_client_tx,
                gateway_vm_tx,
                gateway_vm_rx,
                lookup_tables: GatewayLookupTable::new(),
                _phantom: std::marker::PhantomData,
            },
            vm_tx,
            vm_rx,
        ))
    }

    ///
    /// # Description
    ///
    /// Runs the gateway.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    #[tokio::main]
    pub async fn run(&mut self) -> Result<()> {
        self.spawn_systemd_handler()?;
        let listener: TcpListener = TcpListener::bind(self.http_addr).await?;
        loop {
            tokio::select! {
                // Attempt to accept a new client.
                Ok((stream, addr)) = listener.accept() => {
                   if let Err(e) = self.handle_accept(stream, addr).await {
                        warn!("run(): {:?}", e);
                   }
                },
                // Attempt to receive a message from any peer.
                Some((addr, message)) = self.gateway_client_rx.recv() => {
                    if let Err(e) = self.handle_client_message(addr, message).await {
                        // Failed to handle peer message, send error back to the client.
                        warn!("run(): {:?}", e);
                        if let Ok(peer) = self.lookup_tables.lookup_addr(addr).await {
                            match peer {
                                GatewayPeer::Client(client) => {
                                    if let Err(e) = client.send(Err(e)) {
                                        error!("run(): {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                },
                // Attempt to receive a message from the VM.
                Some(message) = self.gateway_vm_rx.recv() => {
                    if let Err(e) = self.handle_vm_message(message).await {
                        let reason: String = format!("failed to handle message from VM (error={:?})", e);
                        error!("run(): {}", reason);
                        unimplemented!("send error to the VM");
                    }
                }
            }
        }
    }

    ///
    /// # Description
    ///
    /// Handles accept.
    ///
    /// # Parameters
    ///
    /// - `stream`: TCP stream associated with the client.
    /// - `addr`: Address of the client.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    async fn handle_accept(&mut self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        trace!("handle_accept(): addr={:?}", addr);

        // Create an asynchronous channel to enable communication from the gateway to the client.
        let (client_tx, client_rx): (ClientTx, ClientRx) =
            mpsc::unbounded_channel::<Result<Message, anyhow::Error>>();

        let client: Pin<Box<dyn Future<Output = std::result::Result<(), anyhow::Error>> + Send>> =
            T::run(T::new(addr, self.gateway_client_tx.clone(), client_rx), stream);

        // Attempt to register the client.
        self.lookup_tables
            .register_addr(addr, crate::route::GatewayPeer::Client(client_tx))
            .await?;

        let lookup_tables: GatewayLookupTable = self.lookup_tables.clone();
        tokio::task::spawn(async move {
            if let Err(e) = client.await {
                warn!("failed to run client: {:?}", e);
            }

            // Handle client disconnection.
            Self::handle_disconnect(&lookup_tables, addr).await
        });

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Handles a client disconnection.
    ///
    /// # Parameters
    ///
    /// - `lookup_tables`: Lookup tables.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    async fn handle_disconnect(lookup_tables: &GatewayLookupTable, addr: SocketAddr) -> Result<()> {
        trace!("handle_disconnect(): addr={:?}", addr);

        GatewayLookupTable::remove(lookup_tables, addr).await?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Handles a message from a client.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to handle.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    async fn handle_client_message(&mut self, addr: SocketAddr, message: Message) -> Result<()> {
        trace!(
            "handle_client_message(): addr={:?}, message.source={:?}, message.destination={:?}",
            addr,
            message.source,
            message.destination
        );

        // Check if destination is the system daemon.
        if message.destination == ProcessIdentifier::KERNEL {
            let reason: &str = "system daemon is not a valid destination";
            error!("handle_client_message(): {}", reason);
            anyhow::bail!(reason);
        }

        // Check if source does not claim to be the system daemon.
        if message.source == ProcessIdentifier::KERNEL {
            let reason: &str = "system daemon is not a valid source";
            error!("handle_client_message(): {}", reason);
            anyhow::bail!(reason);
        }

        self.lookup_tables
            .register_pid(message.source, addr)
            .await?;

        // Forward message to the VM.
        self.gateway_vm_tx.send(message)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Handles a message from the VM.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to handle.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or `Err(e)` on failure.
    ///
    async fn handle_vm_message(&mut self, message: Message) -> Result<()> {
        trace!(
            "handle_vm_message(): message.source={:?}, message.destination={:?}",
            message.source,
            message.destination
        );

        let destination: ProcessIdentifier = message.destination;

        // Parse message destination.
        if destination == ProcessIdentifier::KERNEL {
            // Forward message to daemon.

            // Check if the system daemon is connected.
            match &self.systemd_tx {
                // System daemon is connected.
                Some(systemd_tx) => {
                    if let Err(e) = systemd_tx.send(message) {
                        let reason: String =
                            format!("failed to send message to system daemon (error={:?})", e);
                        error!("handle_vm_message(): {}", reason);
                        anyhow::bail!(reason);
                    }
                },
                // System daemon is not connected.
                None => {
                    let reason: &str = "system daemon is not connected";
                    error!("handle_vm_message(): {}", reason);
                    anyhow::bail!(reason);
                },
            }
        } else {
            // Forward message to client.

            // Retrieve peer.
            let peer: GatewayPeer = self.lookup_tables.lookup_pid(destination).await?;

            // Parse peer type.
            match peer {
                // HTTP client.
                GatewayPeer::Client(client) => {
                    // Forward the message to the client.
                    if let Err(e) = client.send(Ok(message)) {
                        let reason: String =
                            format!("failed to send message to client (error={:?})", e);
                        error!("handle_vm_message(): {}", reason);
                        anyhow::bail!(reason);
                    }
                },
            }
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Spawns the system daemon handler.
    ///
    fn spawn_systemd_handler(&mut self) -> Result<()> {
        if let Some(systemd_conn) = self.systemd_conn.take() {
            let mut systemd_conn = TcpStream::from_std(systemd_conn)?;

            // Create an asynchronous channel to enable communication from the gateway to the system daemon.
            let (systemd_tx, mut systemd_rx): (SysdTx, SysdRx) =
                mpsc::unbounded_channel::<Message>();

            self.systemd_tx = Some(systemd_tx);

            let gateway_vm_tx: UnboundedSender<Message> = self.gateway_vm_tx.clone();
            tokio::task::spawn(async move {
                loop {
                    // Receive a message from the VM.
                    let message: Message = match systemd_rx.recv().await {
                        Some(message) => message,
                        None => {
                            let reason: &str = "system daemon channel has disconnected";
                            error!("failed to receive message from system daemon: {}", reason);
                            break;
                        },
                    };

                    // Forward message to the system daemon.
                    if let Err(e) = systemd_conn.write_all(&message.to_bytes()).await {
                        let reason: String =
                            format!("failed to send message to system daemon (error={:?})", e);
                        error!("spawn_systemd_handler(): {}", reason);
                        unimplemented!("send error to the VM");
                    }

                    // Wait for the system daemon to reply.
                    if let Err(e) = systemd_conn.readable().await {
                        let reason: String =
                            format!("failed to wait for system daemon reply (error={:?})", e);
                        error!("spawn_systemd_handler(): {}", reason);
                        unimplemented!("send error to the VM");
                    }

                    // Read the reply from the system daemon.
                    let mut bytes: [u8; config::kernel::IPC_MESSAGE_SIZE] =
                        [0; config::kernel::IPC_MESSAGE_SIZE];
                    match systemd_conn.read_exact(&mut bytes).await {
                        Ok(_) => {
                            let reply: Message = match Message::try_from_bytes(bytes) {
                                Ok(reply) => reply,
                                Err(e) => {
                                    let reason: String = format!(
                                        "failed to parse message from system daemon (error={:?})",
                                        e
                                    );
                                    error!("spawn_systemd_handler(): {}", reason);
                                    unimplemented!("send error to the VM");
                                },
                            };

                            // Forward the reply to the VM.
                            if let Err(e) = gateway_vm_tx.send(reply) {
                                let reason: String =
                                    format!("failed to send message to VM (error={:?})", e);
                                error!("spawn_systemd_handler(): {}", reason);
                                unimplemented!("send error to the VM");
                            }
                        },
                        Err(e) => {
                            let reason: String = format!(
                                "failed to read message from system daemon (error={:?})",
                                e
                            );
                            error!("spawn_systemd_handler(): {}", reason);
                            unimplemented!("send error to the VM");
                        },
                    }
                }

                // Close connection with the system daemon.
                if let Err(e) = systemd_conn.shutdown().await {
                    let reason: String =
                        format!("failed to shutdown system daemon connection (error={:?})", e);
                    warn!("spawn_systemd_handler(): {}", reason);
                }
            });
        }

        Ok(())
    }
}
