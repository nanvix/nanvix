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
use ::log::{
    debug,
    error,
    trace,
};
use ::std::mem;
use ::sys::ipc::Message;
use ::syscomm::{
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    WriteAll,
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
                                        "shutdown received, draining outbound messages (elapsed_ms={})",
                                        start_instant.elapsed().as_millis()
                                    );
                                    // Close the channel and drain any buffered outbound
                                    // messages before dropping the system VM socket.
                                    data_rx.close();
                                    while let Some(mut msg) = data_rx.recv().await {
                                        // Label: uservm::io_thread::system_vm::write()
                                        profiler::timestamp_message!(&mut msg.payload,
                                            std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                                + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                                        );
                                        let bytes: [u8; ::std::mem::size_of::<Message>()] = msg.to_bytes();
                                        if let Err(e) = system_vm_tx.write_all(&bytes).await {
                                            error!("failed to flush message to system VM during shutdown: {e}");
                                            break;
                                        }
                                    }
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

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::orchestrator::IoControlResponse;
    use ::std::mem;
    use ::sys::ipc::Message;
    use ::syscomm::SocketStream;
    use ::tokio::{
        net::UnixStream,
        sync::mpsc,
        task::JoinHandle,
        time::{
            Duration,
            timeout,
        },
    };

    /// Maximum time any single test is allowed to run before it is considered hung.
    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    /// Creates a pair of connected [`SocketStream`] instances backed by Unix sockets.
    fn unix_stream_pair() -> (SocketStream, SocketStream) {
        let (a, b): (UnixStream, UnixStream) =
            UnixStream::pair().expect("failed to create unix stream pair");
        (SocketStream::Unix(a), SocketStream::Unix(b))
    }

    /// Validates that when `IoControlResponse::Shutdown` is received, all messages previously
    /// buffered in `data_rx` are forwarded to the system VM socket before the I/O thread exits.
    #[tokio::test]
    async fn shutdown_drains_buffered_outbound_messages() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            // Wire up the channels.
            let (data_tx, data_rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) =
                mpsc::channel(64);
            let (inbound_tx, _inbound_rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) =
                mpsc::channel(1);
            let (ctrl_cmd_tx, _ctrl_cmd_rx): (
                mpsc::Sender<IoControlCommand>,
                mpsc::Receiver<IoControlCommand>,
            ) = mpsc::channel(1);
            let (ctrl_resp_tx, ctrl_resp_rx): (
                mpsc::Sender<IoControlResponse>,
                mpsc::Receiver<IoControlResponse>,
            ) = mpsc::channel(1);

            // System VM socket: the I/O thread writes to one end, we read from the other.
            let (system_vm_stream, system_vm_peer): (SocketStream, SocketStream) =
                unix_stream_pair();
            // Control-plane socket: unused, but required by IoThread::run().
            let (cp_stream, _cp_peer): (SocketStream, SocketStream) = unix_stream_pair();

            let counters: MessageCounters = MessageCounters::new();

            // Pre-load 3 outbound messages with distinguishable payloads.
            let num_messages: usize = 3;
            for i in 0..num_messages {
                let mut msg: Message = Message::default();
                msg.payload[0] = u8::try_from(i).expect("num_messages fits in u8");
                data_tx.send(msg).await.expect("send message");
            }
            // Drop the only sender so the channel will close after draining.
            drop(data_tx);

            // Spawn the I/O thread.
            let io_handle: JoinHandle<::anyhow::Result<()>> = IoThread::spawn(
                system_vm_stream,
                data_rx,
                inbound_tx,
                ctrl_cmd_tx,
                ctrl_resp_rx,
                cp_stream,
                counters,
            )
            .expect("spawn io thread");

            // Send Shutdown to trigger the drain path.
            ctrl_resp_tx
                .send(IoControlResponse::Shutdown)
                .await
                .expect("send shutdown");

            // Wait for the I/O thread to finish.
            let io_result: ::anyhow::Result<()> = io_handle.await.expect("join io thread");
            assert!(io_result.is_ok(), "I/O thread returned an error: {io_result:?}");

            // Read back all messages from the peer end of the system VM socket.
            let msg_size: usize = mem::size_of::<Message>();
            let mut buf: Vec<u8> = vec![0u8; msg_size * num_messages];
            let mut total_read: usize = 0;
            let (mut peer_rx, _peer_tx) = system_vm_peer.split();
            while total_read < buf.len() {
                let n: usize = peer_rx
                    .read(&mut buf[total_read..])
                    .await
                    .expect("read from peer socket");
                assert!(n > 0, "peer socket returned 0 bytes before all messages were read");
                total_read += n;
            }

            assert_eq!(
                total_read,
                msg_size * num_messages,
                "expected {} bytes ({} messages), got {} bytes",
                msg_size * num_messages,
                num_messages,
                total_read
            );

            // Verify that message content arrived intact and in order.
            for i in 0..num_messages {
                let offset: usize = i * msg_size;
                let received: Message = Message::try_from_bytes(
                    buf[offset..offset + msg_size]
                        .try_into()
                        .expect("slice len"),
                )
                .expect("decode message");
                let expected: u8 = u8::try_from(i).expect("num_messages fits in u8");
                assert_eq!(
                    received.payload[0], expected,
                    "message {i} payload mismatch: expected {expected}, got {}",
                    received.payload[0]
                );
            }

            Ok(())
        })
        .await
        .expect("test timed out — I/O thread likely hung (regression of issue #1450)");
        result.expect("test failed");
    }

    /// Validates that an `mpsc` channel's `recv()` returns `None` once the sole `Sender` is
    /// dropped, after all buffered messages have been consumed. This property is a prerequisite
    /// for the shutdown drain loop: if the channel never closes, the drain would block forever.
    #[tokio::test]
    async fn channel_closes_when_sole_sender_dropped() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (tx, mut rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel(64);

            // Enqueue some messages, then drop the only sender.
            tx.send(Message::default()).await.expect("send");
            tx.send(Message::default()).await.expect("send");
            drop(tx);

            // Drain buffered messages.
            let mut drained: usize = 0;
            while let Some(_msg) = rx.recv().await {
                drained += 1;
            }

            // recv() must have returned None after draining, proving the channel closed.
            assert_eq!(drained, 2, "expected to drain 2 buffered messages");

            Ok(())
        })
        .await
        .expect("test timed out — channel did not close (sender lifetime leak)");
        result.expect("test failed");
    }
}
