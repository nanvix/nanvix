// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::path::PathBuf;
use ::tempfile::TempDir;

#[tokio::test]
async fn ensure_kernel_binary_available_accepts_existing_kernel() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let kernel_path: PathBuf = tmp_dir.path().join(KERNEL_BINARY_NAME);
    ::std::fs::write(&kernel_path, b"fake kernel").expect("failed to write kernel binary");
    let args: Args = Args::parse(vec![
        "nanvixd".to_string(),
        "-bin-dir".to_string(),
        tmp_dir.path().to_string_lossy().into_owned(),
        "--".to_string(),
        "hello".to_string(),
    ])
    .expect("failed to parse args");

    let result: String = ensure_kernel_binary_available(&args)
        .await
        .expect("kernel should be available");
    assert_eq!(result, kernel_path.to_string_lossy());
}

#[tokio::test]
async fn ensure_kernel_binary_available_rejects_missing_kernel() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let args: Args = Args::parse(vec![
        "nanvixd".to_string(),
        "-bin-dir".to_string(),
        tmp_dir.path().to_string_lossy().into_owned(),
        "--".to_string(),
        "hello".to_string(),
    ])
    .expect("failed to parse args");

    let error: ::anyhow::Error = ensure_kernel_binary_available(&args)
        .await
        .expect_err("missing kernel should fail");
    let message: String = error.to_string();
    assert!(message.contains("not available locally"));
    assert!(message.contains(KERNEL_BINARY_NAME));
    assert!(message.contains(Args::OPT_BIN_DIRECTORY));
}

#[test]
fn log_destination_follows_log_to_stdout() {
    let args: Args = Args::parse(vec![
        "nanvixd".to_string(),
        "-log-to-stdout".to_string(),
        "--".to_string(),
        "hello".to_string(),
    ])
    .expect("failed to parse args");
    assert_eq!(log_destination(&args), LogDestination::Stdout);

    let args: Args = Args::parse(vec![
        "nanvixd".to_string(),
        "-log-dir".to_string(),
        "/tmp/nanvixd-logs".to_string(),
        "--".to_string(),
        "hello".to_string(),
    ])
    .expect("failed to parse args");
    assert_eq!(
        log_destination(&args),
        LogDestination::File {
            directory: "/tmp/nanvixd-logs".to_string(),
            discriminator: None,
        }
    );
}
