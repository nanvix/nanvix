// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn decomposed_ops_leave_out_write_operations_when_read_only() {
    let readonly: Vec<(&str, VfsOp, Option<VfsOp>)> = decomposed_ops(true);

    for (label, minuend, subtrahend) in &readonly {
        assert!(!is_write_op(*minuend), "{label} writes to a read-only mount");
        assert!(
            subtrahend.is_none_or(|op| !is_write_op(op)),
            "{label} writes to a read-only mount"
        );
    }

    let labels: Vec<&str> = readonly.iter().map(|(label, _, _)| *label).collect();
    assert_eq!(labels, ["protocol", "stat", "open+close", "readdir", "seq_read"]);
    assert_eq!(decomposed_ops(false).len(), DECOMPOSED_OPS.len());
}

#[test]
fn ramfs_load_us_reads_the_reported_timing() {
    let mut timings: PhaseTimings = PhaseTimings::new();
    assert_eq!(ramfs_load_us(None), None);
    assert_eq!(ramfs_load_us(Some(timings.clone())), None);

    timings.insert("ramfs_load_us".to_string(), serde_json::json!("fast"));
    assert_eq!(ramfs_load_us(Some(timings.clone())), None);

    timings.insert("ramfs_load_us".to_string(), serde_json::json!(1234));
    assert_eq!(ramfs_load_us(Some(timings)), Some(1234));
}

#[tokio::test]
async fn stderr_collector_returns_what_the_stream_carried() {
    let (mut writer, reader) = ::tokio::io::duplex(64);
    let collector: StderrCollector = StderrCollector::spawn(reader);
    writer
        .write_all(b"boom")
        .await
        .expect("failed to write to the stream");
    drop(writer);

    assert_eq!(collector.collect(Duration::from_secs(30)).await, "boom");
}

#[tokio::test]
async fn stderr_collector_keeps_what_it_read_when_the_stream_does_not_end() {
    let (mut writer, reader) = ::tokio::io::duplex(64);
    let collector: StderrCollector = StderrCollector::spawn(reader);
    writer
        .write_all(b"partial output")
        .await
        .expect("failed to write to the stream");

    // The writer stays open, so only the timeout ends the wait.
    let output: String = collector.collect(Duration::from_millis(50)).await;

    assert_eq!(output, "partial output");
    // The reader is gone only if the task was cancelled, not detached.
    assert!(writer.write_all(b"more").await.is_err());
}
