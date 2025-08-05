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
    message::RequestAssembler,
    venv::VirtualEnviromentDirectory,
    worker_thread::WorkerThreadHandle,
};
use ::anyhow::Result;
use ::std::{
    collections::VecDeque,
    io::{
        ErrorKind,
        Read,
    },
    sync::{
        mpsc::{
            Receiver,
            Sender,
            RecvError,
        },
        Arc,
        Mutex,
        MutexGuard,
        mpsc,
    },
    thread::JoinHandle,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_types::c_ssize_t;
use ::syscall::{
    unistd::message::{
        ReadRequest,
        ReadResponse,
        WriteRequest,
    },
    venv::VirtualEnvironmentIdentifier,
};
use ::syscomm::SocketStream;

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon {
    assembler: Arc<Mutex<RequestAssembler>>,
    uvm_stream: Arc<Mutex<SocketStream>>,
    gateway_stream: Option<SocketStream>,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    pub fn init(
        uvm_stream: SocketStream,
        gateway_stream: Option<SocketStream>,
    ) -> Result<Self, Error> {
        if let Err(error) = uvm_stream.set_nonblocking(true) {
            let reason: &str = "failed to set UVM stream to non-blocking mode";
            error!("init(): {reason:?} (error={error:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(Self {
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            uvm_stream: Arc::new(Mutex::new(uvm_stream)),
            gateway_stream,
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        // Start the thread that will poll input from the gateway.
        // TODO: when one linuxd instance manages more than one input stream we should encapsulate
        // this logic.
        let (gw_stdin_tx, gw_stdout_tx) = if let Some(gateway_stream) = &self.gateway_stream {
            // For the STDIN channel senders (TX) need to wait for a response from the IO thread,
            // hence they send, together with the ReadRequest, the send endpoint of a channel where
            // they will wait for the response. For STDOUT senders need not to wait, hence no need
            // to also send the channel endpoint.
            let (gw_stdin_tx, gw_stdin_rx) = mpsc::channel::<(ReadRequest, Sender<Message>)>();
            let (gw_stdout_tx, gw_stdout_rx) = mpsc::channel::<WriteRequest>();

            // Make sure that the input stream from the gateway is set to blocking, as otherwise we
            // would not be able to differentiate between an EOF and a race between the application
            // code and the gateway.
            let mut gw_stdin_stream = gateway_stream
                .try_clone()
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to clone stream"))?;
            gw_stdin_stream
                .set_nonblocking(false)
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to set non-blocing socket"))?;

            let _gw_stdin_thread: JoinHandle<Result<()>> = std::thread::spawn(move || {
                loop {
                    // Block waiting for the user VM to request reading from STDIN.
                    match gw_stdin_rx.recv() {
                        Ok((_read_request, response_tx)) => {
                            let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
                            let num_read = match gw_stdin_stream
                                .read(&mut response_buf) {
                                    Ok(n) => n,
                                    Err(e) => {
                                        let reason: String = format!("failed to read STDIN from gateway: {e:?}");
                                        error!("{}", reason);
                                        return Err(anyhow::anyhow!(reason));
                                    }
                                };
                            response_tx.send(ReadResponse::build(
                                0.into(),
                                num_read as c_ssize_t,
                                response_buf))?;
                        }
                        Err(RecvError) => {
                            info!("gateway STDIN channel disconnected");
                            break Ok(());
                        }
                    }
                }
            });

            let mut gw_stdout_stream = gateway_stream
                .try_clone()
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to clone stream"))?;
            let _gw_stdout_thread: JoinHandle<Result<()>> = std::thread::spawn(move || {
                loop {
                    // Block waiting the user VM to request writing to stdout.
                    match gw_stdout_rx.recv() {
                        Ok(write_request) => {
                            gw_stdout_stream
                                .write_all(&write_request.buffer[..write_request.count as usize])?;

                            // We don't need to send anything in response of the write, as the
                            // writting thread has already moved on.
                        }
                        Err(RecvError) => {
                            let reason: String = "gateway STDOUT channel disconnected".to_string();
                            error!("{}", reason);
                            return Err(anyhow::anyhow!(reason));
                        }
                    }
                }
            });

            (Some(gw_stdin_tx), Some(gw_stdout_tx))
        } else {
            (None, None)
        };

        // Map keeping track of the worker threads associated the user VM.
        let mut worker_threads: VecDeque<WorkerThreadHandle> = VecDeque::new();

        loop {
            let uvm_stream: Arc<Mutex<SocketStream>> = self.uvm_stream.clone();
            let venv: Arc<Mutex<VirtualEnviromentDirectory>> = self.venv.clone();
            let assembler: Arc<Mutex<RequestAssembler>> = self.assembler.clone();

            // Receive a message from the user virtual machine.
            let message: Message = match Self::recv(uvm_stream.clone()) {
                Ok(message) => message,

                Err(error_kind) => match error_kind {
                    ErrorKind::WouldBlock => continue,
                    ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset => {
                        info!("connection from user VM closed");

                        for worker_thread in worker_threads.drain(..) {
                            trace!("sending interrupt to worker thread (thread_id={:?})", worker_thread.id);
                            worker_thread.stop()?;
                            worker_thread.handle.join()
                                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to join worker thread"))?;
                            log::warn!("done!");
                        }

                        break;
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
                    unimplemented!("received message from process {pid:?} instead of thread");
                },
            };

            // Check if process is associated with a virtual environment.
            let (channel_tx, channel_rx): (Sender<Message>, Option<Receiver<Message>>) = {
                let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> = venv.lock().unwrap();
                let env = venv.get(source);
                if let Some(env) = env {
                    (env.get_channel_tx(), None)
                } else {
                    // Join a new virtual environment.
                    match venv.join(source, VirtualEnvironmentIdentifier::NEW) {
                        Ok((_, channel_tx, channel_rx)) => (channel_tx, Some(channel_rx)),
                        Err(error) => {
                            warn!("failed to join new virtual environment (error={error:?})");
                            let message: Message = crate::build_error(source, error.code);
                            uvm_stream
                                .lock()
                                .unwrap()
                                .write_all(&message.to_bytes())
                                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to write to user VM stream"))?;

                            continue;
                        },
                    }
                }
            };

            // Spawn a new worker thread, if necessary.
            if let Some(channel_rx) = channel_rx {
                // Spawn a thread to handle the message.
                let venv: Arc<Mutex<VirtualEnviromentDirectory>> = venv.clone();
                let gw_stdin_tx = gw_stdin_tx.clone();
                let gw_stdout_tx = gw_stdout_tx.clone();
                let assembler = assembler.clone();

                // Spawn an interruptible thread to handle the message.
                let worker_thread_handle =
                    WorkerThreadHandle::spawn(source, channel_rx, uvm_stream, gw_stdin_tx, gw_stdout_tx, venv, assembler)?;
                worker_threads.push_back(worker_thread_handle);
            }

            // Dispatch message to worker thread.
            if let Err(error) = channel_tx.send(message) {
                error!(
                    "run(): failed to dispatch message to worker thread (tid={source:?}, \
                     error={error:?})"
                );
                // Remove thread from the virtual environment.
                let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> = venv.lock().unwrap();
                if let Err(error) = venv.leave(source) {
                    warn!(
                        "run(): failed to remove thread from virtual environment (tid={source:?}, \
                         error={error:?})",
                    );
                }
            }
        }

        // TODO: https://github.com/nanvix/nanvix/issues/639
        Ok(())
    }

    // Read a message from the user VM stream.
    fn recv(uvm_stream: Arc<Mutex<SocketStream>>) -> Result<Message, ErrorKind> {
        let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
            [0u8; config::kernel::IPC_MESSAGE_SIZE];

        let mut locked_uvm_stream: MutexGuard<'_, SocketStream> = uvm_stream.lock().unwrap();

        if let Err(e) = locked_uvm_stream.read_exact(&mut buf) {
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
