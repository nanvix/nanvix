// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::poll::{
    poll,
    PollEvents,
    PollFd,
    PollTimeout,
};
use ::alloc::vec::Vec;
use ::core::{
    cmp,
    time::Duration,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_short,
    },
    poll::{
        poll_errors::{
            POLLERR,
            POLLHUP,
            POLLNVAL,
        },
        poll_flags::{
            POLLIN,
            POLLOUT,
            POLLPRI,
            POLLRDBAND,
            POLLRDNORM,
            POLLWRBAND,
            POLLWRNORM,
        },
    },
    sys_select::{
        fd_set,
        timeval,
        FD_SETSIZE,
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

/// Descriptor sets in which one polled descriptor appears.
#[derive(Clone, Copy, Debug)]
struct SelectInterests {
    read: bool,
    write: bool,
    error: bool,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Converts a `select()` timeout to a duration, or `None` for an infinite wait.
fn select_timeout(timeout: &Option<timeval>) -> Result<Option<Duration>, Error> {
    let Some(timeout) = timeout else {
        return Ok(None);
    };
    if timeout.tv_sec < 0 || timeout.tv_usec < 0 || timeout.tv_usec >= 1_000_000 {
        return Err(Error::new(ErrorCode::InvalidArgument, "invalid select timeout"));
    }

    let seconds: u64 = u64::try_from(timeout.tv_sec)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid select timeout"))?;
    let nanoseconds: u32 = u32::try_from(timeout.tv_usec * 1_000)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid select timeout"))?;
    Ok(Some(Duration::new(seconds, nanoseconds)))
}

/// Builds one polling entry per descriptor present in any input set.
fn poll_fds(
    nfds: usize,
    readfds: Option<&fd_set>,
    writefds: Option<&fd_set>,
    errorfds: Option<&fd_set>,
) -> Result<(Vec<PollFd>, Vec<SelectInterests>), Error> {
    let mut poll_fds: Vec<PollFd> = Vec::new();
    let mut interests: Vec<SelectInterests> = Vec::new();
    for fd in 0..nfds {
        let read: bool = readfds.is_some_and(|fds| fds.is_set(fd).unwrap_or(false));
        let write: bool = writefds.is_some_and(|fds| fds.is_set(fd).unwrap_or(false));
        let error: bool = errorfds.is_some_and(|fds| fds.is_set(fd).unwrap_or(false));
        if !read && !write && !error {
            continue;
        }

        let mut events: c_short = 0;
        if read {
            events |= POLLIN | POLLRDNORM | POLLRDBAND;
        }
        if write {
            events |= POLLOUT | POLLWRNORM | POLLWRBAND;
        }
        if error {
            events |= POLLPRI;
        }
        let fd: c_int = c_int::try_from(fd)
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "descriptor overflows"))?;
        poll_fds.push(PollFd::new(fd, PollEvents::from(events)));
        interests.push(SelectInterests { read, write, error });
    }
    Ok((poll_fds, interests))
}

/// Replaces input sets with the descriptors reported ready by `poll()`.
fn update_sets(
    poll_fds: &[PollFd],
    interests: &[SelectInterests],
    events: &[PollEvents],
    mut readfds: Option<&mut fd_set>,
    mut writefds: Option<&mut fd_set>,
    mut errorfds: Option<&mut fd_set>,
) -> Result<usize, Error> {
    if poll_fds.len() != interests.len() || poll_fds.len() != events.len() {
        return Err(Error::new(ErrorCode::InvalidMessage, "poll result length mismatch"));
    }
    if events
        .iter()
        .any(|events| c_short::from(events) & POLLNVAL != 0)
    {
        return Err(Error::new(ErrorCode::BadFile, "select contains an invalid descriptor"));
    }

    if let Some(fds) = readfds.as_deref_mut() {
        fds.zero();
    }
    if let Some(fds) = writefds.as_deref_mut() {
        fds.zero();
    }
    if let Some(fds) = errorfds.as_deref_mut() {
        fds.zero();
    }

    let mut ready: usize = 0;
    for ((poll_fd, interests), events) in poll_fds.iter().zip(interests).zip(events) {
        let events: c_short = events.into();
        let fd: usize = usize::try_from(poll_fd.fd())
            .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid poll descriptor"))?;
        let read_ready: bool =
            interests.read && events & (POLLIN | POLLRDNORM | POLLRDBAND | POLLHUP | POLLERR) != 0;
        let write_ready: bool = interests.write
            && events & (POLLOUT | POLLWRNORM | POLLWRBAND | POLLHUP | POLLERR) != 0;
        let error_ready: bool = interests.error && events & POLLPRI != 0;

        if read_ready {
            let fds: &mut fd_set = readfds.as_deref_mut().ok_or_else(|| {
                Error::new(ErrorCode::InvalidMessage, "read interest has no descriptor set")
            })?;
            fds.set_bit(fd)
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid read descriptor"))?;
        }
        if write_ready {
            let fds: &mut fd_set = writefds.as_deref_mut().ok_or_else(|| {
                Error::new(ErrorCode::InvalidMessage, "write interest has no descriptor set")
            })?;
            fds.set_bit(fd)
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid write descriptor"))?;
        }
        if error_ready {
            let fds: &mut fd_set = errorfds.as_deref_mut().ok_or_else(|| {
                Error::new(ErrorCode::InvalidMessage, "error interest has no descriptor set")
            })?;
            fds.set_bit(fd)
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid error descriptor"))?;
        }
        ready += usize::from(read_ready) + usize::from(write_ready) + usize::from(error_ready);
    }
    Ok(ready)
}

///
/// # Description
///
/// Performs synchronous I/O multiplexing.
///
/// # Parameters
///
/// - `nfds`: Highest-numbered file descriptor plus one.
/// - `readfds`: Set of file descriptors to be checked for readability.
/// - `writefds`: Set of file descriptors to be checked for writability.
/// - `errorfds`: Set of file descriptors to be checked for exceptional conditions.
/// - `timeout`: Maximum time to wait, or `None` to wait indefinitely.
///
/// # Return Value
///
/// On success, this function returns the total number of ready bits across the returned sets. On
/// failure, an error code is returned instead.
///
/// # References
///
/// - [POSIX `select()`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/select.html)
///
pub fn select(
    nfds: usize,
    readfds: Option<&mut fd_set>,
    writefds: Option<&mut fd_set>,
    errorfds: Option<&mut fd_set>,
    timeout: &Option<timeval>,
) -> Result<usize, Error> {
    ::syslog::trace!(
        "select(): nfds={:?}, readfds={:?}, writefds={:?}, errorfds={:?}, timeout={:?}",
        nfds,
        readfds,
        writefds,
        errorfds,
        timeout
    );

    if nfds > FD_SETSIZE {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "number of file descriptors exceeds maximum supported",
        ));
    }

    let mut remaining: Option<Duration> = select_timeout(timeout)?;
    let (poll_fds, interests): (Vec<PollFd>, Vec<SelectInterests>) =
        poll_fds(nfds, readfds.as_deref(), writefds.as_deref(), errorfds.as_deref())?;
    let mut probe_interval: Duration = INITIAL_PROBE_INTERVAL;
    let (ready, result_readfds, result_writefds, result_errorfds): (
        usize,
        Option<fd_set>,
        Option<fd_set>,
        Option<fd_set>,
    ) = loop {
        let events: Vec<PollEvents> = poll(&poll_fds, PollTimeout::from(0))?;
        let mut result_readfds: Option<fd_set> = readfds.as_deref().map(|_| fd_set::default());
        let mut result_writefds: Option<fd_set> = writefds.as_deref().map(|_| fd_set::default());
        let mut result_errorfds: Option<fd_set> = errorfds.as_deref().map(|_| fd_set::default());
        let ready: usize = update_sets(
            &poll_fds,
            &interests,
            &events,
            result_readfds.as_mut(),
            result_writefds.as_mut(),
            result_errorfds.as_mut(),
        )?;
        if ready != 0 || remaining.is_some_and(|duration| duration.is_zero()) {
            break (ready, result_readfds, result_writefds, result_errorfds);
        }

        let sleep: Duration = match remaining {
            Some(duration) => cmp::min(probe_interval, duration),
            None => probe_interval,
        };
        ::sys::kcall::pm::__kcall_sleep(sleep)?;
        if let Some(duration) = remaining.as_mut() {
            *duration = duration.checked_sub(sleep).ok_or_else(|| {
                Error::new(ErrorCode::InvalidArgument, "select timeout underflow")
            })?;
        }
        probe_interval = cmp::min(
            probe_interval
                .checked_add(probe_interval)
                .unwrap_or(MAX_PROBE_INTERVAL),
            MAX_PROBE_INTERVAL,
        );
    };

    if let (Some(destination), Some(result)) = (readfds, result_readfds) {
        *destination = result;
    }
    if let (Some(destination), Some(result)) = (writefds, result_writefds) {
        *destination = result;
    }
    if let (Some(destination), Some(result)) = (errorfds, result_errorfds) {
        *destination = result;
    }
    Ok(ready)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_timeout() {
        let timeout: Option<timeval> = Some(timeval {
            tv_sec: 2,
            tv_usec: 1,
        });
        assert_eq!(select_timeout(&timeout).unwrap(), Some(Duration::new(2, 1_000)));
        assert_eq!(select_timeout(&None).unwrap(), None);

        let maximum_timeout: Option<timeval> = Some(timeval {
            tv_sec: i64::MAX,
            tv_usec: 999_999,
        });
        assert_eq!(
            select_timeout(&maximum_timeout).unwrap(),
            Some(Duration::new(i64::MAX as u64, 999_999_000))
        );
    }

    #[test]
    fn rejects_invalid_timeout() {
        for timeout in [
            timeval {
                tv_sec: -1,
                tv_usec: 0,
            },
            timeval {
                tv_sec: 0,
                tv_usec: -1,
            },
            timeval {
                tv_sec: 0,
                tv_usec: 1_000_000,
            },
        ] {
            assert_eq!(
                select_timeout(&Some(timeout)).unwrap_err().code,
                ErrorCode::InvalidArgument
            );
        }
    }

    #[test]
    fn counts_each_ready_set() {
        let mut readfds: fd_set = fd_set::default();
        let mut writefds: fd_set = fd_set::default();
        readfds.set_bit(3).unwrap();
        writefds.set_bit(3).unwrap();
        let (poll_fds, interests) = poll_fds(4, Some(&readfds), Some(&writefds), None).unwrap();
        let events: [PollEvents; 1] = [PollEvents::from(POLLIN | POLLOUT)];

        assert_eq!(
            update_sets(
                &poll_fds,
                &interests,
                &events,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
            )
            .unwrap(),
            2
        );
        assert!(readfds.is_set(3).unwrap());
        assert!(writefds.is_set(3).unwrap());
    }

    #[test]
    fn counts_hangup_as_write_ready() {
        let mut writefds: fd_set = fd_set::default();
        writefds.set_bit(3).unwrap();
        let (poll_fds, interests) = poll_fds(4, None, Some(&writefds), None).unwrap();
        let events: [PollEvents; 1] = [PollEvents::from(POLLHUP)];

        assert_eq!(
            update_sets(&poll_fds, &interests, &events, None, Some(&mut writefds), None).unwrap(),
            1
        );
        assert!(writefds.is_set(3).unwrap());
    }

    #[test]
    fn clears_descriptors_that_are_not_ready() {
        let mut readfds: fd_set = fd_set::default();
        readfds.set_bit(2).unwrap();
        let (poll_fds, interests) = poll_fds(3, Some(&readfds), None, None).unwrap();
        let events: [PollEvents; 1] = [PollEvents::from(0)];

        assert_eq!(
            update_sets(&poll_fds, &interests, &events, Some(&mut readfds), None, None).unwrap(),
            0
        );
        assert!(!readfds.is_set(2).unwrap());
    }

    #[test]
    fn rejects_invalid_descriptors() {
        let poll_fds: [PollFd; 1] = [PollFd::new(7, PollEvents::from(POLLIN))];
        let interests: [SelectInterests; 1] = [SelectInterests {
            read: true,
            write: false,
            error: false,
        }];
        let events: [PollEvents; 1] = [PollEvents::from(POLLNVAL)];
        let mut readfds: fd_set = fd_set::default();
        readfds.set_bit(7).unwrap();

        assert_eq!(
            update_sets(&poll_fds, &interests, &events, Some(&mut readfds), None, None)
                .unwrap_err()
                .code,
            ErrorCode::BadFile
        );
    }
}
