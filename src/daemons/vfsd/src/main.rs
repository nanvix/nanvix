// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

mod assembler;
mod error;
mod handler;
mod hostfs;
mod init;
mod ipc;
mod pending;

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;

use ::proc::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupResponseMessage,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::ProcessIdentifier,
};
use ::syscall::SystemCallMessageHeader;
use alloc::collections::{
    BTreeMap,
    VecDeque,
};

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() {
    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(e) => panic!("failed to get pid (error={:?})", e),
    };
    let myname: &str = ::config::daemons::VFSD_NAME;

    ::syslog::info!("running virtual file system daemon (pid={:?})...", mypid);

    // Initialize VFS and mount the RAMFS image.
    init::vfs_init_ramfs();

    // Signup to the process manager daemon.
    // Because the kernel spawns all multibinary processes concurrently, user processes may send
    // IPC messages to vfsd before vfsd finishes its signup handshake with procd. We handle
    // this by buffering any IPC messages received during the signup phase.
    let mut buffered_messages: VecDeque<Message> = VecDeque::new();
    {
        let message: Message =
            ::proc::signup_request(mypid, myname).expect("failed to build signup request");
        ::sys::kcall::ipc::__kcall_send(&message).expect("failed to send signup request");

        // Wait for the signup response, buffering any interleaved IPC messages.
        loop {
            let message: Message =
                ::sys::kcall::ipc::__kcall_recv().expect("failed to receive signup response");
            if message.message_type == MessageType::Ipc {
                // Try to parse as a signup response.
                if let Ok(sys_msg) = SystemMessage::from_bytes(message.payload) {
                    if matches!(sys_msg.header, SystemMessageHeader::ProcessManagement) {
                        if let Ok(pm_msg) = ProcessManagementMessage::from_bytes(sys_msg.payload) {
                            if matches!(
                                pm_msg.header,
                                ProcessManagementMessageHeader::SignupResponse
                            ) {
                                let resp = SignupResponseMessage::from_bytes(pm_msg.payload);
                                let status = resp.status;
                                if status != 0 {
                                    panic!("signup failed (status={})", status);
                                }
                                ::syslog::info!("signed up with procd");
                                break;
                            }
                        }
                    }
                }
                // Not a signup response — buffer it for later processing.
                ::syslog::info!("buffered IPC message during signup");
                buffered_messages.push_back(message);
            } else {
                // Non-IPC messages during signup are discarded.
                ::syslog::error!(
                    "discarding non-IPC message during signup (type={:?})",
                    message.message_type
                );
            }
        }
    }

    // Multi-part request assembler map keyed by (tid_value, header_discriminant).
    // TODO: add eviction/timeout for incomplete entries to prevent memory leaks from crashed clients.
    let mut assemblers: BTreeMap<(i32, u16), assembler::AssemblerEntry> = BTreeMap::new();

    // Pending hostfs operations awaiting IKC responses.
    let mut pending: pending::PendingQueue = pending::PendingQueue::new();

    // In-flight multi-part hostfs *response* assembler, paired with the op_id
    // extracted eagerly from part 0 so a discarded stream can be cancelled (the
    // pending op would otherwise sit until the eviction timer fires).
    //
    // Long-target `readlink` returns its response as a stream of
    // `HostFsReadlinkResponsePart` messages. Because hostfsd's worker is single-
    // threaded and replies to one request at a time, at most one such stream is in
    // flight at any moment, so a single slot suffices. If a fresh `part_number == 0`
    // arrives while another stream is still being assembled, the old stream is
    // discarded and its caller is failed with `IoErr`.
    //
    // TODO: if hostfsd ever becomes multi-stream (e.g., a worker pool that interleaves
    // long responses), replace this single-slot assembler with an op_id-keyed map so
    // concurrent response streams can be assembled independently.
    let mut readlink_response_asm: Option<(
        ::syscall::message::SystemCallLongMessage,
        ::hostfs_api::OperationId,
    )> = None;

    // Process any messages that were buffered during the signup phase.
    while let Some(message) = buffered_messages.pop_front() {
        match ipc::handle_ipc_message(message, &mut assemblers, &mut pending) {
            Ok(true) => {
                let e = ::sys::kcall::pm::__kcall_exit(0);
                ::syslog::error!("failed to shutdown vfsd (error={:?})", e);
                loop {
                    ::core::hint::spin_loop();
                }
            },
            Ok(false) => continue,
            Err(e) => ::syslog::error!("failed to handle buffered ipc request (error={:?})", e),
        }
    }

    // Single event loop: processes both IPC messages from guest apps and IKC responses
    // from hostfsd without nesting or blocking waits.
    loop {
        match ::sys::kcall::ipc::__kcall_recv() {
            Ok(message) => match message.message_type {
                MessageType::Ipc => {
                    match ipc::handle_ipc_message(message, &mut assemblers, &mut pending) {
                        Ok(true) => break,
                        Ok(false) => {},
                        Err(e) => {
                            ::syslog::error!("failed to handle ipc request (error={:?})", e)
                        },
                    }
                },
                MessageType::Ikc => {
                    // Check if this is a hostfs response for a pending operation.
                    if let Ok(syscall_msg) =
                        ::syscall::SystemCallMessage::try_from_bytes(message.payload)
                    {
                        let header: SystemCallMessageHeader = syscall_msg.header;
                        // Multi-part response stream: assemble parts before dispatch.
                        // The op_id is *not* at the standard payload[2..6] offset for
                        // these messages (those bytes carry SystemCallMessagePart
                        // framing) — it lives in the first 4 bytes of the assembled
                        // body instead.
                        if header == SystemCallMessageHeader::HostFsReadlinkResponsePart {
                            let part: ::syscall::message::SystemCallMessagePart =
                                ::syscall::message::SystemCallMessagePart::from_bytes(
                                    syscall_msg.payload,
                                );
                            // A fresh stream starts at part 0.
                            if part.part_number == 0 {
                                if part.payload_size < 4 {
                                    ::syslog::error!(
                                        "readlink response part 0 too short to carry op_id \
                                         (payload_size={})",
                                        part.payload_size
                                    );
                                    continue;
                                }
                                let op_id: ::hostfs_api::OperationId =
                                    ::hostfs_api::OperationId::from_le_bytes([
                                        part.payload[0],
                                        part.payload[1],
                                        part.payload[2],
                                        part.payload[3],
                                    ]);
                                if let Some((_, stale_op_id)) = readlink_response_asm.take() {
                                    ::syslog::warn!(
                                        "discarding incomplete readlink response stream on new \
                                         part-0 arrival (cancelling stale op_id={})",
                                        stale_op_id
                                    );
                                    if let Some(op) = pending.remove(stale_op_id) {
                                        pending::cancel_pending_op(op, ErrorCode::IoErr);
                                    }
                                }
                                let capacity: usize = part.total_parts.max(1) as usize;
                                match ::syscall::message::SystemCallLongMessage::new(capacity) {
                                    Ok(asm) => {
                                        readlink_response_asm = Some((asm, op_id));
                                    },
                                    Err(e) => {
                                        // Allocation failure: cancel the caller now rather
                                        // than letting the pending op linger until eviction.
                                        ::syslog::error!(
                                            "failed to allocate readlink response assembler \
                                             (op_id={}, capacity={}, error={:?})",
                                            op_id,
                                            capacity,
                                            e
                                        );
                                        readlink_response_asm = None;
                                        if let Some(op) = pending.remove(op_id) {
                                            pending::cancel_pending_op(op, ErrorCode::IoErr);
                                        }
                                        continue;
                                    },
                                }
                            }
                            if let Some((asm, op_id)) = readlink_response_asm.as_mut() {
                                let op_id_copy: ::hostfs_api::OperationId = *op_id;
                                if let Err(e) = asm.add_part(part) {
                                    ::syslog::error!(
                                        "failed to add readlink response part (op_id={}, \
                                         error={:?})",
                                        op_id_copy,
                                        e
                                    );
                                    readlink_response_asm = None;
                                    if let Some(op) = pending.remove(op_id_copy) {
                                        pending::cancel_pending_op(op, ErrorCode::IoErr);
                                    }
                                    continue;
                                }
                                if asm.is_complete() {
                                    let (asm_done, _) = readlink_response_asm.take().unwrap();
                                    let mut body: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
                                    for p in asm_done.take_parts() {
                                        let n: usize = p.payload_size as usize;
                                        body.extend_from_slice(&p.payload[..n]);
                                    }
                                    // op_id is already known from part 0; the body still
                                    // carries it in bytes [0..4] for `complete_readlink_long`.
                                    if let Some(op) = pending.remove(op_id_copy) {
                                        pending::complete_readlink_long(op, &body);
                                    } else {
                                        ::syslog::warn!(
                                            "long readlink response with no pending op (op_id={})",
                                            op_id_copy,
                                        );
                                    }
                                }
                            } else {
                                let pn: u16 = part.part_number;
                                ::syslog::warn!(
                                    "readlink response part received without active assembler \
                                     (part_number={})",
                                    pn,
                                );
                            }
                            continue;
                        }
                        if header.is_hostfs_response() {
                            let op_id: ::hostfs_api::OperationId =
                                ::hostfs_api::get_op_id(&message.payload);
                            if let Some(op) = pending.remove(op_id) {
                                pending::complete_pending_op(op, &message.payload);
                            } else {
                                ::syslog::warn!(
                                    "hostfs response with no pending op (header={:?}, op_id={})",
                                    header,
                                    op_id,
                                );
                            }
                            continue;
                        }
                    }
                    ::syslog::warn!("received unexpected ikc message, ignoring");
                },
                MessageType::Interrupt => {
                    ::syslog::warn!("received unexpected interrupt, ignoring");
                },
                MessageType::Exception => {
                    ::syslog::warn!("received unexpected exception, ignoring");
                },
                MessageType::ProcessTerminationEvent => {
                    ::syslog::warn!("received unexpected process termination event, ignoring");
                },
                MessageType::PullResponse => {
                    ::syslog::warn!("received unexpected pull response, ignoring");
                },
            },
            Err(e) => ::syslog::error!("failed to receive message (error={:?})", e),
        }
    }

    // Shutdown VFS daemon.
    let e = ::sys::kcall::pm::__kcall_exit(0);
    ::syslog::error!("failed to shutdown vfsd (error={:?})", e);

    loop {
        ::core::hint::spin_loop();
    }
}
