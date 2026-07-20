// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Thin, safe wrapper around a Linux `epoll` instance.
//!
//! The decoupled `networkd` reactor multiplexes every host socket it opens on the guest's behalf
//! through a single `epoll` file descriptor. This module owns the raw `::libc::epoll_*` FFI so the
//! rest of the daemon can register, re-arm, and drain readiness for raw file descriptors without
//! any `unsafe`; no `::libc` type leaks through its API.
//!
//! The `epoll` fd itself is meant to be wrapped in a Tokio `AsyncFd` so the reactor can await
//! readiness of the whole set with a single task; [`Epoll::wait`] is then called with a zero
//! timeout to drain the ready list without blocking the runtime.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    io,
    os::fd::{
        AsRawFd,
        FromRawFd,
        OwnedFd,
        RawFd,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Interest/readiness flag: the file descriptor is readable.
pub const EPOLLIN: u32 = ::libc::EPOLLIN as u32;
/// Interest/readiness flag: the file descriptor is writable.
pub const EPOLLOUT: u32 = ::libc::EPOLLOUT as u32;
/// Readiness flag: an error condition occurred on the file descriptor (always reported).
pub const EPOLLERR: u32 = ::libc::EPOLLERR as u32;
/// Readiness flag: the peer hung up (always reported).
pub const EPOLLHUP: u32 = ::libc::EPOLLHUP as u32;

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
    /// The owned `epoll` file descriptor (closed on drop by [`OwnedFd`]).
    fd: OwnedFd,
}

///
/// # Description
///
/// A single readiness event drained from an [`Epoll`] instance.
///
/// This is a transparent wrapper over the kernel's `epoll_event`, so a [`EpollEvent`] buffer is
/// filled by [`Epoll::wait`] directly, without copying or exposing `::libc` types.
///
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EpollEvent(::libc::epoll_event);

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
        let fd: RawFd = unsafe { ::libc::epoll_create1(::libc::EPOLL_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `fd` is a freshly created fd owned exclusively by this `OwnedFd`, which closes
        // it exactly once on drop.
        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    ///
    /// # Description
    ///
    /// Registers `fd` with this `epoll` instance under the given `token`, requesting the readiness
    /// `interest` (a bitmask of `EPOLLIN`/`EPOLLOUT`). Registrations are level-triggered.
    ///
    pub fn add(&self, fd: RawFd, token: u64, interest: u32) -> io::Result<()> {
        self.ctl(::libc::EPOLL_CTL_ADD, fd, token, interest)
    }

    ///
    /// # Description
    ///
    /// Updates the readiness `interest` (and `token`) for an already-registered `fd`.
    ///
    pub fn modify(&self, fd: RawFd, token: u64, interest: u32) -> io::Result<()> {
        self.ctl(::libc::EPOLL_CTL_MOD, fd, token, interest)
    }

    ///
    /// # Description
    ///
    /// Removes `fd` from this `epoll` instance. It is not an error to call this for an fd that is
    /// about to be closed; closing an fd also removes it from every `epoll` set automatically.
    ///
    pub fn delete(&self, fd: RawFd) -> io::Result<()> {
        // `event` is ignored for EPOLL_CTL_DEL on modern kernels but must be non-null on older
        // ones, so a valid zeroed event is passed.
        let mut event: ::libc::epoll_event = ::libc::epoll_event { events: 0, u64: 0 };
        // SAFETY: `event` is a valid, initialized `epoll_event`; the kernel returns -1 on error.
        let rc: i32 = unsafe {
            ::libc::epoll_ctl(self.fd.as_raw_fd(), ::libc::EPOLL_CTL_DEL, fd, &mut event)
        };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Drains up to `events.len()` ready registrations into `events`, waiting at most `timeout_ms`
    /// milliseconds (use `0` for a non-blocking poll). An empty `events` buffer is a no-op that
    /// returns an empty slice without calling into the kernel.
    ///
    /// # Returns
    ///
    /// On success, the slice of `events` that were filled. On failure, the underlying I/O error
    /// (with `EINTR` surfaced to the caller, which should retry), or an `InvalidInput` error if
    /// `events.len()` exceeds `i32::MAX`.
    ///
    pub fn wait<'a>(
        &self,
        events: &'a mut [EpollEvent],
        timeout_ms: i32,
    ) -> io::Result<&'a [EpollEvent]> {
        // `epoll_wait` rejects `maxevents == 0` with EINVAL, so treat an empty buffer as a no-op.
        if events.is_empty() {
            return Ok(&events[..0]);
        }
        // Casting the length with `as` would truncate (and possibly turn negative) for buffers
        // longer than `i32::MAX`, so validate it instead.
        let maxevents: i32 = i32::try_from(events.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "events buffer exceeds i32::MAX entries")
        })?;
        // SAFETY: `EpollEvent` is `repr(transparent)` over `::libc::epoll_event`, so `events` is a
        // valid, writable buffer of `epoll_event`; the kernel writes at most `maxevents` entries
        // and returns the count.
        let n: i32 = unsafe {
            ::libc::epoll_wait(
                self.fd.as_raw_fd(),
                events.as_mut_ptr().cast::<::libc::epoll_event>(),
                maxevents,
                timeout_ms,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(&events[..n as usize])
    }

    /// Issues a single `epoll_ctl` call, translating a negative return into an [`io::Error`].
    fn ctl(&self, op: i32, fd: RawFd, token: u64, interest: u32) -> io::Result<()> {
        let mut event: ::libc::epoll_event = ::libc::epoll_event {
            events: interest,
            u64: token,
        };
        // SAFETY: `event` is a valid, initialized `epoll_event`; `op`/`fd` are validated by the
        // kernel, which returns -1 on error.
        let rc: i32 = unsafe { ::libc::epoll_ctl(self.fd.as_raw_fd(), op, fd, &mut event) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl EpollEvent {
    ///
    /// # Description
    ///
    /// Returns a zeroed event, suitable for filling an [`Epoll::wait`] buffer.
    ///
    pub const fn empty() -> Self {
        Self(::libc::epoll_event { events: 0, u64: 0 })
    }

    ///
    /// # Description
    ///
    /// Returns the registration token (the `u64` passed to [`Epoll::add`]/[`Epoll::modify`])
    /// associated with the ready fd.
    ///
    pub fn token(&self) -> u64 {
        self.0.u64
    }

    ///
    /// # Description
    ///
    /// Returns the readiness flags reported for the fd (a bitmask of `EPOLL*`).
    ///
    pub fn events(&self) -> u32 {
        self.0.events
    }
}

impl Default for EpollEvent {
    fn default() -> Self {
        Self::empty()
    }
}

impl AsRawFd for Epoll {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// A registered pipe read-end reports `EPOLLIN` once its write-end has data.
    #[test]
    fn reports_readable_pipe() {
        let mut fds: [RawFd; 2] = [0; 2];
        // SAFETY: `fds` is a valid two-element array the kernel fills with the pipe fds.
        let rc: i32 = unsafe { ::libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() should succeed");
        // SAFETY: both fds are freshly created and owned exclusively by these `OwnedFd`s, which
        // close them exactly once on drop.
        let (read_fd, write_fd): (OwnedFd, OwnedFd) =
            unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };

        let epoll: Epoll = Epoll::new().expect("epoll_create1");
        epoll
            .add(read_fd.as_raw_fd(), 0x1234, EPOLLIN)
            .expect("epoll add");

        // Nothing written yet: a non-blocking wait returns no events.
        let mut buf: [EpollEvent; 4] = [EpollEvent::empty(); 4];
        let ready: &[EpollEvent] = epoll.wait(&mut buf, 0).expect("epoll wait");
        assert!(ready.is_empty(), "no events before writing");

        // SAFETY: `write_fd` is a valid open fd; one byte is written from a valid pointer.
        let byte: u8 = 0x5A;
        let n: ::libc::ssize_t =
            unsafe { ::libc::write(write_fd.as_raw_fd(), (&byte as *const u8).cast(), 1) };
        assert_eq!(n, 1, "write one byte");

        let mut buf: [EpollEvent; 4] = [EpollEvent::empty(); 4];
        let ready: &[EpollEvent] = epoll.wait(&mut buf, 100).expect("epoll wait");
        assert_eq!(ready.len(), 1, "one ready event after writing");
        assert_eq!(ready[0].token(), 0x1234);
        assert_ne!(ready[0].events() & EPOLLIN, 0, "read end is readable");
    }
}
