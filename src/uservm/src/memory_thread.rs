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
    use ::sys::ipc::Message;
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

    /// Spawns a memory thread for tests with an injected credit function.
    fn spawn(
        data_rx: Receiver<IkcFrame>,
        data_tx: Sender<IkcFrame>,
        control_rx: Receiver<MemoryControlCommand>,
        control_tx: Sender<MemoryControlResponse>,
        mut add_credit: impl FnMut() -> Result<()> + Send + 'static,
    ) -> (JoinHandle<()>, MessageCounters) {
        // Wrap the synchronous test closure into the asynchronous AddCreditFn expected by
        // `MemoryThread::new` in test builds.
        let add_credit_box: Box<AddCreditFn> = Box::new(move || {
            let res: Result<()> = add_credit();
            Box::pin(async move { res })
        });
        let counters: MessageCounters = MessageCounters::new();
        let handle: JoinHandle<()> = MemoryThread::new(
            data_rx,
            data_tx,
            control_rx,
            control_tx,
            add_credit_box,
            counters.clone(),
        )
        .spawn();
        (handle, counters)
    }

    // Helper: small timeout to avoid hanging tests.
    fn short_timeout() -> Duration {
        Duration::from_millis(250)
    }

    #[tokio::test]
    async fn test_message_forward_and_credit_increment() {
        let (io_tx, io_rx): (Sender<IkcFrame>, Receiver<IkcFrame>) =
            ::tokio::sync::mpsc::channel::<IkcFrame>(4);
        let (vm_tx, mut vm_rx): (Sender<IkcFrame>, Receiver<IkcFrame>) =
            ::tokio::sync::mpsc::channel::<IkcFrame>(4);
        let (control_tx, control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = ::tokio::sync::mpsc::channel::<MemoryControlCommand>(2);
        let (resp_tx, _resp_rx): (Sender<MemoryControlResponse>, Receiver<MemoryControlResponse>) =
            ::tokio::sync::mpsc::channel::<MemoryControlResponse>(2);

        let credits: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let credits_clone: Arc<AtomicUsize> = credits.clone();

        let (handle, counters): (JoinHandle<()>, MessageCounters) =
            spawn(io_rx, vm_tx, control_rx, resp_tx, move || {
                credits_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });

        // Send a message that should be forwarded.
        let original: Message = Message::default();
        // Pre-increment the I/O counter to satisfy the debug assertion in
        // on_message_received_from_io_handler (mem_received <= io_sent).
        counters.increment_io_handler_messages_sent();
        io_tx
            .send(IkcFrame::Message(original.clone()))
            .await
            .expect("send to memory thread");

        let received: IkcFrame = timeout(short_timeout(), vm_rx.recv())
            .await
            .expect("forward receive timeout")
            .expect("forward channel closed unexpectedly");

        let received_msg: Message = match received {
            IkcFrame::Message(m) => m,
            _ => panic!("expected IkcFrame::Message"),
        };
        assert_eq!(
            received_msg.to_bytes(),
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
        let (_io_tx, io_rx): (Sender<IkcFrame>, Receiver<IkcFrame>) =
            ::tokio::sync::mpsc::channel::<IkcFrame>(1);
        let (vm_tx, mut _vm_rx): (Sender<IkcFrame>, Receiver<IkcFrame>) =
            ::tokio::sync::mpsc::channel::<IkcFrame>(1);
        let (control_tx, control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = ::tokio::sync::mpsc::channel::<MemoryControlCommand>(1);
        let (resp_tx, _resp_rx): (Sender<MemoryControlResponse>, Receiver<MemoryControlResponse>) =
            ::tokio::sync::mpsc::channel::<MemoryControlResponse>(1);

        let credits: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let credits_clone: Arc<AtomicUsize> = credits.clone();
        let (handle, _counters): (JoinHandle<()>, MessageCounters) =
            spawn(io_rx, vm_tx, control_rx, resp_tx, move || {
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
        let (io_tx, io_rx): (Sender<IkcFrame>, Receiver<IkcFrame>) =
            ::tokio::sync::mpsc::channel::<IkcFrame>(2);
        let (vm_tx, mut vm_rx): (Sender<IkcFrame>, Receiver<IkcFrame>) =
            ::tokio::sync::mpsc::channel::<IkcFrame>(2);
        let (_control_tx, control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = ::tokio::sync::mpsc::channel::<MemoryControlCommand>(1); // No shutdown sent.
        let (resp_tx, _resp_rx): (Sender<MemoryControlResponse>, Receiver<MemoryControlResponse>) =
            ::tokio::sync::mpsc::channel::<MemoryControlResponse>(1);

        let credits: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let (handle, counters): (JoinHandle<()>, MessageCounters) =
            spawn(io_rx, vm_tx, control_rx, resp_tx, move || {
                // Simulate a failure on first credit attempt.
                Err(anyhow!("simulated credit failure"))
            });

        let msg: Message = Message::default();
        // Pre-increment the I/O counter to satisfy the debug assertion in
        // on_message_received_from_io_handler (mem_received <= io_sent).
        counters.increment_io_handler_messages_sent();
        io_tx
            .send(IkcFrame::Message(msg.clone()))
            .await
            .expect("send first message");

        // Forwarded even though credit fails afterwards.
        let forwarded: IkcFrame = timeout(short_timeout(), vm_rx.recv())
            .await
            .expect("forward receive timeout")
            .expect("forward channel closed");
        let forwarded_msg: Message = match forwarded {
            IkcFrame::Message(m) => m,
            _ => panic!("expected IkcFrame::Message"),
        };
        assert_eq!(
            forwarded_msg.to_bytes(),
            msg.clone().to_bytes(),
            "message not forwarded prior to error"
        );

        // Thread should terminate due to credit error.
        let join_result: ::core::result::Result<(), ::tokio::task::JoinError> =
            timeout(short_timeout(), handle)
                .await
                .expect("memory thread did not terminate after credit error");
        assert!(join_result.is_ok(), "memory thread did not exit cleanly after credit error");

        assert_eq!(
            credits.load(Ordering::SeqCst),
            0,
            "credit count should remain zero on error path"
        );
    }
}
