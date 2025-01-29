// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::Gateway;
use ::anyhow::Result;
use ::std::{
    io::ErrorKind,
    sync::mpsc::{
        Receiver,
        Sender,
        TryRecvError,
    },
    thread::{
        self,
        JoinHandle,
    },
};
use ::sys::ipc::Message;

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// Private data of the I/O thread.
///
pub struct IoThread {
    /// Connection to the gateway.
    gateway: Gateway,
    /// Gateway receiver.
    gateway_rx: Receiver<Message>,
    /// Gateway sender.
    gateway_tx: Sender<Message>,
}
//==================================================================================================
// Implementations
//==================================================================================================

impl IoThread {
    ///
    /// # Description
    ///
    /// Spawns a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `gateway`: Connection to gateway.
    /// - `gateway_rx`:   Gateway receiver.
    /// - `gateway_tx`:   Gateway sender.
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    pub fn spawn(
        gateway: Gateway,
        gateway_rx: Receiver<Message>,
        gateway_tx: Sender<Message>,
    ) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            let mut io_thread: IoThread = IoThread::new(gateway, gateway_rx, gateway_tx)?;
            io_thread.run()?;
            Ok(())
        })
    }

    ///
    /// # Description
    ///
    /// Creates a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `gateway`: Connection to gateway.
    /// - `gateway_rx`:   Gateway receiver.
    /// - `gateway_tx`:   Gateway sender.
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    fn new(
        gateway: Gateway,
        gateway_rx: Receiver<Message>,
        gateway_tx: Sender<Message>,
    ) -> Result<Self> {
        Ok(Self {
            gateway,
            gateway_rx,
            gateway_tx,
        })
    }

    ///
    /// # Description
    ///
    /// Runs the I/O thread.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn run(&mut self) -> Result<()> {
        loop {
            self.send()?;
            self.receive()?;
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to send pending messages to the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Errors
    ///
    /// If the message could not be sent, an error is returned.
    ///
    fn send(&mut self) -> Result<()> {
        match self.gateway_rx.try_recv() {
            Ok(msg) => {
                self.gateway.send(msg)?;
            },
            Err(TryRecvError::Empty) => {
                // No message available.
            },
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the microvm has disconnected".to_string();
                error!("send(): {}", reason);
                anyhow::bail!(reason);
            },
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to receive messages from the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn receive(&mut self) -> Result<()> {
        match self.gateway.receive() {
            Ok(message) => {
                if let Err(e) = self.gateway_tx.send(message) {
                    let reason: String =
                        format!("failed to receive message to the microvm (error={:?})", e);
                    error!("receive(): {}", reason);
                    anyhow::bail!(reason);
                }
            },
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    return Ok(());
                }

                let reason: String =
                    format!("failed to receive message from the gateway (error={:?})", e);
                error!("receive(): {}", reason);
                anyhow::bail!(reason);
            },
        }
        Ok(())
    }
}
