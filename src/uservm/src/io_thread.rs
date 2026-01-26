// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    counters::MessageCounters,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
    },
};
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::std::mem;
use ::sys::ipc::Message;
use ::syscomm::{
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    WriteAll,
};
use ::syslog::{
    debug,
    error,
    trace,
};
use ::tokio::{
    select,
    sync::mpsc::{
        Receiver,
        Sender,
    },
    task::JoinHandle,
    time::Instant,
};

//==================================================================================================
// Implementations
//==================================================================================================

/// Event loop responsible for bridging the system VM, control-plane, and guest channels.
pub struct IoThread;

impl IoThread {
    ///
    /// # Description
    ///
    /// Spawns a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `system_vm_stream`: Connection to the system VM.
    /// - `data_rx`: User VM receiver.
    /// - `data_tx`: User VM sender.
    /// - `control_tx`: Command sender.
    /// - `control_rx`: Response receiver.
    /// - `control_plane_stream`: Connection to the control-plane.
    /// - `counters`: Shared counters for tracking message flow across threads.
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    pub fn spawn(
        system_vm_stream: SocketStream,
        data_rx: Receiver<Message>,
        data_tx: Sender<Message>,
        control_tx: Sender<IoControlCommand>,
        control_rx: Receiver<IoControlResponse>,
        control_plane_stream: SocketStream,
        counters: MessageCounters,
    ) -> Result<JoinHandle<Result<()>>> {
        trace!("spawn()");
        let handle: JoinHandle<Result<()>> = tokio::spawn(async move {
            Self::run(
                system_vm_stream,
                data_rx,
                data_tx,
                control_tx,
                control_rx,
                control_plane_stream,
                counters,
            )
            .await
        });
        Ok(handle)
    }

    ///
    /// # Description
    ///
    /// Runs the I/O thread.
    ///
    /// # Parameters
    ///
    /// - `system_vm_stream`: Connection to the system VM.
    /// - `data_rx`: User VM receiver.
    /// - `data_tx`: User VM sender.
    /// - `control_tx`: Command sender.
    /// - `control_rx`: Response receiver.
    /// - `control_plane_stream`: Connection to the control-plane.
    /// - `counters`: Shared counters for tracking message flow across threads.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    async fn run(
        system_vm_stream: SocketStream,
        mut data_rx: Receiver<Message>,
        data_tx: Sender<Message>,
        control_tx: Sender<IoControlCommand>,
        mut control_rx: Receiver<IoControlResponse>,
        control_plane_stream: SocketStream,
        counters: MessageCounters,
    ) -> Result<()> {
        let start_instant: Instant = Instant::now();
        let mut buf: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
        let mut buf_len: usize = 0;
        let mut control_plane_buf: [u8; ::std::mem::size_of::<NanvixdControlMessage>()] =
            [0; ::std::mem::size_of::<NanvixdControlMessage>()];
        let mut control_plane_buf_len: usize = 0;

        // Split system VM stream so that we can evolve independent reader/writer tasks.
        let (mut system_vm_rx, mut system_vm_tx): (SocketStreamReader, SocketStreamWriter) =
            system_vm_stream.split();
        // Split control-plane stream so that we can evolve independent reader/writer tasks.
        let (mut control_plane_rx, mut _control_plane_tx): (
            SocketStreamReader,
            SocketStreamWriter,
        ) = control_plane_stream.split();

        loop {
            select! {

                // Forward incoming messages to User VM.
                result = system_vm_rx.read(&mut buf[buf_len..]) => {
                    trace!("reading message from system VM");
                    match result {
                        Ok(0) => {
                            let reason: String = String::from("system VM socket closed unexpectedly");
                            error!("{reason}");
                            break Err(anyhow::Error::msg(reason));
                        },
                        Ok(n) => {
                            buf_len += n;

                            if buf_len == buf.len() {
                                // Convert bytes to message.
                                let mut message: Message =
                                    Message::try_from_bytes(buf).map_err(|e| {
                                        let reason: String =
                                            format!("failed to decode message from system VM: {e:?}");
                                        error!("{reason}");
                                        anyhow::Error::msg(reason)
                                    })?;

                                // Label: uservm::io_thread::system_vm::read()
                                profiler::timestamp_message!(&mut message.payload,
                                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                        + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                                );

                                buf.fill(0);
                                buf_len = 0;

                                on_message_received_from_system_vm(&counters);

                                // Push message to incoming queue.
                                data_tx.send(message).await?;
                            }
                        },
                        Err(e) => {
                            let reason: String = format!("error reading from system VM socket: {e}");
                            error!("{reason}");
                            break Err(anyhow::Error::msg(reason));
                        },
                    }
                },

                // Forward outgoing messages to system VM.
                result = data_rx.recv() => {
                    trace!("forwarding message to system VM");
                    match result {
                        Some(mut msg) => {
                            // Label: uservm::io_thread::system_vm::write()
                            profiler::timestamp_message!(&mut msg.payload,
                                std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                    + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                            );

                            let bytes: [u8; ::std::mem::size_of::<Message>()] = msg.to_bytes();
                            system_vm_tx.write_all(&bytes).await.map_err(|e| {
                                let reason: String = format!("failed writing message to system VM socket: {e}");
                                error!("{reason}");
                                anyhow::Error::msg(reason)
                            })?;
                        },
                        None => {
                            debug!(
                                "User VM channel closed, exiting I/O thread (elapsed_ms={})",
                                start_instant.elapsed().as_millis()
                            );
                            break Ok(());
                        },
                    }
                }

                result = control_plane_rx.read(&mut control_plane_buf[control_plane_buf_len..]) => {
                    match result {
                        Ok(0) => {
                            let reason: &'static str = "failed reading command from control-plane";
                            error!("{reason}: connection closed");
                            break Err(anyhow::Error::msg(reason));
                        },
                        Ok(n) => {
                            control_plane_buf_len += n;

                            if control_plane_buf_len == control_plane_buf.len() {
                                let msg: NanvixdControlMessage = NanvixdControlMessage::try_from_bytes(&control_plane_buf).map_err(|e| {
                                    let reason: &'static str = "failed parsing command from control-plane";
                                    error!("{reason}: {e:?}");
                                    anyhow::Error::msg(reason)
                                })?;

                                control_plane_buf.fill(0);
                                control_plane_buf_len = 0;

                                match msg.cmd() {
                                    NanvixdCommand::Shutdown => {
                                        if let Err(error) =
                                            control_tx.send(IoControlCommand::Shutdown).await
                                        {
                                            debug!(
                                                "control channel closed while relaying shutdown: {error} (elapsed_ms={})",
                                                start_instant.elapsed().as_millis()
                                            );
                                            break Ok(());
                                        }
                                        debug!(
                                            "received shutdown from control-plane (elapsed_ms={})",
                                            start_instant.elapsed().as_millis()
                                        );
                                    },
                                }
                            }
                        },
                        Err(e) => {
                            let reason: &'static str = "failed reading command from control-plane";
                            error!("{reason}: {e}");
                            break Err(anyhow::Error::msg(reason));
                        },
                    }
                },

                result = control_rx.recv() => {
                    match result {
                        Some(response) => {
                            match response {
                                IoControlResponse::FlushInput => {
                                    debug!("input flush completed");
                                    // Messages are no longer buffer, nothing to flush. We should remove this.
                                    if let Err(error) =
                                        control_tx.send(IoControlCommand::LinuxDaemonFlushed).await
                                    {
                                        debug!(
                                            "control channel closed while sending flush ack: {error}"
                                        );
                                        break Ok(());
                                    }
                                },
                                IoControlResponse::FlushOutput => {
                                    // Messages are no longer buffer, nothing to flush. We should remove this.
                                    debug!("output flush completed");
                                },
                                IoControlResponse::MicroVmPaused => {
                                    debug!("microvm pause acknowledged");
                                },
                                IoControlResponse::SnapshotCreated => {
                                    debug!("snapshot created");
                                },
                                IoControlResponse::Shutdown => {
                                    debug!(
                                        "shutdown completed (elapsed_ms={})",
                                        start_instant.elapsed().as_millis()
                                    );
                                    break Ok(());
                                },
                            }
                        },
                        None => {
                            error!(
                                "VMM control channel closed unexpectedly (elapsed_ms={})",
                                start_instant.elapsed().as_millis()
                            );
                            break Err(anyhow::Error::msg("VMM control channel closed unexpectedly"));
                        },
                    }
                }
            }
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Handler to be called whenever a message is received from the system VM.
///
/// # Parameters
///
/// - `counters`: Shared counters for tracking message flow across threads.
///
fn on_message_received_from_system_vm(counters: &MessageCounters) {
    counters.increment_io_thread_messages_received();
}
