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
mod init;
mod ipc;

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
                // Non-IPC messages during signup are discarded. Before signup completes, vfsd
                // has no registered identity and cannot meaningfully process interrupts,
                // exceptions, IKC, or termination events. If this assumption changes (e.g.,
                // multi-phase initialization), these should be buffered and replayed.
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

    // Process any messages that were buffered during the signup phase.
    while let Some(message) = buffered_messages.pop_front() {
        match ipc::handle_ipc_message(message, &mut assemblers) {
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

    loop {
        match ::sys::kcall::ipc::__kcall_recv() {
            Ok(message) => match message.message_type {
                MessageType::Ipc => match ipc::handle_ipc_message(message, &mut assemblers) {
                    Ok(true) => break,
                    Ok(false) => continue,
                    Err(e) => ::syslog::error!("failed to handle ipc request (error={:?})", e),
                },
                MessageType::Interrupt => {
                    ::syslog::warn!("received unexpected interrupt, ignoring");
                },
                MessageType::Exception => {
                    ::syslog::warn!("received unexpected exception, ignoring");
                },
                MessageType::Ikc => {
                    ::syslog::warn!("received unexpected ikc message, ignoring");
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
