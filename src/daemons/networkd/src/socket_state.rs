// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Per-socket state tracked by the epoll reactor.
//!
//! Each host socket a session owns has an associated [`SocketState`] that records the operations
//! currently parked on it awaiting readiness, plus the `epoll` interest mask currently armed for
//! the socket. A socket is registered with the reactor's `epoll` instance **iff** it has at least
//! one parked operation (i.e. its interest is non-zero); this keeps idle sockets out of `epoll` so
//! always-reported conditions such as `EPOLLHUP`/`EPOLLERR` cannot wake the reactor in a busy loop.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    epoll::{
        EPOLLIN,
        EPOLLOUT,
    },
    ops::Direction,
    wire::NetworkOp,
};
use ::std::{
    collections::VecDeque,
    os::fd::RawFd,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// The reactor's bookkeeping for a single host socket owned by a session.
///
pub struct SocketState {
    /// The host file descriptor this state tracks.
    pub host_fd: RawFd,
    /// The `epoll` interest mask currently armed for this socket. `0` means the socket is not
    /// registered with the reactor's `epoll` instance.
    pub interest: u32,
    /// Operations parked awaiting readability, in arrival order.
    pub parked_read: VecDeque<NetworkOp>,
    /// Operations parked awaiting writability, in arrival order.
    pub parked_write: VecDeque<NetworkOp>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SocketState {
    ///
    /// # Description
    ///
    /// Creates tracking state for a freshly opened, not-yet-registered host socket.
    ///
    pub fn new(host_fd: RawFd) -> Self {
        Self {
            host_fd,
            interest: 0,
            parked_read: VecDeque::new(),
            parked_write: VecDeque::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Parks `op` on this socket in the given readiness `dir`ection.
    ///
    pub fn park(&mut self, dir: Direction, op: NetworkOp) {
        match dir {
            Direction::Read => self.parked_read.push_back(op),
            Direction::Write => self.parked_write.push_back(op),
        }
    }

    ///
    /// # Description
    ///
    /// Re-parks `op` at the front of the given readiness `dir`ection, used when a resumed operation
    /// still could not complete and must retain its position at the head of the queue.
    ///
    pub fn repark_front(&mut self, dir: Direction, op: NetworkOp) {
        match dir {
            Direction::Read => self.parked_read.push_front(op),
            Direction::Write => self.parked_write.push_front(op),
        }
    }

    ///
    /// # Description
    ///
    /// Pops the next operation parked on the given readiness `dir`ection, if any.
    ///
    pub fn pop(&mut self, dir: Direction) -> Option<NetworkOp> {
        match dir {
            Direction::Read => self.parked_read.pop_front(),
            Direction::Write => self.parked_write.pop_front(),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the `epoll` interest mask this socket should currently be armed with, derived from
    /// whether it has operations parked in each direction.
    ///
    pub fn desired_interest(&self) -> u32 {
        let mut interest: u32 = 0;
        if !self.parked_read.is_empty() {
            interest |= EPOLLIN;
        }
        if !self.parked_write.is_empty() {
            interest |= EPOLLOUT;
        }
        interest
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::sys::pm::ThreadIdentifier;
    use ::syscall::sys::socket::message::ReceiveSocketRequest;

    /// Builds a distinct dummy operation tagged with `tid` for queue-ordering assertions.
    fn op(tid: i32) -> NetworkOp {
        NetworkOp::Message(ReceiveSocketRequest::build(ThreadIdentifier::from(tid), 0, 0, 0))
    }

    /// A freshly created socket has no parked operations and no `epoll` interest.
    #[test]
    fn new_socket_is_idle() {
        let sock: SocketState = SocketState::new(7);
        assert_eq!(sock.host_fd, 7);
        assert_eq!(sock.interest, 0);
        assert_eq!(sock.desired_interest(), 0);
        assert!(sock.parked_read.is_empty());
        assert!(sock.parked_write.is_empty());
    }

    /// Parking in each direction is reflected in the desired `epoll` interest mask.
    #[test]
    fn desired_interest_tracks_parked_directions() {
        let mut sock: SocketState = SocketState::new(3);

        sock.park(Direction::Read, op(1));
        assert_eq!(sock.desired_interest(), EPOLLIN);

        sock.park(Direction::Write, op(2));
        assert_eq!(sock.desired_interest(), EPOLLIN | EPOLLOUT);
    }

    /// Operations parked in a direction are popped in arrival (FIFO) order.
    #[test]
    fn park_pops_in_fifo_order() {
        let mut sock: SocketState = SocketState::new(3);
        sock.park(Direction::Read, op(1));
        sock.park(Direction::Read, op(2));

        assert_eq!(sock.pop(Direction::Read).map(|o| i32::from(o.tid())), Some(1));
        assert_eq!(sock.pop(Direction::Read).map(|o| i32::from(o.tid())), Some(2));
        assert!(sock.pop(Direction::Read).is_none());
    }

    /// Re-parking restores an operation to the head of its queue ahead of any already parked.
    #[test]
    fn repark_front_restores_head_of_queue() {
        let mut sock: SocketState = SocketState::new(3);
        sock.park(Direction::Write, op(2));
        sock.repark_front(Direction::Write, op(1));

        assert_eq!(sock.pop(Direction::Write).map(|o| i32::from(o.tid())), Some(1));
        assert_eq!(sock.pop(Direction::Write).map(|o| i32::from(o.tid())), Some(2));
    }

    /// Popping the only parked operation drops the corresponding interest bit.
    #[test]
    fn desired_interest_clears_when_drained() {
        let mut sock: SocketState = SocketState::new(3);
        sock.park(Direction::Read, op(1));
        assert_eq!(sock.desired_interest(), EPOLLIN);

        sock.pop(Direction::Read);
        assert_eq!(sock.desired_interest(), 0);
    }
}
