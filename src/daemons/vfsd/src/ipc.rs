// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    assembler::{
        assemble_and_dispatch,
        AssemblerEntry,
    },
    error::{
        build_error,
        send_response,
    },
    handler,
    pending::PendingQueue,
};
use ::proc::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    message::SystemCallMessagePart,
    SystemCallMessage,
    SystemCallMessageHeader,
};
use alloc::{
    collections::BTreeMap,
    vec::Vec,
};

//==================================================================================================
// Helper: Extract caller TID from message source
//==================================================================================================

/// Extracts the caller's thread identifier from an IPC message.
///
/// When the message source encodes a PID (as is the case for messages routed through the kernel),
/// the TID is derived by casting the PID value to a TID. This is correct only for single-threaded
/// processes where TID == PID. Multi-threaded callers would require the source to encode the TID
/// directly.
fn caller_tid(message: &Message) -> ThreadIdentifier {
    let source = message.source;
    match source.as_id() {
        Ok(pid) => {
            // PID-encoded source — derive TID from PID value (valid for single-threaded callers).
            ThreadIdentifier::from(i32::from(pid))
        },
        Err(tid) => tid,
    }
}

//==================================================================================================
// SystemMessage Handler (procd shutdown)
//==================================================================================================

fn handle_system_message(message: Message) -> Result<bool, Error> {
    let sys_msg: SystemMessage = SystemMessage::from_bytes(message.payload)?;
    match sys_msg.header {
        SystemMessageHeader::ProcessManagement => {
            let pm_msg: ProcessManagementMessage =
                ProcessManagementMessage::from_bytes(sys_msg.payload)?;
            match pm_msg.header {
                ProcessManagementMessageHeader::Shutdown => {
                    let shutdown: ShutdownMessage = ShutdownMessage::from_bytes(pm_msg.payload);
                    ::syslog::info!("shutting down (code={:?})...", shutdown.code);
                    Ok(true)
                },
                _ => {
                    ::syslog::warn!("received unknown process management message, ignoring...");
                    Ok(false)
                },
            }
        },
        SystemMessageHeader::MemoryManagement => {
            ::syslog::warn!("received memory management message, ignoring...");
            Ok(false)
        },
        SystemMessageHeader::FilesystemManagement => {
            ::syslog::warn!("received filesystem management message, ignoring...");
            Ok(false)
        },
    }
}

//==================================================================================================
// IPC Message Dispatch
//==================================================================================================

pub(crate) fn handle_ipc_message(
    message: Message,
    assemblers: &mut BTreeMap<(i32, u16), AssemblerEntry>,
    pending: &mut PendingQueue,
) -> Result<bool, Error> {
    let msg_source = message.source;
    let source_tid: ThreadIdentifier = caller_tid(&message);
    let source_pid: ProcessIdentifier = match msg_source.as_id() {
        Ok(pid) => pid,
        Err(tid) => ProcessIdentifier::from(i32::from(tid)),
    };

    // Route messages from the process manager daemon (PROCD).
    if source_pid == ProcessIdentifier::PROCD {
        return handle_system_message(message);
    }

    // Parse as SystemCallMessage from user processes.
    let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(message.payload) {
        Ok(msg) => msg,
        Err(e) => {
            ::syslog::error!("failed to parse syscall message (error={:?})", e);
            send_response(&build_error(source_tid, ErrorCode::InvalidMessage));
            return Ok(false);
        },
    };

    match syscall_msg.header {
        //==========================================================================================
        // Short requests: single message request, single message response.
        //==========================================================================================
        SystemCallMessageHeader::CloseRequest => {
            if let Some(response) =
                handler::handle_close_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::SeekRequest => {
            if let Some(response) =
                handler::handle_seek_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::FileSyncRequest => {
            if let Some(response) =
                handler::handle_fsync_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::FileDataSyncRequest => {
            let response: Message = handler::handle_fdatasync(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileTruncateRequest => {
            if let Some(response) =
                handler::handle_ftruncate_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::FileSpaceControlRequest => {
            let response: Message = handler::handle_fallocate(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileAdvisoryInformationRequest => {
            let response: Message = handler::handle_fadvise(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileControlRequest => {
            let response: Message = handler::handle_fcntl(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileChmodRequest => {
            let response: Message = handler::handle_fchmod(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileChownRequest => {
            let response: Message = handler::handle_fchown(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileChdirRequest => {
            let response: Message = handler::handle_fchdir(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::UpdateFileAccessTimeRequest => {
            let response: Message = handler::handle_futimens(source_tid, syscall_msg);
            send_response(&response);
        },

        //==========================================================================================
        // Read/Write: single message request + bulk data via push/pull.
        //==========================================================================================
        SystemCallMessageHeader::ReadRequest => {
            if let Some(response) =
                handler::handle_read_with_hostfs(source_pid, source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::WriteRequest => {
            if let Some(response) =
                handler::handle_write_with_hostfs(source_pid, source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },

        //==========================================================================================
        // Partial read/write: inline data in message payload.
        //==========================================================================================
        SystemCallMessageHeader::PartialReadRequest => {
            let response: Message = handler::handle_pread(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::PartialWriteRequest => {
            let response: Message = handler::handle_pwrite(source_tid, syscall_msg);
            send_response(&response);
        },

        //==========================================================================================
        // Long responses: single request, multi-part response.
        //==========================================================================================
        SystemCallMessageHeader::FileStatRequest => {
            if let Some(responses) =
                handler::handle_fstat_with_hostfs(source_tid, syscall_msg, pending)
            {
                for response in responses {
                    send_response(&response);
                }
            }
        },
        SystemCallMessageHeader::GetCurrentWorkingDirectoryRequest => {
            let responses: Vec<Message> = handler::handle_getcwd(source_tid);
            for response in responses {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::GetDirectoryEntriesRequest => {
            if let Some(responses) =
                handler::handle_getdents_with_hostfs(source_tid, syscall_msg, pending)
            {
                for response in responses {
                    send_response(&response);
                }
            }
        },

        //==========================================================================================
        // Long requests: multi-part request, single or multi-part response.
        //==========================================================================================
        SystemCallMessageHeader::OpenAtRequestPart
        | SystemCallMessageHeader::RenameAtRequestPart
        | SystemCallMessageHeader::UnlinkAtRequestPart
        | SystemCallMessageHeader::FileStatAtRequestPart
        | SystemCallMessageHeader::MakeDirectoryAtRequestPart
        | SystemCallMessageHeader::ChangeDirectoryRequestPart
        | SystemCallMessageHeader::FileAccessAtRequestPart
        | SystemCallMessageHeader::SymbolicLinkAtRequestPart
        | SystemCallMessageHeader::LinkAtRequestPart
        | SystemCallMessageHeader::ReadLinkAtRequestPart
        | SystemCallMessageHeader::UpdateFileAccessTimeAtRequestPart
        | SystemCallMessageHeader::FileChownAtRequestPart
        | SystemCallMessageHeader::FileChmodAtRequestPart
        | SystemCallMessageHeader::HostMountRequestPart
        | SystemCallMessageHeader::HostUmountRequestPart => {
            let part: SystemCallMessagePart =
                SystemCallMessagePart::from_bytes(syscall_msg.payload);
            if let Some(responses) =
                assemble_and_dispatch(source_tid, syscall_msg.header, part, assemblers, pending)
            {
                for response in responses {
                    send_response(&response);
                }
            }
        },

        //==========================================================================================
        // Unknown or unsupported headers.
        //==========================================================================================
        _ => {
            let hdr = syscall_msg.header;
            ::syslog::warn!("received unsupported syscall header: {:?}", hdr);
            send_response(&build_error(source_tid, ErrorCode::InvalidMessage));
        },
    }

    Ok(false)
}
