// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host-side IKC dispatch and daemon integration.
//!
//! This module is the host-side counterpart of the guest's inter-kernel
//! communication (IKC) channel. It mirrors the Nanvix `uservm` standalone I/O
//! handler, but instead of re-implementing the host-side daemons it **reuses**
//! them directly:
//!
//! - `read(2)`/`write(2)` against stdio are bridged to the host's
//!   stdin/stdout (the terminal role of standalone `nanvixd`);
//! - filesystem requests (`vfsd` → `hostfsd`) are dispatched to the real
//!   [`hostfsd::HostFsHandler`];
//! - networking requests (`→ networkd`) are dispatched to the real
//!   [`networkd::NetworkDaemon`].
//!
//! The guest emits a message by writing the guest-physical address of a
//! [`VmBusMessage`] envelope to the stdout port (`0xe9`); it fetches a queued
//! host response by writing an envelope to the stdin port (`0xea`) while the
//! shared `CREDITS` register is positive. Because the device runs on the single
//! vCPU thread, all dispatch happens synchronously here.

use crate::io::GuestIo;
use ::anyhow::Context as _;
use ::guestmem::GuestMemory;
use ::hostfs_api::{
    get_op_id,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
    HOSTFS_ERR_IO,
};
use ::std::{
    collections::VecDeque,
    path::PathBuf,
};
use ::sys::{
    ipc::{
        DataChunkHeader,
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
        VmBusMessage,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    message::SystemCallMessagePart,
    unistd::message::{
        ReadResponse,
        WriteRequest,
        WriteResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};

/// Size of a serialized IPC [`Message`] in bytes.
const MESSAGE_SIZE: usize = Message::HEADER_SIZE + Message::PAYLOAD_SIZE;

/// A host-produced frame awaiting delivery to the guest via the stdin port.
enum HostFrame {
    /// A complete IPC message to copy into the guest's message buffer.
    Message(Message),
    /// A bulk payload to write at `data_addr`, paired with the pull-completion
    /// header used to synthesize the guest's `PullResponse` notification.
    Bulk {
        data: Vec<u8>,
        data_addr: u32,
        header: DataChunkHeader,
    },
}

/// A guest stdio request awaiting its following bulk frame.
enum Pending {
    /// A `write(2)`: the next bulk frame carries the bytes to emit.
    Write { tid: ThreadIdentifier, fd: i32 },
    /// A `read(2)`: the next bulk frame is the pull header naming the guest
    /// destination buffer and maximum length.
    Read { tid: ThreadIdentifier },
}

/// Bridges guest IKC traffic to host stdio and the host-side daemons.
pub struct IkcBridge {
    /// The stdio request whose bulk frame is expected next, if any.
    pending: Option<Pending>,
    /// Frames queued for delivery to the guest.
    outbound: VecDeque<HostFrame>,
    /// Guest standard-I/O endpoint (host terminal or in-process channel).
    io: Box<dyn GuestIo>,
    /// Host filesystem daemon, present when a mount directory is configured.
    hostfs: Option<hostfsd::HostFsHandler>,
    /// Host network daemon, present when host networking is enabled.
    network: Option<networkd::NetworkDaemon>,
}

impl IkcBridge {
    /// Creates a new bridge.
    ///
    /// `io` is the guest standard-I/O endpoint (see [`crate::io`]). When
    /// `mount_directory` is set, a [`hostfsd::HostFsHandler`] rooted there serves
    /// guest filesystem requests. When `networking` is true, a
    /// [`networkd::NetworkDaemon`] serves guest networking requests.
    pub fn new(io: Box<dyn GuestIo>, mount_directory: Option<PathBuf>, networking: bool) -> Self {
        let hostfs =
            mount_directory.and_then(|dir| match hostfsd::HostFsHandler::new(dir.clone()) {
                Ok(h) => {
                    log::info!("hostfsd: serving mount root {}", dir.display());
                    Some(h)
                },
                Err(e) => {
                    log::error!("hostfsd: failed to initialize for {}: {e}", dir.display());
                    None
                },
            });

        let network = if networking {
            match networkd::NetworkDaemon::new() {
                Ok(nd) => {
                    log::info!("networkd: host networking enabled");
                    Some(nd)
                },
                Err(e) => {
                    log::error!("networkd: failed to initialize: {e:?}");
                    None
                },
            }
        } else {
            None
        };

        Self {
            pending: None,
            outbound: VecDeque::new(),
            io,
            hostfs,
            network,
        }
    }

    /// Handles a guest write to the stdout port (an outbound IKC envelope).
    pub fn guest_stdout(
        &mut self,
        gm: &GuestMemory,
        envelope: &VmBusMessage,
    ) -> anyhow::Result<()> {
        if envelope.is_ikc() {
            self.handle_message(gm, envelope.message_addr())
        } else {
            self.handle_bulk(gm, envelope.message_addr())
        }
    }

    /// Handles a guest read from the stdin port: delivers one queued frame.
    pub fn guest_stdin(&mut self, gm: &GuestMemory, envelope: &VmBusMessage) -> anyhow::Result<()> {
        let Some(frame) = self.outbound.pop_front() else {
            // The guest only fetches when CREDITS is positive, so this is not
            // expected; nothing to deliver.
            log::warn!("guest fetched IKC message with empty queue");
            return Ok(());
        };

        match frame {
            HostFrame::Message(msg) => {
                gm.write_at(u64::from(envelope.message_addr()), &msg.to_bytes())
                    .context("failed to deliver IKC message")?;
            },
            HostFrame::Bulk {
                data,
                data_addr,
                header,
            } => {
                gm.write_at(u64::from(data_addr), &data)
                    .context("failed to write bulk payload")?;
                let msg = pull_response(header);
                gm.write_at(u64::from(envelope.message_addr()), &msg.to_bytes())
                    .context("failed to deliver pull response")?;
            },
        }

        self.sync_credits(gm)
    }

    /// Parses an outbound IPC message and routes it to stdio or a daemon.
    fn handle_message(&mut self, gm: &GuestMemory, addr: u32) -> anyhow::Result<()> {
        let mut bytes = [0u8; MESSAGE_SIZE];
        gm.read_at(u64::from(addr), &mut bytes)
            .context("failed to read IKC message")?;
        let message = Message::try_from_bytes(bytes)
            .map_err(|e| anyhow::anyhow!("failed to parse IKC message: {e:?}"))?;

        let syscall_msg = match SystemCallMessage::try_from_bytes(message.payload) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("ignoring unparsable system-call message: {e:?}");
                return Ok(());
            },
        };
        let header = syscall_msg.header;
        let tid = extract_tid(message.source);

        match header {
            SystemCallMessageHeader::WriteRequest => {
                let req = WriteRequest::from_bytes(syscall_msg.payload);
                self.pending = Some(Pending::Write { tid, fd: req.fd });
            },
            SystemCallMessageHeader::ReadRequest => {
                self.pending = Some(Pending::Read { tid });
            },
            h if h.is_hostfs() => self.handle_hostfs(&message, h, &syscall_msg.payload),
            _ => {
                // Copy out of the packed message struct before comparing.
                let destination = message.destination;
                if destination == MessageReceiver::NETWORKD {
                    self.handle_networking(message);
                } else {
                    log::warn!("ignoring IKC message with unsupported header {header:?}");
                }
            },
        }
        self.sync_credits(gm)
    }

    /// Dispatches a filesystem request to the reused `hostfsd` handler.
    ///
    /// When no mount directory is configured, emits a hostfs error response so
    /// the guest's `vfsd` can drain its pending operation and report the failure
    /// to the caller instead of blocking forever (mirroring the Nanvix `uservm`
    /// standalone handler's no-mount path).
    fn handle_hostfs(
        &mut self,
        message: &Message,
        header: SystemCallMessageHeader,
        syscall_payload: &[u8; SystemCallMessage::PAYLOAD_SIZE],
    ) {
        if let Some(handler) = self.hostfs.as_mut() {
            if let Some(response) = handler.handle_request(&message.payload) {
                self.outbound
                    .push_back(HostFrame::Message(hostfs_response(response)));
                while let Some(extra) = handler.take_next_response_part() {
                    self.outbound
                        .push_back(HostFrame::Message(hostfs_response(extra)));
                }
            }
            // `None` means an intermediate multi-part request; no response yet.
            return;
        }

        if let Some(op_id) = hostfs_error_target(header, &message.payload, syscall_payload) {
            log::warn!("hostfs request received but no mount configured; sending error response");
            if let Some(err) = build_hostfs_error(header, op_id) {
                self.outbound.push_back(HostFrame::Message(err));
            }
        }
    }

    /// Dispatches a networking request to the reused `networkd` daemon.
    fn handle_networking(&mut self, message: Message) {
        let Some(daemon) = self.network.as_ref() else {
            log::warn!("networking request received but host networking is disabled");
            return;
        };
        if let Some(responses) = daemon.handle_message(message) {
            for response in responses {
                self.outbound.push_back(HostFrame::Message(response));
            }
        }
    }

    /// Processes the bulk frame following a stdio request message.
    fn handle_bulk(&mut self, gm: &GuestMemory, addr: u32) -> anyhow::Result<()> {
        let mut header_bytes = [0u8; DataChunkHeader::SIZE];
        gm.read_at(u64::from(addr), &mut header_bytes)
            .context("failed to read data chunk header")?;
        let header = DataChunkHeader::try_from_bytes(header_bytes)
            .map_err(|e| anyhow::anyhow!("failed to parse data chunk header: {e:?}"))?;

        match self.pending.take() {
            Some(Pending::Write { tid, fd }) => {
                let mut data = vec![0u8; header.data_len() as usize];
                gm.read_at(u64::from(header.data_addr()), &mut data)
                    .context("failed to read write payload")?;
                self.complete_write(tid, fd, &data);
            },
            Some(Pending::Read { tid }) => {
                self.complete_read(tid, &header);
            },
            None => log::warn!("received bulk frame without a pending stdio request"),
        }
        self.sync_credits(gm)
    }

    /// Emits write data to the guest-I/O endpoint and queues a `WriteResponse`.
    fn complete_write(&mut self, tid: ThreadIdentifier, fd: i32, data: &[u8]) {
        let written = self.io.write_stdout(fd, data);
        let response =
            WriteResponse::build(tid, written, ProcessIdentifier::KERNEL, MessageType::Ikc);
        self.outbound.push_back(HostFrame::Message(response));
    }

    /// Satisfies a guest `read(2)` from streaming host input.
    ///
    /// Blocks until host stdin has data or reaches end-of-file, matching the
    /// blocking-read semantics of the Nanvix `uservm` standalone handler. An
    /// empty result (EOF) is surfaced to the guest as a zero-length read.
    fn complete_read(&mut self, tid: ThreadIdentifier, pull_header: &DataChunkHeader) {
        let max = pull_header.data_len() as usize;
        let data: Vec<u8> = self.io.read_stdin(max);
        let actual_len = data.len() as u32;

        let response_header = DataChunkHeader::new(
            pull_header.source_pid(),
            pull_header.source_tid(),
            pull_header.destination_pid(),
            pull_header.destination_tid(),
            pull_header.data_addr(),
            actual_len,
        );
        let empty = [0u8; ReadResponse::BUFFER_SIZE];
        let response = ReadResponse::build(
            tid,
            actual_len as i32,
            empty,
            ProcessIdentifier::KERNEL,
            MessageType::Ikc,
        );

        self.outbound.push_back(HostFrame::Bulk {
            data,
            data_addr: pull_header.data_addr(),
            header: response_header,
        });
        self.outbound.push_back(HostFrame::Message(response));
    }

    /// Writes the current outbound-queue depth to the shared `CREDITS` register.
    fn sync_credits(&self, gm: &GuestMemory) -> anyhow::Result<()> {
        let credits = self.outbound.len() as u32;
        gm.write_at(config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as u64, &credits.to_le_bytes())
            .context("failed to update credits register")
    }
}

/// Wraps a `hostfsd` response payload in an IPC message addressed to `vfsd`.
fn hostfs_response(payload: [u8; Message::PAYLOAD_SIZE]) -> Message {
    Message::new(
        MessageSender::from(ProcessIdentifier::KERNEL),
        MessageReceiver::from(ProcessIdentifier::VFSD),
        MessageType::Ikc,
        None,
        payload,
    )
}

/// Returns the operation id an error response should target, or `None` if the
/// frame should be silently dropped (a non-first part of a long request).
///
/// For single-message requests the op id lives at `Message.payload[2..6]`. For
/// the multi-part "request part" frames only part 0 yields an error, carrying
/// the logical op id from the first four bytes of its chunk.
fn hostfs_error_target(
    header: SystemCallMessageHeader,
    message_payload: &[u8; Message::PAYLOAD_SIZE],
    syscall_payload: &[u8; SystemCallMessage::PAYLOAD_SIZE],
) -> Option<OperationId> {
    let is_request_part = matches!(
        header,
        SystemCallMessageHeader::HostFsOpenRequestPart
            | SystemCallMessageHeader::HostFsRenameRequestPart
            | SystemCallMessageHeader::HostFsUnlinkRequestPart
            | SystemCallMessageHeader::HostFsMkdirRequestPart
            | SystemCallMessageHeader::HostFsRmdirRequestPart
            | SystemCallMessageHeader::HostFsSymlinkRequestPart
            | SystemCallMessageHeader::HostFsReadlinkRequestPart
            | SystemCallMessageHeader::HostFsLstatRequestPart
    );

    if is_request_part {
        let part = SystemCallMessagePart::from_bytes(*syscall_payload);
        if { part.part_number } != 0 {
            return None;
        }
        let declared = { part.payload_size } as usize;
        if declared < core::mem::size_of::<OperationId>() {
            return Some(OperationId::INVALID);
        }
        let chunk = &part.payload;
        Some(OperationId::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    } else {
        Some(get_op_id(message_payload))
    }
}

/// Builds the hostfs error response message for `header`, echoing `op_id`.
///
/// Each hostfs operation detects failure differently: most check a leading
/// `i32`, `lseek` checks an `i64` offset, and `stat`/`readdir` use an all-zero
/// payload as their error/end sentinel. Returns `None` if the request header has
/// no corresponding response header.
fn build_hostfs_error(header: SystemCallMessageHeader, op_id: OperationId) -> Option<Message> {
    let resp_header = header.hostfs_response_header()?;

    let mut payload = [0u8; Message::PAYLOAD_SIZE];
    payload[0..2].copy_from_slice(&(resp_header as u16).to_ne_bytes());
    set_op_id(&mut payload, op_id);

    let ds = HOSTFS_DATA_START;
    match resp_header {
        SystemCallMessageHeader::HostFsLseekResponse => {
            payload[ds..ds + 8].copy_from_slice(&(HOSTFS_ERR_IO as i64).to_le_bytes());
        },
        SystemCallMessageHeader::HostFsStatResponse
        | SystemCallMessageHeader::HostFsReadDirResponse => {
            // All-zero payload is the error/end-of-directory sentinel.
        },
        _ => {
            payload[ds..ds + 4].copy_from_slice(&HOSTFS_ERR_IO.to_le_bytes());
        },
    }

    Some(Message::new(
        MessageSender::from(ProcessIdentifier::KERNEL),
        MessageReceiver::from(ProcessIdentifier::VFSD),
        MessageType::Ikc,
        None,
        payload,
    ))
}

/// Builds a `PullResponse` notification message wrapping a completion header.
fn pull_response(header: DataChunkHeader) -> Message {
    let mut payload = [0u8; Message::PAYLOAD_SIZE];
    payload[..DataChunkHeader::SIZE].copy_from_slice(&header.to_bytes());
    Message::new(
        MessageSender::from(ProcessIdentifier::KERNEL),
        MessageReceiver::from(ProcessIdentifier::KERNEL),
        MessageType::PullResponse,
        None,
        payload,
    )
}

/// Recovers the originating thread identifier from a message source.
fn extract_tid(source: MessageSender) -> ThreadIdentifier {
    match source.as_id() {
        Err(tid) => tid,
        Ok(_pid) => ThreadIdentifier::from(1i32),
    }
}
