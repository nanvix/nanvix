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
    SocketError,
    SocketStream,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, IntoPrimitive, TryFromPrimitive)]
pub enum Command {
    Shutdown,
}

// This function is actually used by linuxd, consuming nanvixd as a library. It is never used from
// the main nanvixd binary, hence why we need to add this annotation to silent clippy.
#[allow(dead_code)]
pub fn try_read_command(stream: &mut SocketStream) -> Result<Command, SocketError> {
    let mut buf: [u8; 1] = [0u8; 1];

    // Try read exact returns the number of bytes read `n` with 0 < n <= buf.len(). In this case
    // it is safe to ignore the return value because n can only ever be 1.
    let num_read = stream.try_read_exact(&mut buf)?;
    debug_assert!(num_read == 1);

    Command::try_from(buf[0]).map_err(|_| {
        Error::new(ErrorKind::InvalidData, "error parsing control-plane command".to_string()).into()
    })
}

#[allow(dead_code)]
pub fn send_command(stream: &mut SocketStream, cmd: Command) -> Result<(), SocketError> {
    let byte: u8 = cmd.into();
    stream.write_all(&[byte])?;
    Ok(())
}
