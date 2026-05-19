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
