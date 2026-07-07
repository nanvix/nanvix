// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wire::{
    LENGTH_PREFIX_BYTES,
    MAX_FRAME_BYTES,
};
use ::std::io::{
    Error,
    ErrorKind,
    Result,
};
use ::syscomm::{
    ReadExact,
    SocketStreamReader,
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Reads a single length-prefixed frame body from `reader`.
///
/// Every frame on the wire is a [`LENGTH_PREFIX_BYTES`]-byte little-endian length followed by that
/// many body bytes. This is the single shared reader used by both the `networkd` server and the
/// user-VM client, so the two ends can never drift in how they frame requests and responses.
///
/// # Cancellation
///
/// This function is not cancel-safe if the same connection will be reused after cancellation. It
/// stores in-progress frame state in this future, and the body read uses `ReadExact::read_exact()`,
/// whose contract permits partial reads before cancellation. Await it to completion, or drop the
/// connection with the task. Do not race it in `select!` and then keep using `reader`; use an
/// external incremental reader for that pattern.
///
/// # Parameters
///
/// - `reader`: The read half of the connection.
///
/// # Returns
///
/// - `Ok(Some(body))` with the frame body on success.
/// - `Ok(None)` if the peer disconnected cleanly at a frame boundary.
/// - `Err(_)` on a truncated frame, an oversized frame, or an unexpected transport error.
///
pub async fn read_frame(reader: &mut SocketStreamReader) -> Result<Option<Vec<u8>>> {
    let mut len_buf: [u8; LENGTH_PREFIX_BYTES] = [0; LENGTH_PREFIX_BYTES];

    let mut len_read: usize = 0;
    while len_read < len_buf.len() {
        let n: usize = reader.read(&mut len_buf[len_read..]).await?;
        if n == 0 {
            if len_read == 0 {
                return Ok(None);
            }
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "networkd frame length prefix truncated",
            ));
        }
        len_read += n;
    }

    let len: usize = usize::try_from(u32::from_le_bytes(len_buf))
        .map_err(|_| Error::new(ErrorKind::InvalidData, "networkd frame length overflow"))?;
    if len == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "networkd frame body is empty"));
    }
    if len > MAX_FRAME_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("networkd frame too large: {len} bytes (max {MAX_FRAME_BYTES})"),
        ));
    }

    let mut body: Vec<u8> = vec![0; len];
    reader.read_exact(&mut body).await?;
    Ok(Some(body))
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::{
        fs,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };
    use ::syscomm::{
        SocketListener,
        SocketStream,
        SocketType,
        UnboundSocket,
        WriteAll,
    };

    fn socket_path(name: &str) -> String {
        let mut path: PathBuf = ::std::env::temp_dir();
        let now: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("networkd-{name}-{}-{now}.sock", ::std::process::id()));
        path.to_string_lossy().to_string()
    }

    async fn connected_pair(name: &str) -> (SocketStream, SocketStream, String) {
        let path: String = socket_path(name);
        let listener: SocketListener = UnboundSocket::new(SocketType::Unix)
            .bind(&path)
            .await
            .expect("failed to bind test socket");
        let server = ::tokio::spawn(async move { listener.accept().await.expect("accept failed") });
        let client: SocketStream = UnboundSocket::new(SocketType::Unix)
            .connect(&path)
            .await
            .expect("connect failed");
        let server: SocketStream = server.await.expect("accept task failed");
        (server, client, path)
    }

    #[tokio::test]
    async fn read_frame_returns_none_on_clean_frame_boundary_close() {
        let (server, client, path) = connected_pair("clean-close").await;
        let (mut reader, _writer) = server.split();
        drop(client);

        assert!(read_frame(&mut reader).await.unwrap().is_none());
        let _ = fs::remove_file(path);
    }

    #[tokio::test]
    async fn read_frame_rejects_partial_length_prefix() {
        let (server, mut client, path) = connected_pair("partial-prefix").await;
        let (mut reader, _writer) = server.split();
        client.write_all(&[0x01, 0x00]).await.unwrap();
        drop(client);

        let err = read_frame(&mut reader).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
        let _ = fs::remove_file(path);
    }

    #[tokio::test]
    async fn read_frame_rejects_empty_body() {
        let (server, mut client, path) = connected_pair("empty-body").await;
        let (mut reader, _writer) = server.split();
        client.write_all(&0u32.to_le_bytes()).await.unwrap();
        drop(client);

        let err = read_frame(&mut reader).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        let _ = fs::remove_file(path);
    }
}
