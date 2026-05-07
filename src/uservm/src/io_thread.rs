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
    warn,
};
use ::std::mem;
use ::sys::ipc::{
    DataChunk,
    IkcFrame,
    Message,
};
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

/// Timestamps, serialises, and writes a single [`Message`] to the system VM socket.
/// The `frame_type` byte is prepended to the message payload in a single write to reduce
/// syscall overhead and avoid sending the frame byte as a separate tiny segment.
async fn forward_message_to_system_vm(
    msg: &mut Message,
    system_vm_tx: &mut SocketStreamWriter,
    frame_type: u8,
) -> Result<()> {
    // Label: uservm::io_thread::system_vm::write()
    profiler::timestamp_message!(
        &mut msg.payload,
        std::mem::offset_of!(syscall::SystemCallMessage, payload)
            + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
    );
    // SAFETY: `Message` derives `Clone`; cloning avoids consuming the caller's binding.
    let msg_bytes: [u8; ::std::mem::size_of::<Message>()] = msg.clone().to_bytes();
    let mut buf: [u8; 1 + ::std::mem::size_of::<Message>()] =
        [0; 1 + ::std::mem::size_of::<Message>()];
    buf[0] = frame_type;
    buf[1..].copy_from_slice(&msg_bytes);
    system_vm_tx.write_all(&buf).await.map_err(|e| {
        let reason: String = format!("failed writing message to system VM socket: {e}");
        error!("{reason}");
        anyhow::Error::msg(reason)
    })?;
    Ok(())
}

/// Wire format for data chunk transfers over the system VM socket: a 4-byte little-endian length
/// prefix (u32) followed by the serialized [`DataChunk`] payload (header + data).
const BULK_TRANSFER_LENGTH_PREFIX_SIZE: usize = mem::size_of::<u32>();

/// Serialises and writes a [`DataChunk`] to the system VM socket using a simple
/// length-prefixed framing protocol. The `frame_type` byte, length prefix, and payload are
/// coalesced into a single vectored write to reduce syscall overhead.
async fn forward_bulk_to_system_vm(
    bulk: &mut DataChunk,
    system_vm_tx: &mut SocketStreamWriter,
    frame_type: u8,
) -> Result<()> {
    // Label: uservm::io_thread::system_vm::write()
    profiler::timestamp_message!(bulk.data_mut(), 0);
    let payload: Vec<u8> = bulk.to_bytes();
    let len_prefix: [u8; BULK_TRANSFER_LENGTH_PREFIX_SIZE] =
        u32::to_le_bytes(u32::try_from(payload.len()).map_err(|e| {
            let reason: String = format!("bulk payload length exceeds u32: {e}");
            error!("{reason}");
            anyhow::Error::msg(reason)
        })?);
    // Coalesce frame type, length prefix, and payload into a single vectored write
    // to avoid an extra heap allocation and full-payload copy.
    let frame_byte: [u8; 1] = [frame_type];
    system_vm_tx
        .write_all_vectored(&mut [
            std::io::IoSlice::new(&frame_byte),
            std::io::IoSlice::new(&len_prefix),
            std::io::IoSlice::new(&payload),
        ])
        .await
        .map_err(|e| {
            let reason: String = format!("failed writing bulk transfer: {e}");
            error!("{reason}");
            anyhow::Error::msg(reason)
        })?;
    Ok(())
}

/// Forwards a [`IkcFrame`] to the system VM socket. The frame type byte is coalesced with the
/// payload into a single write to reduce syscall overhead.
async fn forward_transfer_to_system_vm(
    transfer: &mut IkcFrame,
    system_vm_tx: &mut SocketStreamWriter,
) -> Result<()> {
    let frame_type: u8 = transfer.frame_type_byte();
    match transfer {
        IkcFrame::Message(msg) => forward_message_to_system_vm(msg, system_vm_tx, frame_type).await,
        IkcFrame::Bulk(bulk) => forward_bulk_to_system_vm(bulk, system_vm_tx, frame_type).await,
    }
}

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
        data_rx: Receiver<IkcFrame>,
        data_tx: Sender<IkcFrame>,
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
        mut data_rx: Receiver<IkcFrame>,
        data_tx: Sender<IkcFrame>,
        control_tx: Sender<IoControlCommand>,
        mut control_rx: Receiver<IoControlResponse>,
        control_plane_stream: SocketStream,
        counters: MessageCounters,
    ) -> Result<()> {
        let start_instant: Instant = Instant::now();
        let mut frame_type_buf: [u8; 1] = [0u8; 1];
        let mut frame_type_buf_len: usize = 0;
        let mut msg_buf: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
        let mut msg_buf_len: usize = 0;
        let mut bulk_len_buf: [u8; 4] = [0u8; 4];
        let mut bulk_len_buf_len: usize = 0;
        let mut bulk_payload_buf: Vec<u8> = Vec::new();
        let mut bulk_payload_len: usize = 0;
        let mut bulk_expected_len: usize = 0;
        /// Inbound state machine for the framing protocol.
        #[derive(PartialEq)]
        enum InboundState {
            /// Waiting for the 1-byte frame type.
            FrameType,
            /// Accumulating a fixed-size IPC message.
            Message,
            /// Accumulating the 4-byte bulk length prefix.
            BulkLength,
            /// Accumulating the bulk payload.
            BulkPayload,
        }
        let mut inbound_state: InboundState = InboundState::FrameType;
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

                // Forward incoming transfers from system VM to user VM using framing protocol.
                result = async {
                    match inbound_state {
                        InboundState::FrameType =>
                            system_vm_rx.read(&mut frame_type_buf[frame_type_buf_len..]).await,
                        InboundState::Message =>
                            system_vm_rx.read(&mut msg_buf[msg_buf_len..]).await,
                        InboundState::BulkLength =>
                            system_vm_rx.read(&mut bulk_len_buf[bulk_len_buf_len..]).await,
                        InboundState::BulkPayload =>
                            system_vm_rx.read(&mut bulk_payload_buf[bulk_payload_len..]).await,
                    }
                } => {
                    trace!("reading transfer from system VM");
                    match result {
                        Ok(0) => {
                            let reason: String = String::from("system VM socket closed unexpectedly");
                            error!("{reason}");
                            break Err(anyhow::Error::msg(reason));
                        },
                        Ok(n) => {
                            match inbound_state {
                                InboundState::FrameType => {
                                    frame_type_buf_len += n;
                                    if frame_type_buf_len == 1 {
                                        match frame_type_buf[0] {
                                            IkcFrame::MESSAGE_FRAME => {
                                                inbound_state = InboundState::Message;
                                                msg_buf_len = 0;
                                                msg_buf.fill(0);
                                            },
                                            IkcFrame::DATA_CHUNK_FRAME => {
                                                inbound_state = InboundState::BulkLength;
                                                bulk_len_buf_len = 0;
                                                bulk_len_buf.fill(0);
                                            },
                                            unknown => {
                                                let reason: String = format!(
                                                    "unknown inbound frame type ({unknown:#04x})"
                                                );
                                                error!("{reason}");
                                                break Err(anyhow::Error::msg(reason));
                                            },
                                        }
                                        frame_type_buf_len = 0;
                                    }
                                },
                                InboundState::Message => {
                                    msg_buf_len += n;
                                    if msg_buf_len == msg_buf.len() {
                                        let mut message: Message =
                                            Message::try_from_bytes(msg_buf).map_err(|e| {
                                                let reason: String = format!(
                                                    "failed to decode message from system VM: {e:?}"
                                                );
                                                error!("{reason}");
                                                anyhow::Error::msg(reason)
                                            })?;

                                        // Label: uservm::io_thread::system_vm::read()
                                        profiler::timestamp_message!(&mut message.payload,
                                            std::mem::offset_of!(syscall::SystemCallMessage, payload)
                                                + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                                        );

                                        on_message_received_from_system_vm(&counters);
                                        data_tx.send(IkcFrame::Message(message)).await?;
                                        inbound_state = InboundState::FrameType;
                                    }
                                },
                                InboundState::BulkLength => {
                                    bulk_len_buf_len += n;
                                    if bulk_len_buf_len == 4 {
                                        bulk_expected_len =
                                            u32::from_le_bytes(bulk_len_buf) as usize;
                                        bulk_payload_buf = vec![0u8; bulk_expected_len];
                                        bulk_payload_len = 0;
                                        inbound_state = InboundState::BulkPayload;
                                    }
                                },
                                InboundState::BulkPayload => {
                                    bulk_payload_len += n;
                                    if bulk_payload_len == bulk_expected_len {
                                        let mut bulk: DataChunk =
                                            DataChunk::try_from_bytes(&bulk_payload_buf)
                                                .map_err(|e| {
                                                    let reason: String = format!(
                                                        "failed to decode data chunk transfer: {e:?}"
                                                    );
                                                    error!("{reason}");
                                                    anyhow::Error::msg(reason)
                                                })?;
                                        // Label: uservm::io_thread::system_vm::read()
                                        profiler::timestamp_message!(bulk.data_mut(), 0);
                                        on_message_received_from_system_vm(&counters);
                                        data_tx.send(IkcFrame::Bulk(bulk)).await?;
                                        inbound_state = InboundState::FrameType;
                                    }
                                },
                            }
                        },
                        Err(e) => {
                            let reason: String = format!("error reading from system VM socket: {e}");
                            error!("{reason}");
                            break Err(anyhow::Error::msg(reason));
                        },
                    }
                },

                // Forward outgoing transfers to system VM.
                result = data_rx.recv() => {
                    trace!("forwarding transfer to system VM");
                    match result {
                        Some(mut transfer) => {
                            forward_transfer_to_system_vm(&mut transfer, &mut system_vm_tx).await?;
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
                                    // Messages are no longer buffered, nothing to flush. We should remove this.
                                    if let Err(error) =
                                        control_tx.send(IoControlCommand::SystemCallFlushed).await
                                    {
                                        debug!(
                                            "control channel closed while sending flush ack: {error}"
                                        );
                                        break Ok(());
                                    }
                                },
                                IoControlResponse::FlushOutput => {
                                    // Messages are no longer buffered, nothing to flush. We should remove this.
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
                                        "shutdown received, draining outbound transfers (elapsed_ms={})",
                                        start_instant.elapsed().as_millis()
                                    );
                                    // Close the channel and drain any buffered outbound
                                    // transfers before dropping the system VM socket.
                                    data_rx.close();
                                    let mut drained: usize = 0;
                                    while let Some(mut transfer) = data_rx.recv().await {
                                        if let Err(e) = forward_transfer_to_system_vm(&mut transfer, &mut system_vm_tx).await {
                                            let remaining: usize = data_rx.len();
                                            warn!("drain aborted after {drained} transfers, {remaining} transfers dropped: {e}");
                                            break;
                                        }
                                        drained += 1;
                                    }
                                    debug!("drained {drained} outbound transfers");
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
            let (data_tx, data_rx): (mpsc::Sender<IkcFrame>, mpsc::Receiver<IkcFrame>) =
                mpsc::channel(64);
            let (inbound_tx, _inbound_rx): (mpsc::Sender<IkcFrame>, mpsc::Receiver<IkcFrame>) =
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
                data_tx
                    .send(IkcFrame::Message(msg))
                    .await
                    .expect("send message");
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
            // Each outbound message is framed as: 1-byte frame type + message bytes.
            let msg_size: usize = mem::size_of::<Message>();
            let frame_size: usize = 1 + msg_size;
            let expected_total: usize = frame_size * num_messages;
            let mut buf: Vec<u8> = vec![0u8; expected_total];
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
                total_read, expected_total,
                "expected {} bytes ({} framed messages), got {} bytes",
                expected_total, num_messages, total_read
            );

            // Verify that message content arrived intact and in order.
            for i in 0..num_messages {
                let offset: usize = i * frame_size;
                // First byte of each frame is the IkcFrame::MESSAGE_FRAME marker.
                assert_eq!(buf[offset], IkcFrame::MESSAGE_FRAME, "message {i} frame type mismatch");
                let received: Message = Message::try_from_bytes(
                    buf[offset + 1..offset + 1 + msg_size]
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
            let (tx, mut rx): (mpsc::Sender<IkcFrame>, mpsc::Receiver<IkcFrame>) =
                mpsc::channel(64);

            // Enqueue some messages, then drop the only sender.
            tx.send(IkcFrame::Message(Message::default()))
                .await
                .expect("send");
            tx.send(IkcFrame::Message(Message::default()))
                .await
                .expect("send");
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
