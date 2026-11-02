// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    let (control_tx, control_rx): (Sender<MemoryControlCommand>, Receiver<MemoryControlCommand>) =
        ::tokio::sync::mpsc::channel::<MemoryControlCommand>(2);
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
    assert_eq!(received_msg.to_bytes(), original.clone().to_bytes(), "message payload mismatch");
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
    let (control_tx, control_rx): (Sender<MemoryControlCommand>, Receiver<MemoryControlCommand>) =
        ::tokio::sync::mpsc::channel::<MemoryControlCommand>(1);
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
    let (_control_tx, control_rx): (Sender<MemoryControlCommand>, Receiver<MemoryControlCommand>) =
        ::tokio::sync::mpsc::channel::<MemoryControlCommand>(1); // No shutdown sent.
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

    assert_eq!(credits.load(Ordering::SeqCst), 0, "credit count should remain zero on error path");
}
