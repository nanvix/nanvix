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
use ::serde_json::Value;
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
use ::wasmd::{
    WasmdMessage,
    WasmdMessageHeader,
};
use std::{
    collections::LinkedList,
    io::BufRead,
};
use wasmd::LoadMessage;

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

    let chunks: Vec<Vec<u8>> = wasm_file
        .bytes()
        .filter_map(Result::ok)
        .collect::<Vec<u8>>()
        .chunks(LoadMessage::PAYLOAD_SIZE)
        .map(|chunk| {
            let mut payload = Vec::<u8>::with_capacity(LoadMessage::PAYLOAD_SIZE);
            payload.extend_from_slice(chunk);
            payload
        })
        .collect();

    trace!("sending {} chunks, bytes={:?}", chunks.len(), chunks.len() * LoadMessage::PAYLOAD_SIZE);
    let mut messages = LinkedList::new();
    for chunk in chunks {
        let message: LoadMessage = LoadMessage {
            len: chunk.len() as u32,
            payload: {
                let mut payload = [0; LoadMessage::PAYLOAD_SIZE];
                payload[..chunk.len()].copy_from_slice(&chunk);
                payload
            },
        };
        let message: WasmdMessage = WasmdMessage::new(WasmdMessageHeader::Wasm, message.to_bytes());
        // let message: Message = Message::new(
        //     ProcessIdentifier::from(0),
        //     ProcessIdentifier::from(1),
        //     MessageType::Ikc,
        //     message.to_bytes(),
        // );
        messages.push_back(message);
    }

    // Create an empty message to signal the end of the transmission.
    let message: LoadMessage = LoadMessage {
        len: 0,
        payload: [0; LoadMessage::PAYLOAD_SIZE],
    };
    let message: WasmdMessage = WasmdMessage::new(WasmdMessageHeader::Wasm, message.to_bytes());
    messages.push_back(message);
    trace!("number of messages: {}", messages.len());

    // Convert messages to a vector of bytes.
    let mut payload: Vec<u8> = Vec::new();
    while let Some(msg) = messages.pop_front() {
        payload.extend(msg.to_bytes());
    }

    // Create a JSON object with fields "source", "destination", and "input"
    let source: i32 = 0;
    let destination: i32 = 1;
    let json_obj: Value = serde_json::json!({
        "source": source,
        "destination": destination,
        "payload": payload,
    });

    // Encapsulate JSON into a HTTP POST request
    let http_request = format!(
        "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        json_obj.to_string().len(),
        json_obj
    );

    trace!("sending HTTP request");
    conn.write_all(http_request.as_bytes())?;

    let mut buf_reader = std::io::BufReader::new(conn);
    for line in buf_reader.by_ref().lines() {
        let line = line?;
        if line.is_empty() {
            break;
        }
    }
    // Print the response from the server.
    for line in buf_reader.by_ref().lines() {
        let line = line?;
        println!("{}", line);
    }

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
