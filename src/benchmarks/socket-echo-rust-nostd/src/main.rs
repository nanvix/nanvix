// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::alloc::vec::Vec;
use ::core::sync::atomic::Ordering;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;
use ::syscall::{
    netinet::in_::{
        Ipv4Addr,
        Protocol,
        SocketAddrV4,
    },
    sys::socket::{
        syscall::{
            accept,
            bind,
            listen,
            recv,
            send,
            socket,
        },
        AddressFamily,
        SocketAddr,
        SocketType,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Loopback port on which the echo server binds when no port argument is provided.
const DEFAULT_ECHO_PORT: u16 = 34254;

/// Number of bytes in the host-to-guest length prefix.
const LENGTH_PREFIX_SIZE: usize = 8;

/// Largest payload accepted by the benchmark protocol.
const MAX_PAYLOAD_SIZE: usize = 64 * 1024;

/// Maximum number of pending connections on the listening socket.
const LISTEN_BACKLOG: c_int = 1;

/// Flags passed to `recv()`/`send()`.
const NO_FLAGS: c_int = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs a TCP stream echo server over a network socket.
///
/// The server binds a TCP socket to the loopback port passed as its first argument (or to a default
/// port when omitted), accepts one connection at a time, and echoes length-prefixed payloads.
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let echo_port: u16 = echo_port()?;
    let sockfd: c_int = socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)?;

    let bind_addr: SocketAddr =
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new([127, 0, 0, 1]), echo_port));
    bind(sockfd, &bind_addr)?;
    listen(sockfd, LISTEN_BACKLOG)?;

    let mut length_prefix: [u8; LENGTH_PREFIX_SIZE] = [0; LENGTH_PREFIX_SIZE];
    let mut buffer: Vec<u8> = Vec::new();

    loop {
        let (client_fd, _client_addr): (c_int, SocketAddr) = accept(sockfd)?;

        loop {
            if !recv_exact(client_fd, &mut length_prefix)? {
                break;
            }

            let payload_size: usize = match usize::try_from(u64::from_be_bytes(length_prefix)) {
                Ok(0) | Err(_) => break,
                Ok(payload_size) if payload_size > MAX_PAYLOAD_SIZE => break,
                Ok(payload_size) => payload_size,
            };

            if buffer.len() < payload_size {
                buffer.resize(payload_size, 0);
            }

            if !recv_exact(client_fd, &mut buffer[..payload_size])? {
                break;
            }

            if !send_all(client_fd, &buffer[..payload_size])? {
                break;
            }
        }

        ::syscall::unistd::close(client_fd)?;
    }
}

/// Returns the TCP port passed as the first argument, or the default port when omitted.
fn echo_port() -> Result<u16, Error> {
    let argc: i32 = nvx_crt0::ARGC.load(Ordering::SeqCst);
    let argv: *mut *const u8 = nvx_crt0::ARGV.load(Ordering::SeqCst);

    if argc <= 1 {
        return Ok(DEFAULT_ECHO_PORT);
    }
    if argv.is_null() {
        return Err(Error::new(ErrorCode::InvalidArgument, "missing argv"));
    }

    let port_arg: *const u8 = unsafe { *argv.add(1) };
    parse_port(port_arg)
}

/// Parses a null-terminated decimal TCP port argument.
fn parse_port(arg: *const u8) -> Result<u16, Error> {
    if arg.is_null() {
        return Err(Error::new(ErrorCode::InvalidArgument, "missing port argument"));
    }

    let mut value: u32 = 0;
    let mut len: usize = 0;
    loop {
        let byte: u8 = unsafe { *arg.add(len) };
        if byte == 0 {
            break;
        }
        if !byte.is_ascii_digit() {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid port argument"));
        }

        value = match value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u32::from(byte - b'0')))
        {
            Some(value) => value,
            None => {
                return Err(Error::new(ErrorCode::InvalidArgument, "port argument out of range"))
            },
        };
        if value > u32::from(u16::MAX) {
            return Err(Error::new(ErrorCode::InvalidArgument, "port argument out of range"));
        }
        len += 1;
    }

    if len == 0 || value == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument, "invalid port argument"));
    }

    Ok(value as u16)
}

/// Reads exactly `buffer.len()` bytes from a connected TCP socket.
fn recv_exact(sockfd: c_int, buffer: &mut [u8]) -> Result<bool, Error> {
    let mut received: usize = 0;

    while received < buffer.len() {
        let nread: usize = match recv(sockfd, &mut buffer[received..], NO_FLAGS) {
            Ok(0) => return Ok(false),
            Ok(nread) => nread,
            Err(error) => return Err(error),
        };
        received += nread;
    }

    Ok(true)
}

/// Writes all bytes in `buffer` to a connected TCP socket.
fn send_all(sockfd: c_int, buffer: &[u8]) -> Result<bool, Error> {
    let mut sent: usize = 0;

    while sent < buffer.len() {
        let nwritten: usize = match send(sockfd, &buffer[sent..], NO_FLAGS) {
            Ok(0) => return Ok(false),
            Ok(nwritten) => nwritten,
            Err(error) => return Err(error),
        };
        sent += nwritten;
    }

    Ok(true)
}
