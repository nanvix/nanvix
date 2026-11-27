// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::tokio::{
    task::JoinHandle,
    time::{
        Duration,
        timeout,
    },
};

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn stalled_writer_does_not_stall_guest_and_eof_preserves_order() {
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1);
    let (writer_tx, mut writer_rx) = mpsc::channel::<Vec<u8>>(1);
    let forwarder: JoinHandle<io::Result<()>> =
        tokio::spawn(async move { buffer_output(&mut rx, writer_tx, false).await });
    let expected: Vec<u8> = (0..4096u32)
        .flat_map(|seq| seq.to_le_bytes().repeat(64))
        .collect();

    timeout(TEST_TIMEOUT, async {
        for bytes in expected.chunks(256) {
            tx.send(bytes.to_vec()).await.expect("guest send");
        }
    })
    .await
    .expect("guest must finish while the host writer remains stalled");
    drop(tx);

    let actual: Vec<u8> = timeout(TEST_TIMEOUT, async {
        let mut actual: Vec<u8> = Vec::new();
        while let Some(batch) = writer_rx.recv().await {
            actual.extend(batch);
        }
        actual
    })
    .await
    .expect("buffer must drain before EOF");
    forwarder
        .await
        .expect("forwarder task")
        .expect("lossless forwarding");
    assert_eq!(actual, expected, "all accepted bytes must arrive in order before EOF");
}

#[tokio::test]
async fn exhausted_budget_is_reported_without_blocking() {
    const BUDGET: usize = 16;
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(2);
    tx.try_send(vec![1; BUDGET]).expect("first chunk");
    tx.try_send(vec![2]).expect("excess chunk");
    let (writer_tx, _writer_rx) = mpsc::channel::<Vec<u8>>(1);
    writer_tx.try_send(vec![0]).expect("stall writer");

    let error: io::Error =
        timeout(TEST_TIMEOUT, buffer_output_with_budget(&mut rx, writer_tx, BUDGET, false))
            .await
            .expect("budget error must not block")
            .expect_err("one byte beyond the budget must fail");
    assert_eq!(error.kind(), io::ErrorKind::OutOfMemory);
}

#[tokio::test]
async fn closed_writer_is_reported() {
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1);
    tx.try_send(vec![1]).expect("guest data");
    drop(tx);
    let (writer_tx, writer_rx) = mpsc::channel::<Vec<u8>>(1);
    drop(writer_rx);

    let error: io::Error = buffer_output(&mut rx, writer_tx, false)
        .await
        .expect_err("writer closure must fail");
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}
