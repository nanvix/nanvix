// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Backend selection for resolved `close()` requests.

//==================================================================================================
// Imports
//==================================================================================================

use crate::fdtable::{
    Resolution,
    Route,
};
use ::sys::{
    ipc::MessageType,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Structures
//==================================================================================================

/// IPC target selected for a resolved `close()` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CloseTarget {
    /// Descriptor number the selected backend must close.
    pub(crate) fd: i32,
    /// Backend process that serves the close request.
    pub(crate) destination: ProcessIdentifier,
    /// Message type expected by the backend.
    pub(crate) message_type: MessageType,
    /// Whether a missing backend is treated as success.
    pub(crate) tolerate_missing_backend: bool,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Chooses the backend and descriptor number for a resolved standalone `close()` request.
pub(crate) fn close_target(fd: i32, resolution: Resolution) -> CloseTarget {
    match resolution.route {
        // VFS-backed descriptors are closed by vfsd using the backend descriptor it reported.
        Route::Vfs => CloseTarget {
            fd: resolution.backend_fd,
            destination: crate::VFS_DESTINATION,
            message_type: crate::VFS_MESSAGE_TYPE,
            tolerate_missing_backend: false,
        },
        // When a guest vfsd exists, a console descriptor also occupies a slot in vfsd's flat table;
        // closing one must release that slot so the descriptor number can be reused. The slot is
        // addressed by the caller-facing flat descriptor, not by the console stream number carried
        // in `backend_fd`. A console alias minted by `fcntl(F_DUPFD)` (e.g. a shell parking stdout
        // at fd 10 while it redirects) shares stream number 1 but lives in slot 10, so closing it
        // must free slot 10. Closing `backend_fd` (1) would instead drop the live console and leave
        // the alias dangling. When no guest vfsd exists (direct-ELF standalone), there is no slot
        // to free, so delivery failure is tolerated and the close is a no-op.
        Route::Console => CloseTarget {
            fd,
            destination: crate::VFS_DESTINATION,
            message_type: crate::VFS_MESSAGE_TYPE,
            tolerate_missing_backend: true,
        },
        // A socket occupies a flat slot owned by vfsd: closing it releases that slot, and vfsd
        // forwards the endpoint close to networkd when the last reference is dropped. The slot is
        // addressed by the caller-facing flat descriptor, not the networkd descriptor `backend_fd`
        // routes I/O to.
        Route::Socket => CloseTarget {
            fd,
            destination: crate::VFS_DESTINATION,
            message_type: crate::VFS_MESSAGE_TYPE,
            tolerate_missing_backend: false,
        },
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::sysapi::unistd::STDOUT_FILENO;

    /// Tests that closing a duplicated console descriptor addresses vfsd's flat slot, not the
    /// console stream number returned as `backend_fd`.
    #[test]
    fn console_alias_close_targets_flat_slot() {
        const ALIAS_FD: i32 = 10;

        let target: CloseTarget = close_target(
            ALIAS_FD,
            Resolution {
                route: Route::Console,
                backend_fd: STDOUT_FILENO,
            },
        );

        assert_eq!(target.fd, ALIAS_FD, "console aliases must close their flat slot");
        assert_eq!(target.destination, crate::VFS_DESTINATION, "console slots live in vfsd");
        assert_eq!(target.message_type, crate::VFS_MESSAGE_TYPE, "vfsd uses its message type");
        assert!(
            target.tolerate_missing_backend,
            "direct-ELF console close must tolerate the absence of guest vfsd"
        );
    }
}
