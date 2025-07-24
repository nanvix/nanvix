// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod assemble;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    gateway::{
        GatewayCommand,
        GatewayHandle,
        GatewayPollThread,
    },
    message::RequestAssembler,
    user_vm_handle::UserVmHandle,
    venv::{
        VenvCommand,
        VirtualEnviromentDirectory,
    },
    worker_thread::WorkerThreadHandle,
};
use ::anyhow::Result;
use ::mio::{
    Events,
    Interest,
    Poll,
    Token,
};
use ::nanvixd::control_plane;
use ::slab_external::Slab;
use ::std::{
    collections::{
        HashMap,
        VecDeque,
    },
    io::ErrorKind,
    sync::{
        mpsc::{
            Receiver,
            Sender,
        },
        Arc,
        Mutex,
        MutexGuard,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::syscall::venv::VirtualEnvironmentIdentifier;
use ::syscomm::{
    SocketListener,
    SocketStream,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon {
    // We guard with a mutex the members  that will be accessed by all worker threads. The
    // other members will only be accessed by the main thread.
    control_plane_socket: SocketStream,
    user_vm_listener: SocketListener,
    gateway_listener: Option<SocketListener>,
    assembler: Arc<Mutex<RequestAssembler>>,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    pub fn init(
        control_plane_socket: SocketStream,
        user_vm_listener: SocketListener,
        gateway_listener: Option<SocketListener>,
    ) -> Result<Self, Error> {
        Ok(Self {
            control_plane_socket,
            user_vm_listener,
            gateway_listener,
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
        })
    }

    /// This helper method accepts connections into the main user VM listener socket, and, if
    /// necessary, accepts incoming connections for the gateway into this user VM.
    fn accept_connections(
        &mut self,
        user_vm_connections: &mut Slab<UserVmHandle>,
        poll: &Poll,
        start_token: usize,
        gw_handle: &Option<GatewayHandle>,
    ) -> Result<(), Error> {
        // Accept new connection in a loop, as we have a non-blocking socket, and
        // we may have more than one connection pending to be accepted.
        loop {
            match self.user_vm_listener.accept() {
                Ok(mut user_vm_stream) => {
                    let entry = user_vm_connections.vacant_entry();
                    let entry_key = entry.key();
                    let token = Token(start_token + entry_key);

                    poll.registry()
                        .register(
                            &mut user_vm_stream,
                            token,
                            Interest::READABLE | Interest::WRITABLE,
                        )
                        .map_err(|_| {
                            Error::new(ErrorCode::IoErr, "failed to register new user VM to poll")
                        })?;

                    // After accepting a connection from the user VM, accept a connection from the
                    // gateway if necessary.
                    //
                    // TODO: we should make this step more flexible to:
                    // 1. Avoid race conditions when multiple user VMs connect at the same time.
                    // 2. Support the situation where some user VMs may not have a gateway and
                    //    others do (within the same linuxd instance).
                    // 3. Support the situation where user VMs use a gateway lazily.
                    //
                    // A possible solution to 1 is for the user VM to send its ID right after connecting to
                    // linuxd, and we provision a wrapper around netcat that connects to the gateway with
                    // the same id.
                    let gw_stream: Option<SocketStream> = if let Some(gw_handle) = gw_handle {
                        gw_handle
                            .gw_cmd_tx
                            .send(GatewayCommand::AcceptConn)
                            .map_err(|_| {
                                Error::new(ErrorCode::IoErr, "failed to send message to GW thread")
                            })?;
                        gw_handle.waker.wake().map_err(|_| {
                            Error::new(ErrorCode::IoErr, "failed to wake GW thread")
                        })?;
                        Some(gw_handle.gw_conn_rx.recv().map_err(|_| {
                            Error::new(ErrorCode::IoErr, "failed to read response from GW thread")
                        })?)
                    } else {
                        None
                    };
                    if gw_stream.is_some() {
                        debug!("linuxd accepted gateway connection for user VM");
                    }

                    entry.insert(UserVmHandle::new(entry_key, user_vm_stream, gw_stream).map_err(
                        |_| Error::new(ErrorCode::IoErr, "failed to insert user VM handle to slab"),
                    )?);
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connections to be accepted, break.
                    break;
                },
                Err(e) => {
                    // This is a fatal error for the user VM, but we don't want to
                    // kill linuxd.
                    error!("Error accepting connection from user VM: {e:?}");
                    break;
                },
            }
        }

        Ok(())
    }

    /// Wrapper around the extraction of the connection from the slab, as we need a mutable
    /// reference, and with the function call we ensure we return the borrow.
    fn process_connection(
        &mut self,
        user_vm_connections: &mut Slab<UserVmHandle>,
        conn_id: usize,
    ) -> Result<UserVmHandle, Error> {
        match user_vm_connections.get_mut(conn_id) {
            Some(user_vm_handle) => Ok(user_vm_handle.clone()),
            None => Err(Error::new(ErrorCode::InvalidArgument, "Invalid connection ID")),
        }
    }

    /// Helper method to close a connection to a user VM identified by the connection id. Closing
    /// the connection also involes stopping all associated worker threads.
    fn close_connection(
        &mut self,
        uvm_handle: UserVmHandle,
        poll: &Poll,
        worker_threads: Option<VecDeque<WorkerThreadHandle>>,
    ) {
        // De-register the user VM socket from the poll structure.
        let user_vm_stream = uvm_handle.get_user_vm_stream();
        let mut user_vm_stream = user_vm_stream.lock().unwrap();
        if let Err(e) = poll.registry().deregister(&mut *user_vm_stream) {
            error!("failed to de-revister user VM from poll (error={e:?})");
        }

        // Send a shutdown message to all worker threads associated
        // with this user VM.
        if let Some(mut worker_threads) = worker_threads {
            for worker_thread in worker_threads.drain(..) {
                trace!("sending interrupt to worker thread (thread_id={:?})", worker_thread.id);

                // If any of the commands fail, continue trying to drain the remainnig
                // threads.
                match worker_thread.cmd_tx.send(VenvCommand::Shutdown) {
                    Ok(_) => {},
                    Err(e) => {
                        error!("error sending shutdown command to worker thread (thread_id={e:?})");
                        continue;
                    },
                }
                match worker_thread.stop() {
                    Ok(_) => {},
                    Err(e) => {
                        error!("error sending interrupt to worker thread (thread_id={e:?})");
                        continue;
                    },
                }
                match worker_thread.handle.join() {
                    Ok(_) => {},
                    Err(e) => {
                        error!("error joining worker thread (thread_id={e:?})");
                    },
                }
            }
        }
    }

    /// This is the main run loop for linuxd. It uses a listener socket to
    /// accept connections from multiple user VMs, and it polls over all
    /// active connections to serve requests.
    pub fn run(&mut self) -> Result<(), Error> {
        // We keep a slab of tokens to active connections from user VMs. We
        // reserve the first token for the main listener socket, that should
        // out-live user VMs.
        //
        // We use a slab because it is better suited to index a highly-dense
        // collection with usize keys.
        const LISTENER_TOKEN: Token = Token(0);
        // The main poll uses token number 1 to monitor commands from the control plane, and the
        // poll in the gateway thread uses token number 1 to wake-up to respond to commands from
        // the main linuxd thread.
        const CONTROL_PLANE_TOKEN: Token = Token(1);
        const WAKER_TOKEN: Token = Token(1);
        const START_TOKEN: usize = 2;
        let mut user_vm_connections: Slab<UserVmHandle> = Slab::new();

        let mut poll =
            Poll::new().map_err(|_| Error::new(ErrorCode::IoErr, "failed to create Poll"))?;
        let mut events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);
        poll.registry()
            .register(&mut self.user_vm_listener, LISTENER_TOKEN, Interest::READABLE)
            .map_err(|_| {
                Error::new(ErrorCode::IoErr, "failed to register user VM listener to poll")
            })?;
        poll.registry()
            .register(&mut self.control_plane_socket, CONTROL_PLANE_TOKEN, Interest::READABLE)
            .map_err(|_| {
                Error::new(ErrorCode::IoErr, "failed to register user VM listener to poll")
            })?;

        // Start a gateway reactor thread that polls the gateway listener socket for incoming
        // connections to user VMs.
        let gw_thread_handle: Option<GatewayHandle> = if let Some(gateway_listener) =
            self.gateway_listener.take()
        {
            Some(
                GatewayPollThread::spawn(gateway_listener, LISTENER_TOKEN, WAKER_TOKEN)
                    .map_err(|_| Error::new(ErrorCode::IoErr, "failed to spawn gateway reactor"))?,
            )
        } else {
            None
        };

        // Map keeping track of the worker threads associated to each user VM identified by
        // connection ID. We use a HashMap and not a Slab because we need to support insert/removal
        // by key.
        let mut worker_threads: HashMap<usize, VecDeque<WorkerThreadHandle>> = HashMap::new();

        'main_loop: loop {
            let venv: Arc<Mutex<VirtualEnviromentDirectory>> = self.venv.clone();
            let assembler: Arc<Mutex<RequestAssembler>> = self.assembler.clone();

            poll.poll(&mut events, None)
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to poll user VM events"))?;

            for event in events.iter() {
                match event.token() {
                    // Process control-plane messages before anything else.
                    CONTROL_PLANE_TOKEN => {
                        let cmd: control_plane::Command =
                            control_plane::try_read_command(&mut self.control_plane_socket)
                                .map_err(|_| {
                                    Error::new(
                                        ErrorCode::IoErr,
                                        "failed read command from control-plane",
                                    )
                                })?;
                        match cmd {
                            control_plane::Command::Shutdown => {
                                info!("linuxd received shutdown message from control-plane");
                                // Close all existing connections to user VMs.
                                while let Some(uvm_handle) = user_vm_connections.drain().next() {
                                    let conn_id = uvm_handle.get_conn_id();
                                    info!("shutting down user VM (conn_id={conn_id})");

                                    self.close_connection(
                                        uvm_handle,
                                        &poll,
                                        worker_threads.remove(&conn_id),
                                    );
                                }

                                // Draining the user VM connections should also drain the worker
                                // threads. Print an error if not.
                                if !worker_threads.is_empty() {
                                    error!("finished shutdown with orphaned worker threads");
                                }

                                break 'main_loop;
                            },
                        }
                    },

                    // Check if we have received any messages on the main listener socket.
                    // These indicate new user VMs connecting to linuxd.
                    LISTENER_TOKEN => {
                        self.accept_connections(
                            &mut user_vm_connections,
                            &poll,
                            START_TOKEN,
                            &gw_thread_handle,
                        )?;
                    },

                    // Now we process events from active connections.
                    Token(t) => {
                        let conn_id = t - START_TOKEN;

                        // Receive a message from the user virtual machine.
                        let uvm_handle =
                            self.process_connection(&mut user_vm_connections, conn_id)?;
                        let message: Message = match Self::recv(uvm_handle.get_user_vm_stream()) {
                            Ok(message) => message,

                            Err(error_kind) => match error_kind {
                                ErrorKind::WouldBlock => continue,
                                ErrorKind::UnexpectedEof => {
                                    info!("connection from user VM closed (conn_id={conn_id})");
                                    self.close_connection(
                                        uvm_handle,
                                        &poll,
                                        worker_threads.remove(&conn_id.clone()),
                                    );
                                    user_vm_connections.remove(conn_id);
                                    continue;
                                },
                                _ => {
                                    let reason: String =
                                        format!("failed to read message (error={error_kind:?})");
                                    unimplemented!("handle: {reason}");
                                },
                            },
                        };

                        trace!(
                            "message.source={:?}, message.destination={:?}, message.type={:?}",
                            { message.source },
                            { message.destination },
                            message.message_type,
                        );

                        let source: ThreadIdentifier = match { message.source }.as_id() {
                            Err(tid) => tid,
                            Ok(pid) => {
                                unimplemented!(
                                    "received message from process {pid:?} instead of thread"
                                );
                            },
                        };

                        // Check if process is associated with a virtual environment.
                        let (channel_tx, channel_rx): (
                            Sender<VenvCommand>,
                            Option<Receiver<VenvCommand>>,
                        ) = {
                            let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> =
                                venv.lock().unwrap();
                            let env = venv.get(source);
                            if let Some(env) = env {
                                (env.get_channel_tx(), None)
                            } else {
                                // Join a new virtual environment.
                                match venv.join(source, VirtualEnvironmentIdentifier::NEW) {
                                    Ok((_, channel_tx, channel_rx)) => {
                                        (channel_tx, Some(channel_rx))
                                    },
                                    Err(error) => {
                                        warn!(
                                            "failed to join new virtual environment \
                                             (error={error:?})"
                                        );
                                        let message: Message =
                                            crate::build_error(source, error.code);
                                        uvm_handle
                                            .get_user_vm_stream()
                                            .lock()
                                            .unwrap()
                                            .write_all(&message.to_bytes())
                                            .map_err(|_| {
                                                let reason = "failed to write to user VM stream";
                                                error!("{reason}");
                                                Error::new(ErrorCode::IoErr, reason)
                                            })?;
                                        continue;
                                    },
                                }
                            }
                        };

                        // Spawn a new worker thread, if necessary.
                        if let Some(channel_rx) = channel_rx {
                            // Spawn a thread to handle the message.
                            let assembler = assembler.clone();

                            // Spawn an interruptible thread to handle the message.
                            let worker_thread_handle: WorkerThreadHandle =
                                WorkerThreadHandle::spawn(
                                    source,
                                    channel_rx,
                                    channel_tx.clone(),
                                    uvm_handle,
                                    assembler,
                                )?;
                            worker_threads
                                .entry(conn_id)
                                .or_default()
                                .push_back(worker_thread_handle);
                        }

                        // Dispatch message to worker thread.
                        if let Err(error) = channel_tx.send(VenvCommand::Work(message)) {
                            error!(
                                "run(): failed to dispatch message to worker thread \
                                 (tid={source:?}, error={error:?})"
                            );
                            // Remove thread from the virtual environment.
                            let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> =
                                venv.lock().unwrap();
                            if let Err(error) = venv.leave(source) {
                                warn!(
                                    "run(): failed to remove thread from virtual environment \
                                     (tid={source:?}, error={error:?})",
                                );
                            }
                        }
                    },
                }
            }
        }

        // Stop the gateway thread.
        if let Some(gw_thread_handle) = gw_thread_handle {
            // Send the shutdown command and wake the thread. Log errors but do not fail as we are
            // cleaning up.
            if let Err(e) = gw_thread_handle.gw_cmd_tx.send(GatewayCommand::Shutdown) {
                error!("failed to send shutdown command to gateway thread (error={e:?})");
            }
            if let Err(e) = gw_thread_handle.waker.wake() {
                error!("failed to wake gateway thread (error={e:?})");
            }
            if let Err(e) = gw_thread_handle.gw_thread.join() {
                error!("failed to join gateway thread (error={e:?})");
            }
        }

        Ok(())
    }

    // Read a message from the user VM stream.
    fn recv(uvm_stream: Arc<Mutex<SocketStream>>) -> Result<Message, ErrorKind> {
        let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
            [0u8; config::kernel::IPC_MESSAGE_SIZE];

        let mut locked_uvm_stream: MutexGuard<'_, SocketStream> = uvm_stream.lock().unwrap();

        // If we are trying to read from the user VM stream, it means we have been notified in the
        // poll structure, so data should be available.
        if let Err(e) = locked_uvm_stream.try_read_exact(&mut buf) {
            return Err(e.kind());
        };

        let message: Message = match Message::try_from_bytes(buf) {
            Ok(message) => message,
            Err(e) => {
                let reason: String = format!("failed to parse message (error={e:?})");
                unimplemented!("handle: {}", reason);
            },
        };

        Ok(message)
    }
}
