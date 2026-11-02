// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! This module implements the VMM "memory thread", a lightweight worker responsible for relaying
//! messages between the I/O subsystem and the guest, while participating in a simple credit-based
//! flow-control mechanism.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    counters::MessageCounters,
    orchestrator::{
        MemoryControlCommand,
        MemoryControlResponse,
    },
};
use ::anyhow::{
    Error,
    Result,
};
use ::log::{
    debug,
    error,
    trace,
};
use ::std::{
    marker::Send,
    pin::Pin,
};
use ::sys::ipc::IkcFrame;
use ::tokio::sync::mpsc::{
    Receiver,
    Sender,
};

//==================================================================================================
// Types
//==================================================================================================

/// Type alias for the async credit addition function used by the memory thread.
pub type AddCreditFn =
    dyn FnMut() -> Pin<Box<dyn ::std::future::Future<Output = Result<()>> + Send>> + Send + 'static;

//==================================================================================================
// Structures
//==================================================================================================

/// Tokio task that relays [`IkcFrame`] frames while coordinating credit management.
pub struct MemoryThread {
    data_rx: Receiver<IkcFrame>,
    data_tx: Sender<IkcFrame>,
    control_rx: Receiver<MemoryControlCommand>,
    control_tx: Sender<MemoryControlResponse>,
    add_credit: Box<AddCreditFn>,
    counters: MessageCounters,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl MemoryThread {
    ///
    /// # Description
    ///
    /// Creates a new [`MemoryThread`] instance with the provided communication channels.
    ///
    /// # Parameters
    ///
    /// - `data_rx`: Receives data messages from the I/O handler.
    /// - `data_tx`: Sends data messages to the virtual machine's stdin.
    /// - `control_rx`: Receives control commands from the VMM.
    /// - `control_tx`: Sends control responses to the VMM.
    /// - `add_credit`: Closure that adds a credit to the virtual machine credit pool.
    /// - `counters`: Shared counters for tracking message flow across threads.
    ///
    pub fn new(
        data_rx: Receiver<IkcFrame>,
        data_tx: Sender<IkcFrame>,
        control_rx: Receiver<MemoryControlCommand>,
        control_tx: Sender<MemoryControlResponse>,
        add_credit: Box<AddCreditFn>,
        counters: MessageCounters,
    ) -> Self {
        Self {
            data_rx,
            data_tx,
            control_rx,
            control_tx,
            add_credit,
            counters,
        }
    }

    ///
    /// # Description
    ///
    /// Spawns a new memory thread.
    ///
    /// This thread relays messages between the I/O subsystem and the guest,
    /// while participating in a simple credit-based flow-control mechanism.
    ///
    /// # Returns
    ///
    /// Returns a `::tokio::task::JoinHandle<()>` for the spawned memory thread.
    ///
    pub fn spawn(self) -> ::tokio::task::JoinHandle<()> {
        trace!("spawn()");
        ::tokio::spawn(async move {
            let mut data_rx: Receiver<IkcFrame> = self.data_rx;
            let data_tx: Sender<IkcFrame> = self.data_tx;
            let mut control_rx: Receiver<MemoryControlCommand> = self.control_rx;
            let _control_tx: Sender<MemoryControlResponse> = self.control_tx;
            let mut add_credit: Box<AddCreditFn> = self.add_credit;
            let counters: MessageCounters = self.counters;

            let result: Result<(), Error> = loop {
                ::tokio::select! {
                    command = control_rx.recv() => {
                        match command {
                            Some(MemoryControlCommand::Shutdown) => {
                                debug!("spawn(): received shutdown command");
                                break Ok(())
                            },
                            None => {
                                debug!("spawn(): VMM control channel has been disconnected");
                                break Ok(())
                            },
                        }
                    }
                    msg = data_rx.recv() => {
                        match msg {
                            Some(transfer) => {
                                on_message_received_from_io_handler(&counters);

                                if let Err(e) = data_tx.send(transfer).await {
                                    error!("spawn(): failed to send transfer: {e}");
                                    continue;
                                }
                                if let Err(error) = add_credit().await {
                                    error!("spawn(): failed to add credit: {error}");
                                    break Err(error);
                                }
                            },
                            None => {
                                debug!("spawn(): channel has been disconnected");
                                break Ok(());
                            },
                        }
                    }
                }
            };

            if let Err(error) = result {
                error!("spawn(): exited with error: {error}");
            } else {
                debug!("spawn(): exited normally");
            }
        })
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Handler to be called whenever a message is received from the I/O handler.
///
/// # Parameters
///
/// - `counters` - Shared counters for tracking message flow across threads.
///
fn on_message_received_from_io_handler(counters: &MessageCounters) {
    counters.increment_mem_thread_messages_received();

    // Sanity check that no messages are lost.
    #[cfg(debug_assertions)]
    {
        // The following check is not atomic, but since the two counters are monotonically
        // increasing AND they are strictly updated one after another, it should be sufficient to
        // detect message losses.

        let cached_mem_thread_num_messages_received: usize =
            counters.get_mem_thread_messages_received();

        let cached_io_handler_num_messages_sent: usize = counters.get_io_handler_messages_sent();

        debug_assert!(
            cached_mem_thread_num_messages_received <= cached_io_handler_num_messages_sent,
            "memory thread has received more messages than the I/O handler sent (
                                        {} > {})",
            cached_mem_thread_num_messages_received,
            cached_io_handler_num_messages_sent
        );
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests;
