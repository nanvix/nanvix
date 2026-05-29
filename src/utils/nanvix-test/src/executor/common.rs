// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::tokio::io::{
    AsyncRead,
    AsyncReadExt,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Buffer size used while draining a daemon stdio pipe.
///
/// Sized to match the default Linux pipe-buffer chunk so a single `read` empties one kernel buffer
/// slot without over-allocating on the stack.
const DRAIN_CHUNK_SIZE: usize = 4096;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Reads from an async stream until EOF and discards the bytes. Used to keep daemon stdio
/// pipes from filling up when the executor does not need their contents.
pub(crate) async fn drain_stream<R>(mut reader: R) -> ::std::io::Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut chunk: [u8; DRAIN_CHUNK_SIZE] = [0u8; DRAIN_CHUNK_SIZE];
    loop {
        let bytes_read: usize = reader.read(&mut chunk).await?;
        if bytes_read == 0 {
            break;
        }
    }
    Ok(())
}
