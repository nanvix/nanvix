// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
