// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// This file contains the messages used in the control-plane API between
// nanvixd and linuxd/user VM.

use ::anyhow::Result;
use ::num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};
use ::std::io::{
    Error,
    ErrorKind,
};
use ::syscomm::{
    // TODO: move back to SocketStream when linuxd polls connections.
    BlockingSocketStream,
    SocketError,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, IntoPrimitive, TryFromPrimitive)]
pub enum Command {
    Shutdown,
}

// This function is actually used by linuxd, consuming nanvixd as a library. It is never used from
// the main nanvixd binary, hence why we need to add this annotation to silent clippy.
#[allow(dead_code)]
pub fn read_command(stream: &mut BlockingSocketStream) -> Result<Command, SocketError> {
    let mut buf: [u8; 1] = [0u8; 1];
    stream.read_exact(&mut buf)?;

    Command::try_from(buf[0]).map_err(|_| {
        Error::new(ErrorKind::InvalidData, "error parsing control-plane command".to_string()).into()
    })
}

pub fn send_command(stream: &mut BlockingSocketStream, cmd: Command) -> Result<(), SocketError> {
    let byte: u8 = cmd.into();
    stream.write_all(&[byte])?;
    Ok(())
}
