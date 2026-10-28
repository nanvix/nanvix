// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ReadExact,
    SocketAddr,
    SocketListener,
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::std::{
    io::IoSlice,
    path::{
        Path,
        PathBuf,
    },
};
use ::tokio::task::JoinHandle;

//==================================================================================================
// Unit Tests
//==================================================================================================

#[tokio::test]
async fn unix_socket_read_exact_write_all_success() {
    // Create a unique path in temp dir based on pid and timestamp.
    let mut path: PathBuf = std::env::temp_dir();
    let now: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique: String = format!("sock-test-{}-{}", std::process::id(), now);
    path.push(unique);
    let path: String = path.to_string_lossy().to_string();

    let listener: SocketListener = UnboundSocket::new(SocketType::Unix)
        .bind(&path)
        .await
        .expect("unix bind failed");

    let server: JoinHandle<()> = tokio::spawn(async move {
        let stream: SocketStream = listener.accept().await.expect("accept failed");
        let (mut r, mut w): (SocketStreamReader, SocketStreamWriter) = stream.split();
        let mut buf: [u8; 3] = [0u8; 3];
        r.read_exact(&mut buf).await.expect("read_exact failed");
        w.write_all(&buf).await.expect("write_all failed");
    });

    let mut client: SocketStream = UnboundSocket::new(SocketType::Unix)
        .connect(&path)
        .await
        .expect("unix connect failed");

    client.write_all(b"hey").await.expect("client write failed");
    let mut buf: [u8; 3] = [0u8; 3];
    client
        .read_exact(&mut buf)
        .await
        .expect("client read_exact failed");
    assert_eq!(&buf, b"hey");

    server.await.expect("server join failed");
}

#[tokio::test]
async fn unix_socket_read_write() {
    let mut path: PathBuf = std::env::temp_dir();
    let now: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique: String = format!("sock-rw-test-{}-{}", std::process::id(), now);
    path.push(unique);
    let path: String = path.to_string_lossy().to_string();

    let listener: SocketListener = UnboundSocket::new(SocketType::Unix)
        .bind(&path)
        .await
        .expect("unix bind failed");

    let server: JoinHandle<()> = tokio::spawn(async move {
        let stream: SocketStream = listener.accept().await.expect("accept failed");
        let (mut r, mut w): (SocketStreamReader, SocketStreamWriter) = stream.split();

        let expected: &[u8; 4] = b"ping";
        let mut sent: usize = 0;
        while sent < expected.len() {
            let wrote: usize = w.write(&expected[sent..]).await.expect("write failed");
            assert!(wrote > 0);
            sent += wrote;
        }

        let mut buf: [u8; 4] = [0u8; 4];
        let mut received: usize = 0;
        while received < buf.len() {
            let read: usize = r.read(&mut buf[received..]).await.expect("read failed");
            assert!(read > 0);
            received += read;
        }
        assert_eq!(&buf, b"pong");
    });

    let mut client: SocketStream = UnboundSocket::new(SocketType::Unix)
        .connect(&path)
        .await
        .expect("unix connect failed");

    let mut buf: [u8; 4] = [0u8; 4];
    let mut received: usize = 0;
    while received < buf.len() {
        let read: usize = client
            .read(&mut buf[received..])
            .await
            .expect("client read failed");
        assert!(read > 0);
        received += read;
    }
    assert_eq!(&buf, b"ping");

    let expected: &[u8; 4] = b"pong";
    let mut sent: usize = 0;
    while sent < expected.len() {
        let wrote: usize = client
            .write(&expected[sent..])
            .await
            .expect("client write failed");
        assert!(wrote > 0);
        sent += wrote;
    }

    server.await.expect("server join failed");
}

#[tokio::test]
async fn unix_connect_missing() {
    let res: ::std::io::Result<SocketStream> = UnboundSocket::new(SocketType::Unix)
        .connect("/nonexistent/path.sock")
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn unix_socket_write_all_vectored_success() {
    let mut path: PathBuf = std::env::temp_dir();
    let now: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique: String = format!("sock-vec-test-{}-{}", std::process::id(), now);
    path.push(unique);
    let path: String = path.to_string_lossy().to_string();

    let listener: SocketListener = UnboundSocket::new(SocketType::Unix)
        .bind(&path)
        .await
        .expect("unix bind failed");

    let server: JoinHandle<()> = tokio::spawn(async move {
        let stream: SocketStream = listener.accept().await.expect("accept failed");
        let (mut r, _w): (SocketStreamReader, SocketStreamWriter) = stream.split();
        let mut buf: [u8; 10] = [0u8; 10];
        r.read_exact(&mut buf).await.expect("read_exact failed");
        assert_eq!(&buf, b"helloworld");
    });

    let client: SocketStream = UnboundSocket::new(SocketType::Unix)
        .connect(&path)
        .await
        .expect("unix connect failed");

    let (_r, mut w): (SocketStreamReader, SocketStreamWriter) = client.split();
    let a: &[u8] = b"hello";
    let b: &[u8] = b"world";
    w.write_all_vectored(&mut [IoSlice::new(a), IoSlice::new(b)])
        .await
        .expect("write_all_vectored failed");

    server.await.expect("server join failed");
}

#[tokio::test]
async fn unix_socket_write_all_vectored_three_slices() {
    let mut path: PathBuf = std::env::temp_dir();
    let now: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique: String = format!("sock-vec3-test-{}-{}", std::process::id(), now);
    path.push(unique);
    let path: String = path.to_string_lossy().to_string();

    let listener: SocketListener = UnboundSocket::new(SocketType::Unix)
        .bind(&path)
        .await
        .expect("unix bind failed");

    let server: JoinHandle<()> = tokio::spawn(async move {
        let stream: SocketStream = listener.accept().await.expect("accept failed");
        let (mut r, _w): (SocketStreamReader, SocketStreamWriter) = stream.split();
        // Frame type (1) + length prefix (4) + payload (6) = 11 bytes.
        let mut buf: [u8; 11] = [0u8; 11];
        r.read_exact(&mut buf).await.expect("read_exact failed");
        assert_eq!(buf[0], 0x01);
        assert_eq!(&buf[1..5], &6u32.to_le_bytes());
        assert_eq!(&buf[5..], b"abcdef");
    });

    let client: SocketStream = UnboundSocket::new(SocketType::Unix)
        .connect(&path)
        .await
        .expect("unix connect failed");

    let (_r, mut w): (SocketStreamReader, SocketStreamWriter) = client.split();
    let frame_type: [u8; 1] = [0x01];
    let len_prefix: [u8; 4] = 6u32.to_le_bytes();
    let payload: &[u8] = b"abcdef";
    w.write_all_vectored(&mut [
        IoSlice::new(&frame_type),
        IoSlice::new(&len_prefix),
        IoSlice::new(payload),
    ])
    .await
    .expect("write_all_vectored failed");

    server.await.expect("server join failed");
}

#[tokio::test]
async fn unix_socket_peer_addr_success() {
    let mut path: PathBuf = std::env::temp_dir();
    let now: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique: String = format!("sock-peer-test-{}-{}", std::process::id(), now);
    path.push(unique);
    let path: String = path.to_string_lossy().to_string();

    let listener: SocketListener = UnboundSocket::new(SocketType::Unix)
        .bind(&path)
        .await
        .expect("unix bind failed");

    let server: JoinHandle<SocketStream> =
        tokio::spawn(async move { listener.accept().await.expect("accept failed") });

    let client: SocketStream = UnboundSocket::new(SocketType::Unix)
        .connect(&path)
        .await
        .expect("unix connect failed");
    let server: SocketStream = server.await.expect("server join failed");

    // The client is connected to the path that the listener is bound to.
    match client.peer_addr().expect("client peer_addr failed") {
        SocketAddr::Unix(peer) => assert_eq!(peer.as_pathname(), Some(Path::new(&path))),
        SocketAddr::Tcp(_) => unreachable!(),
    }

    // The client is not bound to any path, so the server is connected to an unnamed peer.
    match server.peer_addr().expect("server peer_addr failed") {
        SocketAddr::Unix(peer) => assert!(peer.is_unnamed()),
        SocketAddr::Tcp(_) => unreachable!(),
    }
}

#[tokio::test]
async fn unix_socket_shutdown_write_success() {
    let mut path: PathBuf = std::env::temp_dir();
    let now: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique: String = format!("sock-shutdown-test-{}-{}", std::process::id(), now);
    path.push(unique);
    let path: String = path.to_string_lossy().to_string();

    let listener: SocketListener = UnboundSocket::new(SocketType::Unix)
        .bind(&path)
        .await
        .expect("unix bind failed");

    let server: JoinHandle<()> = tokio::spawn(async move {
        let mut stream: SocketStream = listener.accept().await.expect("accept failed");
        let mut buf: [u8; 3] = [0u8; 3];
        stream
            .read_exact(&mut buf)
            .await
            .expect("read_exact failed");
        assert_eq!(&buf, b"bye");
        // The peer shut down its write half, so this read observes the end of the stream.
        let read: usize = stream.read(&mut buf).await.expect("read failed");
        assert_eq!(read, 0);
        stream.write_all(b"ok").await.expect("write_all failed");
    });

    let mut client: SocketStream = UnboundSocket::new(SocketType::Unix)
        .connect(&path)
        .await
        .expect("unix connect failed");

    client
        .write_all(b"bye")
        .await
        .expect("client write_all failed");
    client
        .shutdown_write()
        .await
        .expect("client shutdown_write failed");
    assert!(client.write(b"!").await.is_err());

    // Shutting down the write half leaves the read half open.
    let mut buf: [u8; 2] = [0u8; 2];
    client
        .read_exact(&mut buf)
        .await
        .expect("client read_exact failed");
    assert_eq!(&buf, b"ok");

    server.await.expect("server join failed");
}
