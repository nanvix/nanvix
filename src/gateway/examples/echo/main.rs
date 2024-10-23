// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Description
//!
//! This program demonstrates how to use the `gateway` module to create a simple echo server.
//!

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod config;
mod logging;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
#[macro_use]
extern crate log;

use crate::args::Args;
use ::anyhow::Result;
use ::gateway::{
    Gateway,
    HttpGatewayClient,
};
use ::std::{
    env,
    mem,
    net::SocketAddr,
    thread::{
        self,
        JoinHandle,
    },
};
use ::sys::ipc::Message;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() -> Result<()> {
    // Initialize logger before doing anything else. If this fails, the program will panic.
    logging::initialize();

    // Parse command line arguments.
    let mut args: Args = Args::parse(env::args().collect())?;

    // Parse HTTP socket address.
    let http_addr: SocketAddr = args.http_addr().parse()?;

    // Create gateway.
    let (mut gateway, tx, mut rx) = Gateway::<HttpGatewayClient>::new(http_addr, None)?;

    // Spawn a thread to run the gateway and handle incoming messages.
    let _gateway_thread: JoinHandle<()> = thread::spawn(move || {
        if let Err(e) = gateway.run() {
            error!("gateway thread failed: {:?}", e);
        }
    });

    // Process incoming messages.
    loop {
        // Attempt to receive a message from the gateway.
        let mut message: Message = match rx.try_recv() {
            Ok(msg) => {
                info!("received message from stdin: {:?}", msg);

                msg
            },
            // No message available.
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => continue,
            // Channel has disconnected.
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                info!("stdin channel has disconnected");
                break;
            },
        };

        // Swap the source and destination of the message.
        mem::swap(&mut message.destination, &mut message.source);

        // Send the message back to the gateway.
        if let Err(e) = tx.send(message) {
            error!("failed to send message (error={:?})", e);
        }
    }

    Ok(())
}
