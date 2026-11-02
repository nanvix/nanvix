// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
