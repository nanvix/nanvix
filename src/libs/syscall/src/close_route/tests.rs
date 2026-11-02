// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
