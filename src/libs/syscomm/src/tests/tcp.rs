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
use ::std::io::IoSlice;
use ::tokio::task::JoinHandle;

//==================================================================================================
// Unit Tests
//==================================================================================================

#[tokio::test]
async fn tcp_socket_read_exact_write_all_success() {
    let listener: SocketListener = UnboundSocket::new(SocketType::Tcp)
        .bind("127.0.0.1:0")
        .await
        .expect("bind failed");

    let addr: SocketAddr = match &listener {
        SocketListener::Tcp { listener, .. } => {
            let bound: ::std::net::SocketAddr = listener.local_addr().expect("local_addr failed");
            SocketAddr::Tcp(bound)
        },
        _ => unreachable!(),
    };

    let server: JoinHandle<()> = tokio::spawn(async move {
        let stream: SocketStream = listener.accept().await.expect("accept failed");
        let (mut r, mut w): (SocketStreamReader, SocketStreamWriter) = stream.split();
        let mut buf: [u8; 5] = [0u8; 5];
        r.read_exact(&mut buf[..]).await.expect("read_exact failed");
        w.write_all(&buf).await.expect("write_all failed");
    });

    let connect_target: String = match &addr {
        SocketAddr::Tcp(addr) => addr.to_string(),
        SocketAddr::Unix(_) => unreachable!(),
    };

    let mut client: SocketStream = UnboundSocket::new(SocketType::Tcp)
        .connect(&connect_target)
        .await
        .expect("connect failed");

    client
        .write_all(b"hello")
        .await
        .expect("client write failed");
    let mut buf: [u8; 5] = [0u8; 5];
    client
        .read_exact(&mut buf[..])
        .await
        .expect("client read_exact failed");
    assert_eq!(&buf, b"hello");

    server.await.expect("server task join failed");
}

#[tokio::test]
async fn tcp_socket_read_write_success() {
    let listener: SocketListener = UnboundSocket::new(SocketType::Tcp)
        .bind("127.0.0.1:0")
        .await
        .expect("bind failed");

    let addr: SocketAddr = match &listener {
        SocketListener::Tcp { listener, .. } => {
            let bound: ::std::net::SocketAddr = listener.local_addr().expect("local_addr failed");
            SocketAddr::Tcp(bound)
        },
        _ => unreachable!(),
    };

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

    let connect_target: String = match &addr {
        SocketAddr::Tcp(addr) => addr.to_string(),
        SocketAddr::Unix(_) => unreachable!(),
    };

    let mut client: SocketStream = UnboundSocket::new(SocketType::Tcp)
        .connect(&connect_target)
        .await
        .expect("connect failed");

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

    server.await.expect("server task join failed");
}

#[tokio::test]
async fn tcp_socket_bind_invalid_address() {
    let res: ::std::io::Result<SocketListener> = UnboundSocket::new(SocketType::Tcp)
        .bind("invalid:address")
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn tcp_socket_connect_invalid_address() {
    let res: ::std::io::Result<SocketStream> = UnboundSocket::new(SocketType::Tcp)
        .connect("127.0.0.1:99999")
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn tcp_socket_write_all_vectored_success() {
    let listener: SocketListener = UnboundSocket::new(SocketType::Tcp)
        .bind("127.0.0.1:0")
        .await
        .expect("bind failed");

    let addr: SocketAddr = match &listener {
        SocketListener::Tcp { listener, .. } => {
            let bound: ::std::net::SocketAddr = listener.local_addr().expect("local_addr failed");
            SocketAddr::Tcp(bound)
        },
        _ => unreachable!(),
    };

    let server: JoinHandle<()> = tokio::spawn(async move {
        let stream: SocketStream = listener.accept().await.expect("accept failed");
        let (mut r, _w): (SocketStreamReader, SocketStreamWriter) = stream.split();
        let mut buf: [u8; 10] = [0u8; 10];
        r.read_exact(&mut buf).await.expect("read_exact failed");
        assert_eq!(&buf, b"helloworld");
    });

    let connect_target: String = match &addr {
        SocketAddr::Tcp(addr) => addr.to_string(),
        SocketAddr::Unix(_) => unreachable!(),
    };

    let client: SocketStream = UnboundSocket::new(SocketType::Tcp)
        .connect(&connect_target)
        .await
        .expect("connect failed");

    let (_r, mut w): (SocketStreamReader, SocketStreamWriter) = client.split();
    let a: &[u8] = b"hello";
    let b: &[u8] = b"world";
    w.write_all_vectored(&mut [IoSlice::new(a), IoSlice::new(b)])
        .await
        .expect("write_all_vectored failed");

    server.await.expect("server task join failed");
}

#[tokio::test]
async fn tcp_socket_write_all_vectored_three_slices() {
    let listener: SocketListener = UnboundSocket::new(SocketType::Tcp)
        .bind("127.0.0.1:0")
        .await
        .expect("bind failed");

    let addr: SocketAddr = match &listener {
        SocketListener::Tcp { listener, .. } => {
            let bound: ::std::net::SocketAddr = listener.local_addr().expect("local_addr failed");
            SocketAddr::Tcp(bound)
        },
        _ => unreachable!(),
    };

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

    let connect_target: String = match &addr {
        SocketAddr::Tcp(addr) => addr.to_string(),
        SocketAddr::Unix(_) => unreachable!(),
    };

    let client: SocketStream = UnboundSocket::new(SocketType::Tcp)
        .connect(&connect_target)
        .await
        .expect("connect failed");

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

    server.await.expect("server task join failed");
}
