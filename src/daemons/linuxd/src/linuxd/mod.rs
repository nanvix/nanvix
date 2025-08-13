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
    user_vm_handle::UserVmHandle,
    venv::{
        VenvCommand,
        VirtualEnviromentDirectory,
    },
    worker_thread::WorkerThreadHandle,
};
use ::anyhow::Result;
use ::std::{
    collections::VecDeque,
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
use ::syscomm::SocketStream;

//==================================================================================================
// Constants
//==================================================================================================

const DEFAULT_CONNECTION_ID: usize = 0;

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon {
    assembler: Arc<Mutex<RequestAssembler>>,
    uvm_stream: SocketStream,
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
        Ok(Self {
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            uvm_stream,
            gateway_stream,
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
        })
    }

    pub fn run(self) -> Result<(), Error> {
        // Map keeping track of the worker threads associated the user VM.
        let mut worker_threads: VecDeque<WorkerThreadHandle> = VecDeque::new();

        // Handle around the user VM.
        let uvm_handle: UserVmHandle =
            UserVmHandle::new(DEFAULT_CONNECTION_ID, self.uvm_stream, self.gateway_stream);

        loop {
            let uvm_handle: UserVmHandle = uvm_handle.clone();
            let venv: Arc<Mutex<VirtualEnviromentDirectory>> = self.venv.clone();
            let assembler: Arc<Mutex<RequestAssembler>> = self.assembler.clone();

            // Receive a message from the user virtual machine.
            let message: Message = match Self::recv(uvm_handle.get_user_vm_stream()) {
                Ok(message) => message,

                Err(error_kind) => match error_kind {
                    ErrorKind::WouldBlock => continue,
                    ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset => {
                        info!("connection from user VM closed");

                        // Each worker thread may be in one of three states:
                        // 1. Running
                        // 2. Blocked on a system call
                        // 3. Blocked waiting for a new message from the channel
                        //
                        // To gracefully shutdown the thread, we enqueue a shutdown message to the
                        // message channel. In case the thread is blocked on a system call, we also
                        // send it an interrupt signal and handle EINTR accordingly.
                        for worker_thread in worker_threads.drain(..) {
                            trace!(
                                "sending interrupt to worker thread (thread_id={:?})",
                                worker_thread.id
                            );

                            // If any of the commands fail, continue trying to drain the remainnig
                            // threads.
                            match worker_thread.cmd_tx.send(VenvCommand::Shutdown) {
                                Ok(_) => {},
                                Err(e) => {
                                    error!(
                                        "error sending shutdown command to worker thread \
                                         (thread_id={e:?})"
                                    );
                                    continue;
                                },
                            }
                            match worker_thread.stop() {
                                Ok(_) => {},
                                Err(e) => {
                                    error!(
                                        "error sending interrupt to worker thread \
                                         (thread_id={e:?})"
                                    );
                                    continue;
                                },
                            }
                            match worker_thread.handle.join() {
                                Ok(_) => {},
                                Err(e) => {
                                    error!("error joining worker thread (thread_id={e:?})");
                                    continue;
                                },
                            }
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
            let (channel_tx, channel_rx): (Sender<VenvCommand>, Option<Receiver<VenvCommand>>) = {
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

                            let uvm_stream: Arc<Mutex<(SocketStream, VecDeque<u8>)>> =
                                uvm_handle.get_user_vm_stream();
                            let mut guard: MutexGuard<'_, (SocketStream, VecDeque<u8>)> =
                                match uvm_stream.lock() {
                                    Ok(guard) => guard,
                                    Err(e) => {
                                        error!(
                                            "error acquiring lock on user VM stream (error={e:?})"
                                        );
                                        continue;
                                    },
                                };
                            let (locked_uvm_stream, _): &mut (SocketStream, VecDeque<u8>) =
                                &mut guard;

                            locked_uvm_stream
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
                let worker_thread_handle: WorkerThreadHandle = WorkerThreadHandle::spawn(
                    source,
                    channel_rx,
                    channel_tx.clone(),
                    uvm_handle,
                    assembler,
                )?;
                worker_threads.push_back(worker_thread_handle);
            }

            // Dispatch message to worker thread.
            if let Err(error) = channel_tx.send(VenvCommand::Work(message)) {
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

    /// Read a message from the user VM stream. We need to handle the situation where we can only
    /// do a partial read, so we keep a buffer alongside our socket. It is safe to have this buffer
    /// dynamically sized, as it will always be smaller than one message size.
    fn recv(uvm_stream: Arc<Mutex<(SocketStream, VecDeque<u8>)>>) -> Result<Message, ErrorKind> {
        let mut guard: MutexGuard<'_, (SocketStream, VecDeque<u8>)> = match uvm_stream.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("error acquiring lock on user VM stream (error={e:?})");
                return Err(ErrorKind::InvalidData);
            },
        };
        let (locked_uvm_stream, partial_read_buffer): &mut (SocketStream, VecDeque<u8>) =
            &mut guard;

        let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
            [0u8; config::kernel::IPC_MESSAGE_SIZE];

        let mut num_filled = 0;
        if !partial_read_buffer.is_empty() {
            // Prepare data in buffer for partial read.
            partial_read_buffer.make_contiguous();
            let partial_bytes = partial_read_buffer.as_slices().0;

            // We take the minimum at the end just in case, but the partial read should always be
            // strictly smaller than the message size.
            let num_partial_read = partial_bytes.len().min(buf.len());

            buf[..num_partial_read].copy_from_slice(&partial_bytes[..num_partial_read]);

            // Clear partial read buffer.
            partial_read_buffer.clear();
            num_filled += num_partial_read;
        }
        // Post-condition: partial_read_buffer is empty.

        match locked_uvm_stream.try_read_exact(&mut buf[num_filled..]) {
            Ok(n) => {
                // Handle partial reads by copying all we have read to the partial read buffer and
                // returning a WouldBlock indicating that we need more data.
                if n + num_filled < buf.len() {
                    partial_read_buffer.extend(&buf[..(n + num_filled)]);
                    return Err(ErrorKind::WouldBlock);
                }
            },
            Err(e) => return Err(e.kind()),
        }

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
