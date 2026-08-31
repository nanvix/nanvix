// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    console_wait::ConsoleWaitTable,
    error::{
        build_error,
        fat32_to_error_code,
    },
};
use ::alloc::{
    vec,
    vec::Vec,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::poll::{
    poll_errors::POLLNVAL,
    poll_flags::{
        POLLIN,
        POLLRDNORM,
    },
};
use ::syscall::{
    message::MessagePartitioner,
    poll::message::{
        PollRequest,
        PollResponse,
    },
};
use ::vfs::Fat32Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub(crate) fn handle_poll(
    source_pid: ProcessIdentifier,
    source: ThreadIdentifier,
    request: PollRequest,
    console_wait: &mut ConsoleWaitTable,
) -> Vec<Message> {
    if let Some((fd, _events)) = console_probe(&request) {
        if let Err(error) = super::try_feed_console_input(fd) {
            return vec![build_error(source, error)];
        }
        super::service_console_readers(console_wait);
        ::vfs::fd::set_current_process(source_pid);
    }

    let revents: Vec<i16> = match collect_ready(&request) {
        Ok(ready) => ready,
        Err(error) => return vec![build_error(source, error)],
    };

    let response: PollResponse = match PollResponse::new(&revents) {
        Ok(response) => response,
        Err(error) => return vec![build_error(source, error.code)],
    };
    match response.into_parts(source, ProcessIdentifier::VFSD, MessageType::Ipc) {
        Ok(parts) => parts,
        Err(error) => vec![build_error(source, error.code)],
    }
}

fn collect_ready(request: &PollRequest) -> Result<Vec<i16>, ErrorCode> {
    let mut revents: Vec<i16> = Vec::with_capacity(request.fds.len());

    for (&fd, &events) in request.fds.iter().zip(&request.events) {
        if fd < 0 {
            revents.push(0);
            continue;
        }

        let events: i16 = match ::vfs::fd::vfs_poll(fd, events) {
            Ok(events) => events,
            Err(Fat32Error::InvalidFd) => POLLNVAL,
            Err(error) => return Err(fat32_to_error_code(&error)),
        };
        revents.push(events);
    }

    Ok(revents)
}

fn console_probe(request: &PollRequest) -> Option<(i32, i16)> {
    const READ_EVENTS: i16 = POLLIN | POLLRDNORM;

    request
        .fds
        .iter()
        .copied()
        .zip(request.events.iter().copied())
        .find_map(|(fd, events)| {
            if events & READ_EVENTS == 0 {
                return None;
            }
            if !matches!(::vfs::fd::vfs_terminal_access(fd), Ok((true, _))) {
                return None;
            }
            match ::vfs::fd::vfs_poll(fd, events) {
                Ok(revents) if revents & READ_EVENTS == 0 => Some((fd, events)),
                _ => None,
            }
        })
}
