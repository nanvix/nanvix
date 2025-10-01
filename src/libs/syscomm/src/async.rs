// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! This module contains async wrappers around the methods in the syscomm library. It is meant to
//! be used in places in the codebase where we are using asynchronous tasks.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SocketListener,
    SocketStream,
};
use ::std::{
    io::{
        self,
        ErrorKind,
    },
    os::fd::AsRawFd,
};
use ::tokio::io::unix::{
    AsyncFd,
    AsyncFdReadyGuard,
};

///
/// # Description
///
/// Asynchronous read exact implementation.
///
/// # Parameters
///
/// - `stream`: A mutable, non-blocking, mio socket stream.
/// - `buf`: A mutable buffer.
///
/// # Returns
///
/// The number of bytes read into the buffer.
///
pub async fn read_exact(stream: &mut SocketStream, mut buf: &mut [u8]) -> io::Result<()> {
    let afd: AsyncFd<i32> = AsyncFd::new(stream.as_raw_fd())?;

    while !buf.is_empty() {
        let mut guard: AsyncFdReadyGuard<'_, i32> = afd.readable().await?;

        match stream.try_read(buf) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => {
                // Move the pointer in the mutable buffer by `n`.
                let (_, rest): (&mut [u8], &mut [u8]) = buf.split_at_mut(n);
                buf = rest;

                // Keep the readiness for possible more bytes in buffer.
            },
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // Consume the readiness and wait again.
                guard.clear_ready();
            },
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

///
/// # Description
///
/// Asynchronous version of the accept call to accept a connection on a socket listener.
///
/// # Parameters
///
/// - `listener`: A non-blocking mio socket listener.
///
/// # Returns
///
/// A new non-blocking mio socket stream, or an error.
///
pub async fn accept(listener: &SocketListener) -> io::Result<SocketStream> {
    let afd: AsyncFd<i32> = AsyncFd::new(listener.as_raw_fd())?;

    loop {
        // Try accepting straight away.
        match listener.accept() {
            Ok(stream) => return Ok(stream),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {},
            Err(e) => return Err(e.into()),
        }

        // Wait until readable again.
        let mut guard: AsyncFdReadyGuard<'_, i32> = afd.readable().await?;
        guard.clear_ready();
    }
}
