// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![forbid(clippy::all)]
#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

/// Wasm engine.
mod engine;

/// Platform abstraction layer.
mod pal;

mod memory;
mod wasi;

/// Static information for an embedded WASM binary.
#[cfg(feature = "wasm_binary")]
mod wasm_binary;

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use self::engine::WasmEngine;
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::core::str::FromStr;
use ::nvx::sys::error::Error;
use ::posix::sys::socket::{
    SocketAddr,
    SocketAddrV4,
};

#[no_mangle]
fn fminf(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmax(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmin(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmaxf(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmod(a: f64, b: f64) -> f64 {
    a % b
}

#[no_mangle]
fn fmodf(a: f32, b: f32) -> f32 {
    a % b
}

//==================================================================================================
// Constants
//==================================================================================================

/// Socket address to which the WASM Daemon listens.
const WASMD_SOCKET_ADDR: Option<&str> = option_env!("NANVIX_WASMD_SOCKADDR");

//==================================================================================================
// Structures
//==================================================================================================

struct WasmBinary {
    name: String,
    args: Vec<String>,
    bytes: Vec<u8>,
}

impl core::fmt::Debug for WasmBinary {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "WasmBinary {{ name: {}, args: {:?}, size: {:?} }}",
            self.name,
            self.args,
            self.bytes.len()
        )
    }
}

impl WasmBinary {
    #[cfg(not(feature = "wasm_binary"))]
    pub fn new() -> Self {
        use ::core::mem;
        use ::posix::{
            ffi::c_int,
            netinet::in_::{
                in_addr,
                sockaddr_in,
            },
            sys::{
                self,
                socket::{
                    self,
                    sockaddr,
                    socklen_t,
                    SocketAddr,
                },
                types::{
                    size_t,
                    ssize_t,
                },
            },
            unistd,
        };

        let sockfd: c_int = match socket::socket(socket::AF_INET as c_int, socket::SOCK_STREAM, 0) {
            sockfd if sockfd >= 0 => sockfd,
            errno => {
                panic!("failed to create socket (errno={})", errno);
            },
        };

        // Bind socket to address to 127.0.0.1:8080.
        let sockaddr_in: sockaddr_in = sockaddr_in {
            sin_family: sys::socket::AF_INET,
            sin_port: u16::to_be(8080),
            sin_addr: in_addr {
                s_addr: u32::from_be_bytes([127, 0, 0, 1]).to_be(),
            },
            sin_zero: [0; 8],
        };

        let sockaddr: SocketAddr = SocketAddr::V4(sockaddr_in.into());

        match sys::socket::bind(sockfd, &sockaddr) {
            0 => {
                ::nvx::log!("bound socket to address");
            },
            errno => {
                panic!("failed to bind socket to address: {:?}", errno);
            },
        }

        // Listen for connections on socket.
        match sys::socket::listen(sockfd, 0) {
            0 => {
                ::nvx::log!("listening for connections on socket");
            },
            errno => {
                panic!("failed to listen for connections on socket: {:?}", errno);
            },
        }

        // Accept connection on socket.
        let mut address: sockaddr = unsafe { core::mem::zeroed() };
        let mut address_len: socklen_t = 0;
        let connfd: i32 = match sys::socket::accept(sockfd, &mut address) {
            connfd if connfd >= 0 => {
                ::nvx::log!("accepted connection on socket with fd {}", connfd);
                connfd
            },
            errno => {
                panic!("failed to accept connection on socket: {:?}", errno);
            },
        };

        // Read payload size.
        let mut payload_buffer: [u8; core::mem::size_of::<u32>()] =
            [0; core::mem::size_of::<u32>()];
        let payload_size = match socket::recv(connfd, &mut payload_buffer, 0) {
            n if n == core::mem::size_of::<u32>() as ssize_t => u32::from_le_bytes(payload_buffer),
            errno => {
                panic!("failed to receive payload size: {:?}", errno);
            },
        };
        ::nvx::log!("received payload size: {}", payload_size);

        // Read payload.
        let mut wasm_bytes: Vec<u8> = alloc::vec![0; payload_size as usize];
        match socket::recv(connfd, &mut wasm_bytes, 0) {
            n if n == payload_size as ssize_t => {
                ::nvx::log!("received payload");
            },
            errno => {
                panic!("failed to receive payload: {:?}", errno);
            },
        }

        // Close connection.
        match unistd::close(connfd) {
            0 => {
                ::nvx::log!("closed connection");
            },
            errno => {
                panic!("failed to close connection: {:?}", errno);
            },
        }

        wasm_bytes.shrink_to_fit();
        ::nvx::log!("loading wasm file ({} bytes)", wasm_bytes.len());

        // TODO: receive name and args from remote.
        // TODO: skip allocation of bytes and use static array instead.
        Self {
            name: "a.wasm".to_string(),
            args: Vec::new(),
            bytes: wasm_bytes,
        }
    }

    #[cfg(feature = "wasm_binary")]
    pub fn new() -> Self {
        use ::alloc::string::ToString;
        let args: Vec<String> = wasm_binary::WASM_BINARY_ARGS
            .iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            name: wasm_binary::WASM_BINARY_NAME.to_string(),
            args,
            bytes: wasm_binary::WASM_BYTES.to_vec(),
        }
    }
}

#[no_mangle]
fn main() -> Result<(), Error> {
    ::nvx::log!("initializing wasm daemon...");

    let wasm_binary = WasmBinary::new();

    ::nvx::log!("wasm file loaded {:?}", wasm_binary);

    let sockaddr: Option<SocketAddr> = match WASMD_SOCKET_ADDR {
        Some(sockaddr) => match SocketAddrV4::from_str(sockaddr) {
            Ok(sockaddr) => Some(SocketAddr::V4(sockaddr)),
            Err(error) => {
                ::nvx::log!("{:?}", error);
                None
            },
        },
        None => None,
    };

    let mut engine: WasmEngine = WasmEngine::new(&wasm_binary, 42, &sockaddr);

    engine.run();

    ::nvx::log!("shutting down wasm daemon...");

    Ok(())
}
