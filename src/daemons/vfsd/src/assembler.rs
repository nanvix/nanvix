// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    console_wait::ConsoleWaitTable,
    error::{
        build_error,
        ResponseContext,
    },
    handler,
    pending::PendingQueue,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        RequestIdentifier,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
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
    poll::message::PollRequest,
    sys::{
        mount::message::{
            MountRequest,
            UmountRequest,
        },
        stat::message::{
            FileChmodAtRequest,
            FileStatAtRequest,
            MakeDirectoryAtRequest,
            UpdateFileAccessTimeAtRequest,
        },
    },
    unistd::message::{
        ChangeDirectoryRequest,
        FileAccessAtRequest,
        FileChownAtRequest,
        LinkAtRequest,
        ReadLinkAtRequest,
        SymbolicLinkAtRequest,
    },
    SystemCallMessageKind,
};
use alloc::{
    collections::BTreeMap,
    vec,
    vec::Vec,
};

//==================================================================================================
// Multi-part Request Assembler
//==================================================================================================

/// Key that distinguishes concurrent multi-part requests from one caller thread.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct AssemblerKey {
    source_pid: i32,
    source_tid: i32,
    header: u16,
    request_id: u32,
}

/// Maximum number of incomplete multipart requests retained by vfsd.
const MAX_ASSEMBLERS: usize = ::config::kernel::MAX_THREADS * 4;

/// Holds in-flight state for a multi-part request while parts are being accumulated.
pub(crate) struct AssemblerEntry {
    header: SystemCallMessageKind,
    assembler: SystemCallLongMessage,
    response_context: ResponseContext,
}

fn assembler_key(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    header: SystemCallMessageKind,
    request_id: RequestIdentifier,
) -> AssemblerKey {
    AssemblerKey {
        source_pid: i32::from(source_pid),
        source_tid: i32::from(source_tid),
        header: header as u16,
        request_id: request_id.raw(),
    }
}

/// Removes every incomplete multipart request owned by `pid`.
pub(crate) fn purge_process(
    assemblers: &mut BTreeMap<AssemblerKey, AssemblerEntry>,
    pid: ProcessIdentifier,
) {
    let raw_pid: i32 = i32::from(pid);
    assemblers.retain(|key, _| key.source_pid != raw_pid);
}

/// Returns the first key of the process that holds the most incomplete multipart requests.
fn greediest_process_key(
    assemblers: &BTreeMap<AssemblerKey, AssemblerEntry>,
) -> Option<AssemblerKey> {
    // Keys are ordered by process first, so each process occupies one contiguous run.
    let mut best: Option<(AssemblerKey, usize)> = None;
    let mut current: Option<(AssemblerKey, usize)> = None;
    for key in assemblers.keys() {
        current = match current {
            Some((first, count)) if first.source_pid == key.source_pid => Some((first, count + 1)),
            _ => Some((*key, 1)),
        };
        if current.map(|(_, count)| count) > best.map(|(_, count)| count) {
            best = current;
        }
    }
    best.map(|(key, _)| key)
}

//==================================================================================================
// Multi-part Request Assembly & Dispatch
//==================================================================================================

pub(crate) fn assemble_and_dispatch(
    response_context: ResponseContext,
    header: SystemCallMessageKind,
    part: SystemCallMessagePart,
    assemblers: &mut BTreeMap<AssemblerKey, AssemblerEntry>,
    pending: &mut PendingQueue,
    console_wait: &mut ConsoleWaitTable,
) -> Option<(ResponseContext, Vec<Message>)> {
    let source: ThreadIdentifier = response_context.source_tid();
    let key: AssemblerKey =
        assembler_key(response_context.source_pid(), source, header, response_context.request_id());

    if !assemblers.contains_key(&key) && assemblers.len() >= MAX_ASSEMBLERS {
        // Evict from the process holding the most incomplete streams, so a client that floods the
        // table cannot starve unrelated processes of their in-flight multipart requests.
        let evicted_key: AssemblerKey =
            greediest_process_key(assemblers).expect("full assembler map should contain an entry");
        if let Some(evicted) = assemblers.remove(&evicted_key) {
            ::syslog::error!(
                "assemble_and_dispatch(): evicting incomplete request at capacity (pid={}, \
                 limit={})",
                evicted_key.source_pid,
                MAX_ASSEMBLERS
            );
            evicted.response_context.send(&build_error(
                evicted.response_context.source_tid(),
                ErrorCode::NoBufferSpace,
            ));
        }
    }

    // Look up or create assembler entry.
    let entry: &mut AssemblerEntry = assemblers.entry(key).or_insert_with(|| {
        let capacity: usize = max_capacity_for_header(header);
        AssemblerEntry {
            header,
            assembler: SystemCallLongMessage::new(capacity)
                .expect("capacity is set to a valid value"),
            response_context,
        }
    });

    // Add part to assembler.
    if let Err(e) = entry.assembler.add_part(part) {
        ::syslog::error!("assemble_and_dispatch(): add_part failed (error={:?})", e);
        assemblers.remove(&key);
        return Some((response_context, vec![build_error(source, ErrorCode::InvalidMessage)]));
    }

    // Check if all parts have arrived.
    if !entry.assembler.is_complete() {
        return None;
    }

    // Take the completed assembler entry.
    let completed: AssemblerEntry = assemblers.remove(&key).unwrap();
    let parts: Vec<SystemCallMessagePart> = completed.assembler.take_parts();

    // Dispatch based on the header type.
    let responses: Vec<Message> = dispatch_assembled_request(
        completed.response_context,
        completed.header,
        &parts,
        pending,
        console_wait,
    );
    Some((completed.response_context, responses))
}

fn max_capacity_for_header(header: SystemCallMessageKind) -> usize {
    match header {
        SystemCallMessageKind::OpenAtRequestPart => {
            OpenAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::RenameAtRequestPart => {
            RenameAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::UnlinkAtRequestPart => {
            UnlinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::FileStatAtRequestPart => {
            FileStatAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::MakeDirectoryAtRequestPart => {
            MakeDirectoryAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::ChangeDirectoryRequestPart => {
            ChangeDirectoryRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::FileAccessAtRequestPart => {
            FileAccessAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::SymbolicLinkAtRequestPart => {
            SymbolicLinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::LinkAtRequestPart => {
            LinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::ReadLinkAtRequestPart => {
            ReadLinkAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::UpdateFileAccessTimeAtRequestPart => {
            UpdateFileAccessTimeAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::FileChownAtRequestPart => {
            FileChownAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::FileChmodAtRequestPart => {
            FileChmodAtRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::HostMountRequestPart => {
            MountRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::HostUmountRequestPart => {
            UmountRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        SystemCallMessageKind::PollRequestPart => {
            PollRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        },
        // Fallback: generous capacity.
        _ => 64,
    }
}

fn dispatch_assembled_request(
    response_context: ResponseContext,
    header: SystemCallMessageKind,
    parts: &[SystemCallMessagePart],
    pending: &mut PendingQueue,
    console_wait: &mut ConsoleWaitTable,
) -> Vec<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    match header {
        SystemCallMessageKind::OpenAtRequestPart => match OpenAtRequest::from_parts(parts) {
            Ok(req) => {
                // Returns `None` when forwarded to hostfsd (response deferred to IKC completion).
                handler::handle_openat_with_hostfs(response_context, req, pending)
                    .unwrap_or_default()
            },
            Err(e) => {
                ::syslog::error!("dispatch: openat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageKind::RenameAtRequestPart => match RenameAtRequest::from_parts(parts) {
            Ok(req) => {
                // Returns `None` when forwarded to hostfsd (response deferred to IKC completion).
                handler::handle_renameat_with_hostfs(response_context, req, pending)
                    .unwrap_or_default()
            },
            Err(e) => {
                ::syslog::error!("dispatch: renameat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageKind::UnlinkAtRequestPart => match UnlinkAtRequest::from_parts(parts) {
            Ok(req) => {
                // Returns `None` when forwarded to hostfsd (response deferred to IKC completion).
                handler::handle_unlinkat_with_hostfs(response_context, req, pending)
                    .unwrap_or_default()
            },
            Err(e) => {
                ::syslog::error!("dispatch: unlinkat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageKind::FileStatAtRequestPart => {
            match FileStatAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_fstatat_with_hostfs(response_context, req, pending)
                    .unwrap_or_default(),
                Err(e) => {
                    ::syslog::error!("dispatch: fstatat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::MakeDirectoryAtRequestPart => {
            match MakeDirectoryAtRequest::from_parts(parts) {
                Ok(req) => {
                    // Returns `None` when forwarded to hostfsd (response deferred to IKC completion).
                    handler::handle_mkdirat_with_hostfs(response_context, req, pending)
                        .unwrap_or_default()
                },
                Err(e) => {
                    ::syslog::error!("dispatch: mkdirat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::ChangeDirectoryRequestPart => {
            match ChangeDirectoryRequest::from_parts(parts) {
                // Returns `None` when forwarded to hostfsd (response deferred to IKC completion).
                Ok(req) => handler::handle_chdir_with_hostfs(response_context, req, pending)
                    .unwrap_or_default(),
                Err(e) => {
                    ::syslog::error!("dispatch: chdir from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::FileAccessAtRequestPart => {
            match FileAccessAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_faccessat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: faccessat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::SymbolicLinkAtRequestPart => {
            match SymbolicLinkAtRequest::from_parts(parts) {
                // `None` means the request was forwarded to hostfsd and the response
                // will be sent asynchronously when the IKC reply arrives; emit no
                // immediate messages in that case.
                Ok(req) => handler::handle_symlinkat_with_hostfs(response_context, req, pending)
                    .unwrap_or_default(),
                Err(e) => {
                    ::syslog::error!("dispatch: symlinkat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::LinkAtRequestPart => match LinkAtRequest::from_parts(parts) {
            Ok(req) => handler::handle_linkat(source, req),
            Err(e) => {
                ::syslog::error!("dispatch: linkat from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageKind::ReadLinkAtRequestPart => {
            match ReadLinkAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_readlinkat_with_hostfs(response_context, req, pending)
                    .unwrap_or_default(),
                Err(e) => {
                    ::syslog::error!("dispatch: readlinkat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::UpdateFileAccessTimeAtRequestPart => {
            match UpdateFileAccessTimeAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_utimensat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: utimensat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::FileChownAtRequestPart => {
            match FileChownAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_fchownat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: fchownat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::FileChmodAtRequestPart => {
            match FileChmodAtRequest::from_parts(parts) {
                Ok(req) => handler::handle_fchmodat(source, req),
                Err(e) => {
                    ::syslog::error!("dispatch: fchmodat from_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::InvalidMessage)]
                },
            }
        },
        SystemCallMessageKind::HostMountRequestPart => match MountRequest::from_parts(parts) {
            Ok(req) => handler::handle_mount(source, req),
            Err(e) => {
                ::syslog::error!("dispatch: mount from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageKind::HostUmountRequestPart => match UmountRequest::from_parts(parts) {
            Ok(req) => handler::handle_umount(source, req),
            Err(e) => {
                ::syslog::error!("dispatch: umount from_parts failed (error={:?})", e);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        SystemCallMessageKind::PollRequestPart => match PollRequest::from_parts(parts) {
            Ok(request) => handler::handle_poll(source_pid, source, request, console_wait),
            Err(error) => {
                ::syslog::error!("dispatch: poll from_parts failed (error={:?})", error);
                vec![build_error(source, ErrorCode::InvalidMessage)]
            },
        },
        _ => {
            ::syslog::warn!("dispatch_assembled_request(): unknown header {:?}", header);
            vec![build_error(source, ErrorCode::InvalidMessage)]
        },
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembler_key_separates_request_identifiers() {
        let pid: ProcessIdentifier = ProcessIdentifier::from(6);
        let source: ThreadIdentifier = ThreadIdentifier::from(7);
        let header: SystemCallMessageKind = SystemCallMessageKind::OpenAtRequestPart;

        let first: AssemblerKey =
            assembler_key(pid, source, header, RequestIdentifier::from_raw(11));
        let second: AssemblerKey =
            assembler_key(pid, source, header, RequestIdentifier::from_raw(12));

        assert_ne!(first, second, "request identifiers should select separate streams");
    }

    #[test]
    fn assembler_key_separates_processes() {
        let source: ThreadIdentifier = ThreadIdentifier::from(7);
        let header: SystemCallMessageKind = SystemCallMessageKind::OpenAtRequestPart;
        let request_id: RequestIdentifier = RequestIdentifier::from_raw(11);

        let first: AssemblerKey =
            assembler_key(ProcessIdentifier::from(6), source, header, request_id);
        let second: AssemblerKey =
            assembler_key(ProcessIdentifier::from(8), source, header, request_id);

        assert_ne!(first, second, "process identifiers should select separate streams");
    }
}
