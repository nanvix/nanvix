// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
        assert_eq!(select_timeout(&Some(timeout)).unwrap_err().code, ErrorCode::InvalidArgument);
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
        update_sets(&poll_fds, &interests, &events, Some(&mut readfds), Some(&mut writefds), None,)
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
