// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::build_error,
    handler,
};
use ::sys::{
    error::ErrorCode,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::syscall::{
    fcntl::message::{
        OpenAtRequest,
        RenameAtRequest,
        UnlinkAtRequest,
    },
    message::{
        MessagePartitioner,
        SystemCallLongMessage,
        SystemCallMessagePart,
    },
    sys::stat::message::{
        FileChmodAtRequest,
        FileStatAtRequest,
        MakeDirectoryAtRequest,
        UpdateFileAccessTimeAtRequest,
    },
    unistd::message::{
        ChangeDirectoryRequest,
        FileAccessAtRequest,
        FileChownAtRequest,
        LinkAtRequest,
        ReadLinkAtRequest,
        SymbolicLinkAtRequest,
    },
    SystemCallMessageHeader,
};
use alloc::{
    collections::BTreeMap,
    vec,
    vec::Vec,
};

//==================================================================================================
// Multi-part Request Assembler
//==================================================================================================

/// Holds in-flight state for a multi-part request while parts are being accumulated.
pub(crate) struct AssemblerEntry {
    header: SystemCallMessageHeader,
    assembler: SystemCallLongMessage,
}

//==================================================================================================
// Multi-part Request Assembly & Dispatch
//==================================================================================================

pub(crate) fn assemble_and_dispatch(
    source: ThreadIdentifier,
    header: SystemCallMessageHeader,
    part: SystemCallMessagePart,
    assemblers: &mut BTreeMap<(i32, u16), AssemblerEntry>,
) -> Option<Vec<Message>> {
    let key: (i32, u16) = (i32::from(source), header as u16);

    // Look up or create assembler entry.
    let entry: &mut AssemblerEntry = assemblers.entry(key).or_insert_with(|| {
        let capacity: usize = max_capacity_for_header(header);
        AssemblerEntry {
            header,
            assembler: SystemCallLongMessage::new(capacity)
                .expect("capacity is set to a valid value"),
        }
    });

    // Add part to assembler.
    if let Err(e) = entry.assembler.add_part(part) {
        ::syslog::error!("assemble_and_dispatch(): add_part failed (error={:?})", e);
        assemblers.remove(&key);
        return Some(vec![build_error(source, ErrorCode::InvalidMessage)]);
    }

    // Check if all parts have arrived.
    if !entry.assembler.is_complete() {
        return None;
    }

    // Take the completed assembler entry.
    let completed: AssemblerEntry = assemblers.remove(&key).unwrap();
    let parts: Vec<SystemCallMessagePart> = completed.assembler.take_parts();

    // Dispatch based on the header type.
    Some(dispatch_assembled_request(source, completed.header, &parts))
}

fn max_capacity_for_header(header: SystemCallMessageHeader) -> usize {
    match header {
        SystemCallMessageHeader::OpenAtRequestPart => {
            OpenAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::RenameAtRequestPart => {
            RenameAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::UnlinkAtRequestPart => {
            UnlinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::FileStatAtRequestPart => {
            FileStatAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::MakeDirectoryAtRequestPart => {
            MakeDirectoryAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::ChangeDirectoryRequestPart => {
            ChangeDirectoryRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::FileAccessAtRequestPart => {
            FileAccessAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::SymbolicLinkAtRequestPart => {
            SymbolicLinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::LinkAtRequestPart => {
            LinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::ReadLinkAtRequestPart => {
            ReadLinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::UpdateFileAccessTimeAtRequestPart => {
            UpdateFileAccessTimeAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::FileChownAtRequestPart => {
            FileChownAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageHeader::FileChmodAtRequestPart => {
            FileChmodAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        // Fallback: generous capacity.
        _ => 64,
    }
}

fn dispatch_assembled_request(
    source: ThreadIdentifier,
    header: SystemCallMessageHeader,
    parts: &[SystemCallMessagePart],
) -> Vec<Message> {
    match header {
        SystemCallMessageHeader::OpenAtRequestPart => match OpenAtRequest::from_parts(parts) {
            Ok(req) => handler::handle_openat(source, req),
            Err(e) => {
                ::syslog::error!("dispatch: openat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageHeader::RenameAtRequestPart => match RenameAtRequest::from_parts(parts) {
            Ok(req) => handler::handle_renameat(source, req),
            Err(e) => {
                ::syslog::error!("dispatch: renameat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageHeader::UnlinkAtRequestPart => match UnlinkAtRequest::from_parts(parts) {
            Ok(req) => handler::handle_unlinkat(source, req),
            Err(e) => {
                ::syslog::error!("dispatch: unlinkat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageHeader::FileStatAtRequestPart => {
            match FileStatAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_fstatat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: fstatat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::MakeDirectoryAtRequestPart => {
            match MakeDirectoryAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_mkdirat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: mkdirat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::ChangeDirectoryRequestPart => {
            match ChangeDirectoryRequest::from_parts(parts) {
                Ok(req) => handler::handle_chdir(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: chdir from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::FileAccessAtRequestPart => {
            match FileAccessAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_faccessat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: faccessat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::SymbolicLinkAtRequestPart => {
            match SymbolicLinkAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_symlinkat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: symlinkat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::LinkAtRequestPart => match LinkAtRequest::from_parts(parts) {
            Ok(req) => handler::handle_linkat(source, req),
            Err(e) => {
                ::syslog::error!("dispatch: linkat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageHeader::ReadLinkAtRequestPart => {
            match ReadLinkAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_readlinkat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: readlinkat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::UpdateFileAccessTimeAtRequestPart => {
            match UpdateFileAccessTimeAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_utimensat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: utimensat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::FileChownAtRequestPart => {
            match FileChownAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_fchownat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: fchownat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageHeader::FileChmodAtRequestPart => {
            match FileChmodAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_fchmodat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: fchmodat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        _ => {
            ::syslog::warn!("dispatch_assembled_request(): unknown header {:?}", header);
            vec![build_error(source, ErrorCode::InvalidMessage)]
        },
    }
}
