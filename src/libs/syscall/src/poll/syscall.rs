// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fdtable::{
        resolve_for_poll,
        Route,
    },
    message::{
        MessagePartitioner,
        SystemCallLongMessage,
        SystemCallMessagePart,
    },
    poll::{
        input_message::{
            PollInputRequest,
            PollInputResponse,
        },
        message::{
            PollRequest,
            PollResponse,
        },
        socket_message::{
            PollSocketRequest,
            PollSocketResponse,
        },
    },
    safe::RawFileDescriptor,
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::alloc::vec::Vec;
use ::core::{
    cmp,
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
    pm::ThreadIdentifier,
    time::SystemTime,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_short,
    },
    poll::{
        poll_errors::POLLNVAL,
        poll_flags::{
            POLLIN,
            POLLOUT,
            POLLRDNORM,
            POLLWRNORM,
        },
    },
    unistd::{
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Initial delay between readiness probes.
const INITIAL_PROBE_INTERVAL: Duration = Duration::from_millis(1);

/// Maximum delay between readiness probes.
const MAX_PROBE_INTERVAL: Duration = Duration::from_millis(32);

//==================================================================================================
// Structures
//==================================================================================================

/// Events that can be polled for.
#[derive(Debug)]
pub struct PollEvents(c_short);

impl From<c_short> for PollEvents {
    fn from(value: c_short) -> Self {
        PollEvents(value)
    }
}

impl From<PollEvents> for c_short {
    fn from(value: PollEvents) -> Self {
        value.0
    }
}

impl From<&c_short> for PollEvents {
    fn from(value: &c_short) -> Self {
        PollEvents(*value)
    }
}
impl From<&PollEvents> for c_short {
    fn from(value: &PollEvents) -> Self {
        value.0
    }
}

impl PollEvents {
    /// Returns whether no events were reported.
    fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

/// Timeout for polling operations.
#[derive(Debug)]
pub struct PollTimeout(c_int);

impl From<c_int> for PollTimeout {
    fn from(value: c_int) -> Self {
        PollTimeout(value)
    }
}
impl From<PollTimeout> for c_int {
    fn from(value: PollTimeout) -> Self {
        value.0
    }
}

/// A pollable file descriptor.
#[derive(Debug)]
pub struct PollFd {
    fd: RawFileDescriptor,
    events: PollEvents,
}

impl PollFd {
    /// Creates a new `PollFd` with the given file descriptor and events.
    pub fn new(fd: RawFileDescriptor, events: PollEvents) -> Self {
        PollFd { fd, events }
    }

    /// Returns the file descriptor.
    pub fn fd(&self) -> RawFileDescriptor {
        self.fd
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Waits for one of a set of file descriptors to become ready to perform I/O.
///
/// # Parameters
///
/// - `fds`: Slice of `PollFd` structures describing the file descriptors to poll.
/// - `timeout`: Timeout in milliseconds. A negative value means infinite timeout.
///
/// # Returns
///
/// Upon success, this function returns one event mask for each input descriptor. If every mask is
/// empty, the timeout expired without any descriptor becoming ready. On failure, this function
/// returns an error.
///
/// # References
///
/// - [POSIX `poll()`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/poll.html)
///
pub fn poll(fds: &[PollFd], timeout: PollTimeout) -> Result<Vec<PollEvents>, Error> {
    ::syslog::trace!("poll(): fds={fds:?}, timeout={timeout:?}");

    let timeout: c_int = timeout.into();

    if fds.is_empty() || fds.iter().all(|fd| fd.fd() < 0) {
        const MAX_SLEEP: Duration = Duration::from_secs(u32::MAX as u64);
        if timeout < 0 {
            loop {
                ::sys::kcall::pm::__kcall_sleep(MAX_SLEEP)?;
            }
        }
        if timeout > 0 {
            let mut remaining: Duration = Duration::from_millis(timeout as u64);
            while remaining > MAX_SLEEP {
                ::sys::kcall::pm::__kcall_sleep(MAX_SLEEP)?;
                remaining = remaining.checked_sub(MAX_SLEEP).ok_or_else(|| {
                    Error::new(ErrorCode::InvalidArgument, "poll timeout underflow")
                })?;
            }
            if !remaining.is_zero() {
                ::sys::kcall::pm::__kcall_sleep(remaining)?;
            }
        }
        return Ok(fds.iter().map(|_| PollEvents(0)).collect());
    }

    let deadline: Option<SystemTime> = if timeout > 0 {
        let mut now: SystemTime = SystemTime::default();
        ::sys::kcall::pm::__kcall_gettime(&mut now)?;
        Some(
            now.checked_add_duration(&Duration::from_millis(timeout as u64))
                .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "poll timeout overflow"))?,
        )
    } else {
        None
    };

    let mut probe_interval: Duration = INITIAL_PROBE_INTERVAL;
    loop {
        let ready: Vec<PollEvents> = poll_once(fds)?;
        if ready.iter().any(|events| !events.is_empty()) || timeout == 0 {
            return Ok(ready);
        }

        let sleep: Duration = match deadline {
            Some(deadline) => {
                let mut now: SystemTime = SystemTime::default();
                ::sys::kcall::pm::__kcall_gettime(&mut now)?;
                let remaining: Duration = match deadline.checked_sub(&now) {
                    Ok(remaining) if !remaining.is_zero() => remaining,
                    _ => return Ok(ready),
                };
                cmp::min(probe_interval, remaining)
            },
            None => probe_interval,
        };
        ::sys::kcall::pm::__kcall_sleep(sleep)?;
        probe_interval = next_probe_interval(probe_interval);
    }
}

/// Returns the next delay in the readiness-probe backoff sequence.
fn next_probe_interval(current: Duration) -> Duration {
    match current.checked_add(current) {
        Some(next) => cmp::min(next, MAX_PROBE_INTERVAL),
        None => MAX_PROBE_INTERVAL,
    }
}

/// Queries each descriptor's resolved backend once without waiting for readiness.
///
/// The routing vectors retain one slot per input entry. Non-VFSD entries are represented by
/// negative placeholders in the VFSD request, then invalid, socket, and direct-console results
/// replace their corresponding slots. The returned vector therefore preserves input order and
/// keeps duplicate descriptors distinct. Backend probes run sequentially, so the result is not an
/// atomic readiness snapshot across all descriptors.
fn poll_once(fds: &[PollFd]) -> Result<Vec<PollEvents>, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let mut poll_fds: Vec<RawFileDescriptor> = Vec::with_capacity(fds.len());
    let mut sockets: Vec<Option<RawFileDescriptor>> = Vec::with_capacity(fds.len());
    let mut direct_consoles: Vec<Option<RawFileDescriptor>> = Vec::with_capacity(fds.len());
    let mut invalid: Vec<bool> = Vec::with_capacity(fds.len());
    for fd in fds {
        if fd.fd() < 0 {
            poll_fds.push(fd.fd());
            sockets.push(None);
            direct_consoles.push(None);
            invalid.push(false);
            continue;
        }

        match resolve_for_poll(fd.fd())? {
            Some((resolution, _)) if resolution.route == Route::Socket => {
                poll_fds.push(-1);
                sockets.push(Some(resolution.backend_fd));
                direct_consoles.push(None);
                invalid.push(false);
            },
            Some((resolution, false)) if resolution.route == Route::Console => {
                poll_fds.push(-1);
                sockets.push(None);
                direct_consoles.push(Some(resolution.backend_fd));
                invalid.push(false);
            },
            Some(_) => {
                poll_fds.push(fd.fd());
                sockets.push(None);
                direct_consoles.push(None);
                invalid.push(false);
            },
            None => {
                poll_fds.push(-1);
                sockets.push(None);
                direct_consoles.push(None);
                invalid.push(true);
            },
        }
    }
    let events: Vec<c_short> = fds.iter().map(|fd| fd.events.0).collect();
    let mut ready: Vec<PollEvents> = if poll_fds.iter().any(|fd| *fd >= 0) {
        poll_vfs_once(tid, fds, &poll_fds, &events)?
    } else {
        fds.iter().map(|_| PollEvents(0)).collect()
    };
    for (index, is_invalid) in invalid.iter().enumerate() {
        if *is_invalid {
            ready[index] = PollEvents(POLLNVAL);
        }
    }
    for (index, socket) in sockets.iter().enumerate() {
        if let Some(sockfd) = socket {
            ready[index] = poll_socket_once(tid, *sockfd, events[index])?;
        }
    }
    for (index, console) in direct_consoles.iter().enumerate() {
        if let Some(stream) = console {
            ready[index] = poll_direct_console_once(tid, *stream, events[index])?;
        }
    }
    Ok(ready)
}

/// Queries VFSD once for its subset of poll entries.
fn poll_vfs_once(
    tid: ThreadIdentifier,
    fds: &[PollFd],
    poll_fds: &[RawFileDescriptor],
    events: &[c_short],
) -> Result<Vec<PollEvents>, Error> {
    let request: PollRequest = PollRequest::new(poll_fds, events, 0)?;
    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;
    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    let capacity: usize = PollResponse::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
    let mut assembler: SystemCallLongMessage = SystemCallLongMessage::new(capacity)?;
    let mut interrupted: Option<Error> = None;

    loop {
        let response: Message = receive_pending_response(&token, &mut interrupted)?;
        if response.status != 0 {
            if let Some(error) = interrupted {
                return Err(error);
            }
            let error_code: ErrorCode = ErrorCode::try_from(response.status).map_err(|error| {
                ::syslog::warn!(
                    "poll(): failed to parse error code (fds={fds:?}, error={error:?})"
                );
                Error::new(ErrorCode::InvalidMessage, "poll() returned an invalid error code")
            })?;
            return Err(Error::new(error_code, "poll() failed"));
        }

        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.kind() {
            SystemCallMessageKind::PollResponsePart => {},
            _ => {
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "poll() received an unexpected response",
                ));
            },
        }

        let part: SystemCallMessagePart = SystemCallMessagePart::from_bytes(message.payload);
        assembler.add_part(part)?;
        if !assembler.is_complete() {
            continue;
        }

        if let Some(error) = interrupted {
            return Err(error);
        }

        let response: PollResponse = PollResponse::from_parts(&assembler.take_parts())?;
        if response.revents.len() != fds.len() {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "poll() response length does not match request",
            ));
        }
        return Ok(response.revents.into_iter().map(PollEvents).collect());
    }
}

/// Queries a direct-ELF console stream without consuming host input.
fn poll_direct_console_once(
    tid: ThreadIdentifier,
    stream: RawFileDescriptor,
    events: c_short,
) -> Result<PollEvents, Error> {
    const READ_EVENTS: c_short = POLLIN | POLLRDNORM;
    const WRITE_EVENTS: c_short = POLLOUT | POLLWRNORM;

    if matches!(stream, STDOUT_FILENO | STDERR_FILENO) {
        return Ok(PollEvents(events & WRITE_EVENTS));
    }
    if stream != STDIN_FILENO || events & READ_EVENTS == 0 {
        return Ok(PollEvents(0));
    }

    let mut request: Message = PollInputRequest::build(tid, 0);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;
    let mut interrupted: Option<Error> = None;
    let response: Message = receive_pending_response(&token, &mut interrupted)?;
    if let Some(error) = interrupted {
        return Err(error);
    }

    let source: ::sys::ipc::MessageSender = response.source;
    if source != ::sys::ipc::MessageSender::KERNEL {
        return Err(Error::new(
            ErrorCode::InvalidMessage,
            "direct console poll returned an invalid sender",
        ));
    }
    if response.status != 0 {
        return Err(Error::new(
            ErrorCode::try_from(response.status)?,
            "direct console poll failed",
        ));
    }

    let response: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
    let header: SystemCallMessageKind = response.kind();
    if header != SystemCallMessageKind::PollInputResponse {
        return Err(Error::new(
            ErrorCode::InvalidMessage,
            "direct console poll returned an unexpected response",
        ));
    }
    let response: PollInputResponse = PollInputResponse::from_bytes(response.payload);
    match response.status() {
        PollInputRequest::STATUS_DATA | PollInputRequest::STATUS_EOF => {
            Ok(PollEvents(events & READ_EVENTS))
        },
        PollInputRequest::STATUS_EMPTY => Ok(PollEvents(0)),
        _ => Err(Error::new(
            ErrorCode::InvalidMessage,
            "direct console poll returned an invalid status",
        )),
    }
}

/// Queries networkd once for a socket readiness snapshot.
fn poll_socket_once(
    tid: ThreadIdentifier,
    sockfd: RawFileDescriptor,
    events: c_short,
) -> Result<PollEvents, Error> {
    let mut request: Message = PollSocketRequest::build(tid, sockfd, events);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    let mut interrupted: Option<Error> = None;
    let response: Message = receive_pending_response(&token, &mut interrupted)?;
    if let Some(error) = interrupted {
        return Err(error);
    }
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        return Err(Error::new(error_code, "socket poll failed"));
    }

    let response: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
    let header: SystemCallMessageKind = response.kind();
    if header != SystemCallMessageKind::PollSocketResponse {
        return Err(Error::new(
            ErrorCode::InvalidMessage,
            "socket poll returned an unexpected response",
        ));
    }
    let response: PollSocketResponse = PollSocketResponse::from_bytes(response.payload);
    Ok(PollEvents(response.revents()))
}

/// Receives a response while remembering signal interruption and keeping the mailbox synchronized.
fn receive_pending_response(
    token: &RequestToken,
    interrupted: &mut Option<Error>,
) -> Result<Message, Error> {
    loop {
        match crate::rpc::recv_response_interruptible(token) {
            Ok(response) => return Ok(response),
            Err(error) if error.code == ErrorCode::Interrupted => {
                interrupted.get_or_insert(error);
            },
            Err(error) => return Err(error),
        }
    }
}
