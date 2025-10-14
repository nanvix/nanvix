// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::*;
use ::anyhow::Result as AnyResult;
use ::std::io::ErrorKind;
use ::tokio::{
    io::AsyncWriteExt,
    net::UnixStream,
};

//==================================================================================================
// Unit Tests
//==================================================================================================

#[tokio::test]
async fn send_and_recv_round_trip() -> AnyResult<()> {
    let (sender_raw, receiver_raw): (UnixStream, UnixStream) = UnixStream::pair()?;
    let mut sender: SocketStream = SocketStream::Unix(sender_raw);
    let mut receiver: SocketStream = SocketStream::Unix(receiver_raw);
    let message: NewUserVm = NewUserVm::new(
        UserVmIdentifier::new(42u32),
        String::from("unix:///tmp/user-vm"),
        SocketType::Unix,
    );

    message.send(&mut sender).await?;
    let received: NewUserVm = NewUserVm::recv(&mut receiver).await?;

    assert_eq!(received.id(), message.id());
    assert_eq!(received.gateway_sockaddr(), message.gateway_sockaddr());
    assert!(matches!(received.gateway_socket_type(), SocketType::Unix));

    Ok(())
}

#[tokio::test]
async fn recv_rejects_invalid_length() -> AnyResult<()> {
    let (mut writer_raw, reader_raw): (UnixStream, UnixStream) = UnixStream::pair()?;
    let len_bytes: [u8; ::core::mem::size_of::<u32>()] = 0u32.to_le_bytes();
    writer_raw.write_all(&len_bytes).await?;

    let mut reader: SocketStream = SocketStream::Unix(reader_raw);
    let result: Result<NewUserVm> = NewUserVm::recv(&mut reader).await;

    assert!(matches!(
        result,
        Err(ref error) if error.kind() == ErrorKind::InvalidData
    ));

    Ok(())
}

#[tokio::test]
async fn recv_rejects_malformed_payload() -> AnyResult<()> {
    let (mut writer_raw, reader_raw): (UnixStream, UnixStream) = UnixStream::pair()?;
    let len_bytes: [u8; ::core::mem::size_of::<u32>()] = 4u32.to_le_bytes();
    let garbage: [u8; 4] = [0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8];
    writer_raw.write_all(&len_bytes).await?;
    writer_raw.write_all(&garbage).await?;

    let mut reader: SocketStream = SocketStream::Unix(reader_raw);
    let result: Result<NewUserVm> = NewUserVm::recv(&mut reader).await;

    assert!(matches!(
        result,
        Err(ref error) if error.kind() == ErrorKind::InvalidData
    ));

    Ok(())
}

#[tokio::test]
async fn send_propagates_stream_failure() -> AnyResult<()> {
    let (sender_raw, receiver_raw): (UnixStream, UnixStream) = UnixStream::pair()?;
    drop(receiver_raw);
    let mut sender: SocketStream = SocketStream::Unix(sender_raw);
    let message: NewUserVm = NewUserVm::new(
        UserVmIdentifier::new(7u32),
        String::from("unix:///tmp/user-vm-fail"),
        SocketType::Unix,
    );

    let result: Result<()> = message.send(&mut sender).await;

    assert!(matches!(
        result,
        Err(ref error)
            if error.kind() == ErrorKind::BrokenPipe
                || error.kind() == ErrorKind::ConnectionReset
                || error.kind() == ErrorKind::ConnectionAborted
    ));

    Ok(())
}
