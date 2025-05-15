// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
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
use ::posix::{
    netinet::in_::SocketAddrV4,
    sys::socket::SocketAddr,
};

#[unsafe(no_mangle)]
fn fminf(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

#[unsafe(no_mangle)]
fn fmax(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

#[unsafe(no_mangle)]
fn fmin(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

#[unsafe(no_mangle)]
fn fmaxf(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

#[unsafe(no_mangle)]
fn fmod(a: f64, b: f64) -> f64 {
    a % b
}

#[unsafe(no_mangle)]
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
        use ::alloc::string::ToString;
        use ::core::mem;
        use ::posix::sys::socket::SocketAddr;
        use pal::socket::Socket;

        let sockaddr: SocketAddr = match WASMD_SOCKET_ADDR {
            Some(sockaddr) => match SocketAddrV4::from_str(sockaddr) {
                Ok(sockaddr) => SocketAddr::V4(sockaddr),
                Err(error) => {
                    panic!("failed to parse socket address: {:?}", error);
                },
            },
            None => {
                panic!("socket address not set");
            },
        };

        let socket: Socket = match pal::setup_network(&sockaddr) {
            Ok(socket) => socket,
            Err(error) => {
                panic!("failed to setup network: {:?}", error);
            },
        };

        // Read payload size.
        let mut payload_buffer: [u8; core::mem::size_of::<u32>()] =
            [0; core::mem::size_of::<u32>()];

        let payload_size: u32 = match socket.recv(&mut payload_buffer) {
            Ok(n) if n == mem::size_of::<u32>() => u32::from_le_bytes(payload_buffer),
            Ok(n) => {
                panic!(
                    "failed to receive payload size: expected {} bytes, got {}",
                    mem::size_of::<u32>(),
                    n
                );
            },
            Err(error) => {
                panic!("failed to receive payload size: {:?}", error);
            },
        };
        ::syslog::info!("received payload size: {}", payload_size);

        // Read payload.
        let mut wasm_bytes: Vec<u8> = alloc::vec![0; payload_size as usize];
        match socket.recv(&mut wasm_bytes) {
            Ok(n) if n == payload_size as usize => {
                ::syslog::info!("received payload");
            },
            errno => {
                panic!("failed to receive payload: {:?}", errno);
            },
        }

        wasm_bytes.shrink_to_fit();
        ::syslog::info!("loading wasm file ({} bytes)", wasm_bytes.len());

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

#[unsafe(no_mangle)]
fn main() -> Result<(), Error> {
    ::syslog::info!("initializing wasm daemon...");

    let wasm_binary = WasmBinary::new();

    ::syslog::info!("wasm file loaded {:?}", wasm_binary);

    let sockaddr: Option<SocketAddr> = match WASMD_SOCKET_ADDR {
        Some(sockaddr) => match SocketAddrV4::from_str(sockaddr) {
            Ok(sockaddr) => Some(SocketAddr::V4(sockaddr)),
            Err(error) => {
                ::syslog::error!("{:?}", error);
                None
            },
        },
        None => None,
    };

    let mut engine: WasmEngine = WasmEngine::new(&wasm_binary, 42, &sockaddr);

    engine.run();

    ::syslog::info!("shutting down wasm daemon...");

    Ok(())
}
