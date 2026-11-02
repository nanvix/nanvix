// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    IKC_SINK_BUDGET_BYTES,
    IkcStdoutSink,
};
use ::std::{
    sync::{
        Arc,
        mpsc as std_mpsc,
    },
    thread,
    time::Duration,
};
use ::sys::{
    ipc::{
        DataChunk,
        DataChunkHeader,
        IkcFrame,
        Message,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::tokio::sync::mpsc;

/// Builds an IKC message frame tagged with a sequence number in its payload prefix.
fn message_frame(seq: u32) -> IkcFrame {
    let mut message: Message = Message::default();
    message.payload[..4].copy_from_slice(&seq.to_le_bytes());
    IkcFrame::Message(message)
}

/// Extracts the sequence number from a frame produced by [`message_frame`].
fn seq_of(frame: &IkcFrame) -> Option<u32> {
    match frame {
        IkcFrame::Message(message) => {
            let bytes: [u8; 4] = message.payload[..4].try_into().expect("4 payload bytes");
            Some(u32::from_le_bytes(bytes))
        },
        IkcFrame::Bulk(_) => None,
    }
}

/// Builds a data chunk frame with `bytes` payload bytes.
fn bulk_frame(bytes: usize) -> IkcFrame {
    let data_len: u32 = u32::try_from(bytes).expect("payload length fits in u32");
    let header: DataChunkHeader = DataChunkHeader::new(
        ProcessIdentifier::from(0),
        ThreadIdentifier::from(0),
        ProcessIdentifier::from(0),
        ThreadIdentifier::from(0),
        0,
        data_len,
    );
    IkcFrame::Bulk(DataChunk::new(header, vec![0u8; bytes]))
}

/// Reproduces the issue #2603 mechanism at the channel level: a direct `blocking_send()` on a
/// full bounded channel blocks the caller indefinitely while the consumer is starved. In the
/// buggy code this call ran on the vCPU thread while it held the WHP partition lock, freezing
/// the guest. [`IkcStdoutSink`] (exercised by the next test) breaks this coupling.
#[test]
fn direct_blocking_send_blocks_when_full_reproducing_the_bug() {
    let (tx, rx) = mpsc::channel::<IkcFrame>(1);
    tx.blocking_send(message_frame(0))
        .expect("first send fits in the channel");

    let (done_tx, done_rx) = std_mpsc::channel::<()>();
    let sender = thread::spawn(move || {
        // No consumer drains `rx`, so this second send blocks until the channel is closed.
        let _ = tx.blocking_send(message_frame(1));
        let _ = done_tx.send(());
    });

    assert!(
        done_rx.recv_timeout(Duration::from_millis(500)).is_err(),
        "blocking_send unexpectedly returned on a full channel; the bug repro is invalid",
    );

    // Closing the channel releases the blocked sender so the helper thread can exit.
    drop(rx);
    sender.join().expect("join sender thread");
}

/// The sink's producer-side `send()` must never block, even when the downstream channel is full
/// and nothing is consuming it. This is the behavior that fixes issue #2603.
#[test]
fn producer_never_blocks_when_downstream_full_and_consumer_starved() {
    const FRAMES: u32 = 1000;
    let total: usize = usize::try_from(FRAMES).expect("frame count fits in usize");

    // Tiny downstream capacity with no initial consumer: the drain thread blocks after filling
    // it, modeling a starved consumer under heavy host load.
    let (tx, mut rx) = mpsc::channel::<IkcFrame>(2);
    let sink: Arc<IkcStdoutSink> = Arc::new(IkcStdoutSink::new(tx));

    let producer_sink: Arc<IkcStdoutSink> = sink.clone();
    let (done_tx, done_rx) = std_mpsc::channel::<()>();
    let producer = thread::spawn(move || {
        for seq in 0..FRAMES {
            producer_sink
                .send(message_frame(seq))
                .expect("send must not fail while the consumer is alive");
        }
        let _ = done_tx.send(());
    });

    // If `send()` blocked under back-pressure (the bug), this would time out.
    done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("producer blocked under back-pressure (regression of issue #2603)");
    producer.join().expect("join producer thread");

    // Draining the downstream now must yield every frame, in order: the hand-off is lossless.
    let mut received: Vec<u32> = Vec::with_capacity(total);
    while received.len() < total {
        let frame: IkcFrame = rx
            .blocking_recv()
            .expect("sink dropped before delivering all frames");
        if let Some(seq) = seq_of(&frame) {
            received.push(seq);
        }
    }
    let expected: Vec<u32> = (0..FRAMES).collect();
    assert_eq!(received, expected, "frames must be delivered losslessly and in order");

    drop(sink);
}

/// Frames still buffered when the sink is dropped must be flushed to the downstream before the
/// channel closes, so application output emitted just before shutdown is not lost.
#[test]
fn drop_flushes_all_buffered_frames_in_order() {
    const FRAMES: u32 = 256;
    let (tx, mut rx) = mpsc::channel::<IkcFrame>(4);
    let sink: IkcStdoutSink = IkcStdoutSink::new(tx);
    for seq in 0..FRAMES {
        sink.send(message_frame(seq)).expect("send");
    }

    // Collect on a separate thread so the drain thread can make progress (the small channel
    // blocks it) while the sink is being dropped and flushed.
    let collector = thread::spawn(move || {
        let mut received: Vec<u32> = Vec::new();
        while let Some(frame) = rx.blocking_recv() {
            if let Some(seq) = seq_of(&frame) {
                received.push(seq);
            }
        }
        received
    });

    // Dropping the sink flushes the backlog and then drops the downstream sender, which ends
    // the collector's stream.
    drop(sink);

    let received: Vec<u32> = collector.join().expect("join collector thread");
    let expected: Vec<u32> = (0..FRAMES).collect();
    assert_eq!(received, expected, "drop must flush all buffered frames in order");
}

/// The sink's producer-side queue is budgeted: if the downstream channel is full and the
/// consumer remains starved, `send()` must fail cleanly instead of buffering guest bulk writes
/// until the host process runs out of memory.
#[test]
fn producer_fails_when_in_process_budget_is_exhausted() {
    let (tx, rx) = mpsc::channel::<IkcFrame>(1);
    tx.blocking_send(message_frame(0))
        .expect("pre-fill downstream channel");
    let sink: IkcStdoutSink = IkcStdoutSink::new(tx);

    sink.send(bulk_frame(IKC_SINK_BUDGET_BYTES - DataChunkHeader::SIZE))
        .expect("first frame exactly consumes the budget");

    let result = sink.send(message_frame(1));
    assert!(result.is_err(), "send past the budget must fail");

    drop(rx);
    drop(sink);
}
