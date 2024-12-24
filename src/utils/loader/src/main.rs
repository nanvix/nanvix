// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
#[macro_use]
extern crate log;

use crate::args::Args;
use ::anyhow::Result;
use ::flexi_logger::Logger;
use ::std::{
    env,
    fs::File,
    io::{
        BufReader,
        Read,
        Write,
    },
    net::TcpStream,
    sync::Once,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() -> Result<()> {
    initialize();

    let args: Args = args::Args::parse(env::args().collect())?;

    // Attempt to open the Wasm file.
    let wasm_filename: String = args.wasm_filename().to_string();
    info!("opening wasm file {wasm_filename}");
    let wasm_file: File = File::open(wasm_filename)?;
    let wasm_file: BufReader<File> = BufReader::new(wasm_file);

    // Attempt to connect to server.
    let sockaddr: String = args.sockaddr().to_string();
    info!("connecting to server at {sockaddr}");
    let mut conn: TcpStream = TcpStream::connect(sockaddr)?;

    // Read WASM file to a vector
    let wasm_file: Vec<u8> = wasm_file.bytes().filter_map(Result::ok).collect();
    let length: u32 = wasm_file.len() as u32;

    // Send the WASM file to the server.
    info!("sending WASM file to server");
    conn.write_all(&length.to_le_bytes())?;
    conn.write_all(&wasm_file)?;

    Ok(())
}

///
/// # Description
///
/// Initializes the logger.
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
pub fn initialize() {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        Logger::try_with_env()
            .expect("malformed RUST_LOG environment variable")
            .start()
            .expect("failed to initialize logger");
    });
}
