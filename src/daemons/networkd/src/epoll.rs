// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Thin, safe wrapper around a Linux `epoll` instance.
//!
//! The decoupled `networkd` reactor multiplexes every host socket it opens on the guest's behalf
//! through a single `epoll` file descriptor. This module owns the raw `libc::epoll_*` FFI so the
//! rest of the daemon can register, re-arm, and drain readiness for raw file descriptors without
//! any `unsafe`. It deliberately mirrors the newtype-over-`libc` style used by `linuxd`'s
//! `poll.rs`.
//!
//! The `epoll` fd itself is meant to be wrapped in a Tokio [`AsyncFd`](tokio::io::unix::AsyncFd)
//! so the reactor can await readiness of the whole set with a single task; [`Epoll::wait`] is then
//! called with a zero timeout to drain the ready list without blocking the runtime.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    io,
    os::fd::{
        AsRawFd,
        RawFd,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Interest/readiness flag: the file descriptor is readable.
pub const EPOLLIN: u32 = libc::EPOLLIN as u32;
/// Interest/readiness flag: the file descriptor is writable.
pub const EPOLLOUT: u32 = libc::EPOLLOUT as u32;
/// Readiness flag: an error condition occurred on the file descriptor (always reported).
pub const EPOLLERR: u32 = libc::EPOLLERR as u32;
/// Readiness flag: the peer hung up (always reported).
pub const EPOLLHUP: u32 = libc::EPOLLHUP as u32;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// An owned Linux `epoll` instance.
///
/// The wrapped file descriptor is closed when the value is dropped.
///
pub struct Epoll {
    /// The raw `epoll` file descriptor.
    fd: RawFd,
}

///
/// # Description
///
/// A single readiness event drained from an [`Epoll`] instance.
///
#[derive(Debug, Clone, Copy)]
pub struct EpollEvent {
    /// The registration token (the `u64` stored in `epoll_data`) associated with the ready fd.
    pub token: u64,
    /// The readiness flags reported for the fd (a bitmask of `EPOLL*`).
    pub events: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Epoll {
    ///
    /// # Description
    ///
    /// Creates a new `epoll` instance with the close-on-exec flag set.
    ///
    /// # Returns
    ///
    /// On success, the owned [`Epoll`]. On failure, the underlying I/O error.
    ///
    pub fn new() -> io::Result<Self> {
        // SAFETY: `epoll_create1` has no memory-safety preconditions; it returns a new fd or -1.
        let fd: RawFd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    ///
    /// # Description
    ///
    /// Registers `fd` with this `epoll` instance under the given `token`, requesting the readiness
    /// `interest` (a bitmask of `EPOLLIN`/`EPOLLOUT`). Registrations are level-triggered.
    ///
    pub fn add(&self, fd: RawFd, token: u64, interest: u32) -> io::Result<()> {
        self.ctl(libc::EPOLL_CTL_ADD, fd, token, interest)
    }

    ///
    /// # Description
    ///
    /// Updates the readiness `interest` (and `token`) for an already-registered `fd`.
    ///
    pub fn modify(&self, fd: RawFd, token: u64, interest: u32) -> io::Result<()> {
        self.ctl(libc::EPOLL_CTL_MOD, fd, token, interest)
    }

    ///
    /// # Description
    ///
    /// Removes `fd` from this `epoll` instance. It is not an error to call this for an fd that is
    /// about to be closed; closing an fd also removes it from every `epoll` set automatically.
    ///
    pub fn delete(&self, fd: RawFd) -> io::Result<()> {
        // SAFETY: `event` is ignored for EPOLL_CTL_DEL on modern kernels but must be non-null on
        // older ones, so a valid zeroed event is passed.
        let mut event: libc::epoll_event = libc::epoll_event { events: 0, u64: 0 };
        let rc: i32 = unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_DEL, fd, &mut event) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Drains up to `events.len()` ready registrations into `events`, waiting at most `timeout_ms`
    /// milliseconds (use `0` for a non-blocking poll).
    ///
    /// # Returns
    ///
    /// On success, the slice of `events` that were filled. On failure, the underlying I/O error
    /// (with `EINTR` surfaced to the caller, which should retry).
    ///
    pub fn wait<'a>(
        &self,
        events: &'a mut [libc::epoll_event],
        timeout_ms: i32,
    ) -> io::Result<&'a [libc::epoll_event]> {
        // SAFETY: `events` is a valid, writable slice of `epoll_event`; the kernel writes at most
        // `events.len()` entries and returns the count.
        let n: i32 = unsafe {
            libc::epoll_wait(self.fd, events.as_mut_ptr(), events.len() as i32, timeout_ms)
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(&events[..n as usize])
    }

    /// Issues a single `epoll_ctl` call, translating a non-zero return into an [`io::Error`].
    fn ctl(&self, op: i32, fd: RawFd, token: u64, interest: u32) -> io::Result<()> {
        let mut event: libc::epoll_event = libc::epoll_event {
            events: interest,
            u64: token,
        };
        // SAFETY: `event` is a valid, initialized `epoll_event`; `op`/`fd` are validated by the
        // kernel, which returns -1 on error.
        let rc: i32 = unsafe { libc::epoll_ctl(self.fd, op, fd, &mut event) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl AsRawFd for Epoll {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        // SAFETY: `self.fd` is an owned, open fd that is not used after this point.
        unsafe {
            libc::close(self.fd);
        }
    }
}

//==================================================================================================
// Free Functions
//==================================================================================================

///
/// # Description
///
/// Extracts a readiness event from a raw [`libc::epoll_event`].
///
pub fn decode_event(event: &libc::epoll_event) -> EpollEvent {
    EpollEvent {
        token: event.u64,
        events: event.events,
    }
}

///
/// # Description
///
/// Returns a zeroed [`libc::epoll_event`], suitable for filling an [`Epoll::wait`] buffer.
///
pub fn empty_event() -> libc::epoll_event {
    libc::epoll_event { events: 0, u64: 0 }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::os::fd::FromRawFd;

    /// A registered pipe read-end reports `EPOLLIN` once its write-end has data.
    #[test]
    fn reports_readable_pipe() {
        let mut fds: [RawFd; 2] = [0; 2];
        // SAFETY: `fds` is a valid two-element array the kernel fills with the pipe fds.
        let rc: i32 = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() should succeed");
        let (read_fd, write_fd): (RawFd, RawFd) = (fds[0], fds[1]);

        let epoll: Epoll = Epoll::new().expect("epoll_create1");
        epoll.add(read_fd, 0x1234, EPOLLIN).expect("epoll add");

        // Nothing written yet: a non-blocking wait returns no events.
        let mut buf: [libc::epoll_event; 4] = [empty_event(); 4];
        let ready = epoll.wait(&mut buf, 0).expect("epoll wait");
        assert!(ready.is_empty(), "no events before writing");

        // SAFETY: `write_fd` is a valid open fd; one byte is written from a valid pointer.
        let byte: u8 = 0x5A;
        let n: isize = unsafe { libc::write(write_fd, (&byte as *const u8).cast(), 1) };
        assert_eq!(n, 1, "write one byte");

        let mut buf: [libc::epoll_event; 4] = [empty_event(); 4];
        let ready = epoll.wait(&mut buf, 100).expect("epoll wait");
        assert_eq!(ready.len(), 1, "one ready event after writing");
        let event: EpollEvent = decode_event(&ready[0]);
        assert_eq!(event.token, 0x1234);
        assert_ne!(event.events & EPOLLIN, 0, "read end is readable");

        // Close the pipe ends. Wrapping them in `OwnedFd` guarantees they are closed exactly once.
        // SAFETY: both fds are owned here and not used afterwards.
        unsafe {
            drop(::std::os::fd::OwnedFd::from_raw_fd(read_fd));
            drop(::std::os::fd::OwnedFd::from_raw_fd(write_fd));
        }
    }

    /// Dropping an `Epoll` closes its fd (a second close of the same fd fails with `EBADF`).
    #[test]
    fn drop_closes_fd() {
        let raw: RawFd = {
            let epoll: Epoll = Epoll::new().expect("epoll_create1");
            epoll.as_raw_fd()
        };
        // SAFETY: `raw` referred to the now-dropped epoll fd; closing it again must fail.
        let rc: i32 = unsafe { libc::close(raw) };
        assert_eq!(rc, -1, "epoll fd should already be closed after drop");
    }
}
