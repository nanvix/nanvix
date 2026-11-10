// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::{
    env,
    future,
    process,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};
use ::tokio::{
    runtime::{
        Builder,
        Runtime,
    },
    time::sleep,
};

/// Delay between checks for the snapshot artifacts.
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Limit that is not expected to expire.
const TEST_LONG_TIMEOUT: Duration = Duration::from_secs(30);
/// Limit that is expected to expire.
const TEST_SHORT_TIMEOUT: Duration = Duration::from_millis(100);
/// Limit for nanvixd to exit after a slow snapshot save.
const TEST_EXIT_TIMEOUT: Duration = Duration::from_millis(500);
/// Time that a slow snapshot save takes, which is longer than `TEST_EXIT_TIMEOUT`.
const TEST_SLOW_SAVE_DELAY: Duration = Duration::from_millis(1000);
/// Time that nanvixd takes to exit after a slow snapshot save.
const TEST_EXIT_DELAY: Duration = Duration::from_millis(50);
/// Size of the fake memory snapshot.
const TEST_VMEM_SIZE: usize = 16;

/// Runs `future` to completion on a fresh Tokio runtime.
fn block_on<F: Future>(future: F) -> F::Output {
    let runtime: Runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    runtime.block_on(future)
}

/// Creates an empty snapshots directory that is unique to the calling test.
fn unique_snapshots_dir(prefix: &str) -> PathBuf {
    let nanos: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is set before the UNIX epoch")
        .as_nanos();
    let snapshots_dir: PathBuf =
        env::temp_dir().join(format!("{prefix}-{pid}-{nanos}", pid = process::id()));
    fs::create_dir_all(&snapshots_dir).expect("failed to create snapshots directory");
    snapshots_dir
}

/// Writes fake snapshot artifacts, as the VMM does once it saves a snapshot.
fn write_snapshot_artifacts(snapshots_dir: &Path) {
    fs::write(snapshots_dir.join(SNAPSHOT_VMEM_FILENAME), [0u8; TEST_VMEM_SIZE])
        .expect("failed to write memory snapshot");
    fs::write(snapshots_dir.join(SNAPSHOT_KVM_JSON_FILENAME), b"{}")
        .expect("failed to write KVM snapshot");
}

#[test]
fn wait_save_and_exit_reports_exit_after_save() {
    let snapshots_dir: PathBuf = unique_snapshots_dir("nvx-save-exit-saved");
    write_snapshot_artifacts(&snapshots_dir);

    let outcome: SaveExitOutcome = block_on(wait_save_and_exit(
        async { Ok(0) },
        &snapshots_dir,
        TEST_POLL_INTERVAL,
        TEST_LONG_TIMEOUT,
        TEST_LONG_TIMEOUT,
    ));

    let _ = fs::remove_dir_all(&snapshots_dir);
    assert!(matches!(outcome, SaveExitOutcome::Exited(Ok(0))), "unexpected outcome: {outcome:?}");
}

#[test]
fn wait_save_and_exit_reports_exit_before_save() {
    let snapshots_dir: PathBuf = unique_snapshots_dir("nvx-save-exit-early");

    let outcome: SaveExitOutcome = block_on(wait_save_and_exit(
        async { Ok(1) },
        &snapshots_dir,
        TEST_POLL_INTERVAL,
        TEST_LONG_TIMEOUT,
        TEST_LONG_TIMEOUT,
    ));

    let _ = fs::remove_dir_all(&snapshots_dir);
    assert!(matches!(outcome, SaveExitOutcome::Exited(Ok(1))), "unexpected outcome: {outcome:?}");
}

#[test]
fn wait_save_and_exit_times_out_on_unfinished_save() {
    let snapshots_dir: PathBuf = unique_snapshots_dir("nvx-save-exit-unsaved");
    // The VMM writes the KVM snapshot only once the memory snapshot is flushed to disk.
    fs::write(snapshots_dir.join(SNAPSHOT_VMEM_FILENAME), [0u8; TEST_VMEM_SIZE])
        .expect("failed to write memory snapshot");

    let outcome: SaveExitOutcome = block_on(wait_save_and_exit(
        future::pending::<Result<i32>>(),
        &snapshots_dir,
        TEST_POLL_INTERVAL,
        TEST_SHORT_TIMEOUT,
        TEST_LONG_TIMEOUT,
    ));

    let _ = fs::remove_dir_all(&snapshots_dir);
    let expected_artifacts: String = format!(
        "{SNAPSHOT_VMEM_FILENAME}={TEST_VMEM_SIZE} bytes, {SNAPSHOT_KVM_JSON_FILENAME}=missing"
    );
    let timed_out_with_expected_artifacts: bool = matches!(
        &outcome,
        SaveExitOutcome::SaveTimedOut(artifacts) if *artifacts == expected_artifacts
    );
    assert!(timed_out_with_expected_artifacts, "unexpected outcome: {outcome:?}");
}

#[test]
fn wait_save_and_exit_times_out_on_exit_after_save() {
    let snapshots_dir: PathBuf = unique_snapshots_dir("nvx-save-exit-hung");
    write_snapshot_artifacts(&snapshots_dir);

    let outcome: SaveExitOutcome = block_on(wait_save_and_exit(
        future::pending::<Result<i32>>(),
        &snapshots_dir,
        TEST_POLL_INTERVAL,
        TEST_LONG_TIMEOUT,
        TEST_SHORT_TIMEOUT,
    ));

    let _ = fs::remove_dir_all(&snapshots_dir);
    assert!(matches!(outcome, SaveExitOutcome::ExitTimedOut), "unexpected outcome: {outcome:?}");
}

#[test]
fn wait_save_and_exit_excludes_save_time_from_exit_timeout() {
    let snapshots_dir: PathBuf = unique_snapshots_dir("nvx-save-exit-slow");
    let writer_dir: PathBuf = snapshots_dir.clone();

    let outcome: SaveExitOutcome = block_on(async {
        let writer: JoinHandle<()> = ::tokio::spawn(async move {
            sleep(TEST_SLOW_SAVE_DELAY).await;
            write_snapshot_artifacts(&writer_dir);
        });
        let outcome: SaveExitOutcome = wait_save_and_exit(
            async {
                sleep(TEST_SLOW_SAVE_DELAY + TEST_EXIT_DELAY).await;
                Ok(0)
            },
            &snapshots_dir,
            TEST_POLL_INTERVAL,
            TEST_LONG_TIMEOUT,
            TEST_EXIT_TIMEOUT,
        )
        .await;
        writer.await.expect("snapshot writer task failed");
        outcome
    });

    let _ = fs::remove_dir_all(&snapshots_dir);
    assert!(matches!(outcome, SaveExitOutcome::Exited(Ok(0))), "unexpected outcome: {outcome:?}");
}

#[test]
fn describe_snapshot_artifacts_reports_sizes_and_missing_artifacts() {
    let snapshots_dir: PathBuf = unique_snapshots_dir("nvx-save-exit-describe");
    let missing: String = describe_snapshot_artifacts(&snapshots_dir);
    write_snapshot_artifacts(&snapshots_dir);
    let present: String = describe_snapshot_artifacts(&snapshots_dir);

    let _ = fs::remove_dir_all(&snapshots_dir);
    assert_eq!(
        missing,
        format!("{SNAPSHOT_VMEM_FILENAME}=missing, {SNAPSHOT_KVM_JSON_FILENAME}=missing")
    );
    assert_eq!(
        present,
        format!(
            "{SNAPSHOT_VMEM_FILENAME}={TEST_VMEM_SIZE} bytes, {SNAPSHOT_KVM_JSON_FILENAME}=2 bytes"
        )
    );
}
