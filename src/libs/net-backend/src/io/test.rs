// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::error::NetError;

/// Tests that `send()` rejects a count that exceeds the buffer length.
#[test]
fn send_count_exceeds_buffer() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
    let buf: [u8; 4] = [0; 4];
    let result: Result<isize, NetError> = backend.send(0, &buf, 8, 0);
    assert!(result.is_err(), "send with count > buf.len() should fail");
    match result {
        Err(NetError::Errno(code)) => {
            assert_eq!(code, ErrorCode::InvalidArgument, "error should be InvalidArgument");
        },
        other => panic!("expected Errno(InvalidArgument), got {other:?}"),
    }
}

/// Tests that `recv()` rejects a count that exceeds the buffer length.
#[test]
fn recv_count_exceeds_buffer() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
    let mut buf: [u8; 4] = [0; 4];
    let result: Result<isize, NetError> = backend.recv(0, &mut buf, 8, 0);
    assert!(result.is_err(), "recv with count > buf.len() should fail");
    match result {
        Err(NetError::Errno(code)) => {
            assert_eq!(code, ErrorCode::InvalidArgument, "error should be InvalidArgument");
        },
        other => panic!("expected Errno(InvalidArgument), got {other:?}"),
    }
}
