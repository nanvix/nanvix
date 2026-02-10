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
use ::std::{
    marker::Send,
    pin::Pin,
};
use ::sys::ipc::Message;
use ::syslog::{
    debug,
    error,
    trace,
};
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

/// Type alias for the async message delivery function used by the memory thread.
///
/// In PMIO mode, this sends the message via a channel and adds a credit.
/// In ring buffer mode, this writes the message directly into the RX ring.
pub type DeliverMessageFn = dyn FnMut(Message) -> Pin<Box<dyn ::std::future::Future<Output = Result<()>> + Send>>
    + Send
    + 'static;

//==================================================================================================
// Structures
//==================================================================================================

/// Tokio task that relays `Message` frames while coordinating credit management.
pub struct MemoryThread {
    data_rx: Receiver<Message>,
    control_rx: Receiver<MemoryControlCommand>,
    control_tx: Sender<MemoryControlResponse>,
    deliver_message: Box<DeliverMessageFn>,
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
    /// - `data_rx`: Receives data messages from the I/O thread.
    /// - `control_rx`: Receives control commands from the VMM.
    /// - `control_tx`: Sends control responses to the VMM.
    /// - `deliver_message`: Closure that delivers a message to the guest. In PMIO mode, this sends
    ///   the message via a channel and adds a credit. In ring buffer mode, this writes the message
    ///   directly into the RX ring.
    /// - `counters`: Shared counters for tracking message flow across threads.
    ///
    pub fn new(
        data_rx: Receiver<Message>,
        control_rx: Receiver<MemoryControlCommand>,
        control_tx: Sender<MemoryControlResponse>,
        deliver_message: Box<DeliverMessageFn>,
        counters: MessageCounters,
    ) -> Self {
        Self {
            data_rx,
            control_rx,
            control_tx,
            deliver_message,
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
            let mut data_rx: Receiver<Message> = self.data_rx;
            let mut control_rx: Receiver<MemoryControlCommand> = self.control_rx;
            let _control_tx: Sender<MemoryControlResponse> = self.control_tx;
            let mut deliver_message: Box<DeliverMessageFn> = self.deliver_message;
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
                            Some(mut msg) => {
                                // Label: uservm::memory_thread::data_rx::recv()
                                profiler::timestamp_message!(&mut msg.payload,
                                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                        + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                                );

                                on_message_received_from_io_thread(&counters);

                                if let Err(error) = deliver_message(msg).await {
                                    error!("spawn(): failed to deliver message: {error}");
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
/// Handler to be called whenever a message is received from the I/O thread.
///
/// # Parameters
///
/// - `counters` - Shared counters for tracking message flow across threads.
///
fn on_message_received_from_io_thread(counters: &MessageCounters) {
    counters.increment_mem_thread_messages_received();

    // Sanity check that no messages are lost.
    #[cfg(debug_assertions)]
    {
        // The following check is not atomic, but since the two counters are monotonically
        // increasing AND they are strictly updated one after another, it should be sufficient to
        // detect message losses.

        let cached_mem_thread_num_messages_received: usize =
            counters.get_mem_thread_messages_received();

        let cached_io_thread_num_messages_received: usize =
            counters.get_io_thread_messages_received();

        debug_assert!(
            cached_mem_thread_num_messages_received <= cached_io_thread_num_messages_received,
            "memory thread has received more messages than the i/o thread (
                                        {} > {})",
            cached_mem_thread_num_messages_received,
            cached_io_thread_num_messages_received
        );
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::anyhow::anyhow;
    use ::std::sync::{
        Arc,
        atomic::{
            AtomicUsize,
            Ordering,
        },
    };
    use ::tokio::{
        task::JoinHandle,
        time::{
            Duration,
            timeout,
        },
    };

    //----------------------------------------------------------------------------------------------
    // Test Helpers
    //----------------------------------------------------------------------------------------------

    /// Spawns a memory thread for tests with an injected delivery function.
    fn spawn(
        data_rx: Receiver<Message>,
        control_rx: Receiver<MemoryControlCommand>,
        control_tx: Sender<MemoryControlResponse>,
        mut deliver: impl FnMut(Message) -> Result<()> + Send + 'static,
    ) -> JoinHandle<()> {
        // Wrap the synchronous test closure into the asynchronous DeliverMessageFn expected by
        // `MemoryThread::new` in test builds.
        let deliver_box: Box<DeliverMessageFn> = Box::new(move |msg| {
            let res: Result<()> = deliver(msg);
            Box::pin(async move { res })
        });
        let counters: MessageCounters = MessageCounters::new();
        MemoryThread::new(data_rx, control_rx, control_tx, deliver_box, counters).spawn()
    }

    // Helper: small timeout to avoid hanging tests.
    fn short_timeout() -> Duration {
        Duration::from_millis(250)
    }

    #[tokio::test]
    async fn test_message_forward_and_credit_increment() {
        let (io_tx, io_rx): (Sender<Message>, Receiver<Message>) =
            ::tokio::sync::mpsc::channel::<Message>(4);
        let (vm_tx, mut vm_rx): (Sender<Message>, Receiver<Message>) =
            ::tokio::sync::mpsc::channel::<Message>(4);
        let (control_tx, control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = ::tokio::sync::mpsc::channel::<MemoryControlCommand>(2);
        let (resp_tx, _resp_rx): (Sender<MemoryControlResponse>, Receiver<MemoryControlResponse>) =
            ::tokio::sync::mpsc::channel::<MemoryControlResponse>(2);

        let credits: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let credits_clone: Arc<AtomicUsize> = credits.clone();

        let handle: JoinHandle<()> = spawn(io_rx, control_rx, resp_tx, move |msg| {
            vm_tx.blocking_send(msg).map_err(|e| anyhow!("{e}"))?;
            credits_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        // Send a message that should be forwarded.
        let original: Message = Message::default();
        io_tx
            .send(original.clone())
            .await
            .expect("send to memory thread");

        let received: Message = timeout(short_timeout(), vm_rx.recv())
            .await
            .expect("forward receive timeout")
            .expect("forward channel closed unexpectedly");

        assert_eq!(
            received.clone().to_bytes(),
            original.clone().to_bytes(),
            "message payload mismatch"
        );
        assert_eq!(credits.load(Ordering::SeqCst), 1, "credit not incremented after message");

        // Ask the thread to shutdown cleanly.
        control_tx
            .send(MemoryControlCommand::Shutdown)
            .await
            .expect("send shutdown");

        // Ensure the task terminates.
        let join_result: ::core::result::Result<(), ::tokio::task::JoinError> =
            timeout(short_timeout(), handle)
                .await
                .expect("memory thread did not shutdown in time");
        assert!(join_result.is_ok(), "memory thread did not exit cleanly");
    }

    #[tokio::test]
    async fn test_shutdown_without_messages() {
        let (_io_tx, io_rx): (Sender<Message>, Receiver<Message>) =
            ::tokio::sync::mpsc::channel::<Message>(1);
        let (control_tx, control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = ::tokio::sync::mpsc::channel::<MemoryControlCommand>(1);
        let (resp_tx, _resp_rx): (Sender<MemoryControlResponse>, Receiver<MemoryControlResponse>) =
            ::tokio::sync::mpsc::channel::<MemoryControlResponse>(1);

        let credits: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let credits_clone: Arc<AtomicUsize> = credits.clone();
        let handle: JoinHandle<()> = spawn(io_rx, control_rx, resp_tx, move |_msg| {
            credits_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        // Immediate shutdown.
        control_tx
            .send(MemoryControlCommand::Shutdown)
            .await
            .expect("send shutdown");

        let join_result: ::core::result::Result<(), ::tokio::task::JoinError> =
            timeout(short_timeout(), handle)
                .await
                .expect("memory thread did not shutdown promptly");
        assert_eq!(credits.load(Ordering::SeqCst), 0, "no credits should have been added");

        assert!(join_result.is_ok(), "memory thread did not exit cleanly on shutdown");
    }

    #[tokio::test]
    async fn test_credit_error_terminates_thread() {
        let (io_tx, io_rx): (Sender<Message>, Receiver<Message>) =
            ::tokio::sync::mpsc::channel::<Message>(2);
        let (_control_tx, control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = ::tokio::sync::mpsc::channel::<MemoryControlCommand>(1); // No shutdown sent.
        let (resp_tx, _resp_rx): (Sender<MemoryControlResponse>, Receiver<MemoryControlResponse>) =
            ::tokio::sync::mpsc::channel::<MemoryControlResponse>(1);

        let delivered: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let handle: JoinHandle<()> = spawn(io_rx, control_rx, resp_tx, move |_msg| {
            // Simulate a failure on first delivery attempt.
            Err(anyhow!("simulated delivery failure"))
        });

        let msg: Message = Message::default();
        io_tx.send(msg.clone()).await.expect("send first message");

        // Thread should terminate due to delivery error.
        let join_result: ::core::result::Result<(), ::tokio::task::JoinError> =
            timeout(short_timeout(), handle)
                .await
                .expect("memory thread did not terminate after delivery error");
        assert!(join_result.is_ok(), "memory thread did not exit cleanly after delivery error");

        assert_eq!(
            delivered.load(Ordering::SeqCst),
            0,
            "delivered count should remain zero on error path"
        );
    }
}
