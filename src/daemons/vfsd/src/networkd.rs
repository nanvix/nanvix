// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! networkd forwarding module for vfsd.
//!
//! Sockets are flat descriptors in vfsd's slot table, but their I/O backend is networkd. When the
//! last reference to a socket slot is dropped — by `close`, a `dup2` that displaces it, or process
//! exit/exec — vfsd forwards the endpoint close to networkd so the remote descriptor does not leak.
//!
//! This mirrors the hostfs orphan-close pattern: the request is fire-and-forget (the closing
//! process is gone or does not wait on it), and networkd's acknowledgement is discarded by the main
//! event loop. The close is addressed to [`ProcessIdentifier::NETWORKD`] as an IKC message, exactly
//! as a guest socket close would be.

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
use ::syscall::unistd::message::CloseRequest;

/// Forwards a socket endpoint close to networkd, best-effort.
///
/// `remote_fd` is the descriptor networkd assigned to the socket (the value stored in the slot's
/// socket handle). The close is fire-and-forget: vfsd does not block on networkd's acknowledgement,
/// which arrives later as an IKC `CloseResponse` and is discarded by the main event loop. The
/// request is sent from vfsd's thread identifier so networkd routes its reply back to vfsd.
pub fn send_close_request(remote_fd: i32) -> Result<(), ErrorCode> {
    let request: Message = CloseRequest::build(
        ThreadIdentifier::VFSD,
        remote_fd,
        ProcessIdentifier::NETWORKD,
        MessageType::Ikc,
    );
    match ::sys::kcall::ipc::__kcall_send(&request) {
        Ok(()) => Ok(()),
        Err(e) => {
            ::syslog::error!(
                "networkd: failed to send close request (remote_fd={}, error={:?})",
                remote_fd,
                e
            );
            Err(ErrorCode::IoErr)
        },
    }
}
