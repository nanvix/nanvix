// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ReadBackend;
use crate::fd_route::Route;
use ::sys::error::ErrorCode;

#[test]
fn terminal_reads_use_console_backend() {
    for route in [Route::Console, Route::Terminal] {
        assert!(matches!(ReadBackend::try_from(route), Ok(ReadBackend::VfsConsole)));
    }
}

#[test]
fn other_vfs_reads_use_vfs_backend() {
    assert!(matches!(ReadBackend::try_from(Route::Vfs), Ok(ReadBackend::Vfs)));
}

#[test]
fn sockets_are_not_read_backends() {
    assert!(matches!(
        ReadBackend::try_from(Route::Socket),
        Err(error) if error.code == ErrorCode::BadFile
    ));
}
