// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::multiimage::{
    ImageEntry,
    MultiImageHeader,
    find_entry_by_tag,
    parse_entries,
    parse_header,
};
use ::vfs_bench_common::ACK_ERR;

/// Size of the FAT images that the tests create.
const TEST_IMAGE_SIZE: usize = 1024 * 1024;

/// Creates a small FAT image in which the scratch directory sits next to regular content.
fn create_test_image(path: &Path) {
    std::fs::write(path, vec![0u8; TEST_IMAGE_SIZE]).expect("failed to create image file");
    let file: std::fs::File = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("failed to open image file");
    let mut storage = fatfs::StdIoWrapper::new(file);
    fatfs::format_volume(&mut storage, fatfs::FormatVolumeOptions::new())
        .expect("failed to format image");

    let fs = fatfs::FileSystem::new(storage, fatfs::FsOptions::new())
        .expect("failed to open formatted image");
    let root = fs.root_dir();
    let scratch = root
        .create_dir(SCRATCH_DIR.trim_start_matches('/'))
        .expect("failed to create scratch directory");
    let dir = root.create_dir("d0").expect("failed to create directory");

    for (parent, name, size) in [
        (&root, "bulk0.dat", 4096),
        (&dir, "f0.dat", 100),
        (&scratch, "tmpfile.dat", 10),
    ] {
        let mut file = parent.create_file(name).expect("failed to create file");
        fatfs::Write::write_all(&mut file, &vec![0x42u8; size]).expect("failed to write file");
        fatfs::Write::flush(&mut file).expect("failed to flush file");
    }
}

#[test]
fn scan_fat_image_skips_scratch_directory() {
    let dir: TempDir = ::tempfile::tempdir().expect("failed to create temporary directory");
    let image: PathBuf = dir.path().join("image.img");
    create_test_image(&image);

    let mut manifest: FatManifest = scan_fat_image(&image).expect("failed to scan image");
    manifest.files.sort();
    manifest.dirs.sort();

    assert_eq!(manifest.files, ["/bulk0.dat", "/d0/f0.dat"]);
    assert_eq!(manifest.dirs, ["/", "/d0"]);
    assert_eq!(manifest.file_sizes.len(), manifest.files.len());
}

#[test]
fn root_container_selects_mount_mode() {
    let dir: TempDir = ::tempfile::tempdir().expect("failed to create temporary directory");
    let image: PathBuf = dir.path().join("image.img");
    let contents: Vec<u8> = (0..5000).map(|i| (i % 251) as u8).collect();
    std::fs::write(&image, &contents).expect("failed to write image");

    for readonly in [false, true] {
        let container: PathBuf = dir.path().join(format!("container-{readonly}.img"));
        build_root_container(&image, readonly, &container).expect("failed to build container");

        let data: Vec<u8> = std::fs::read(&container).expect("failed to read container");
        let header: MultiImageHeader = parse_header(&data).expect("failed to parse header");
        assert_eq!(header.num_images, 1);

        let entries: &[ImageEntry] =
            parse_entries(&data, header.num_images).expect("failed to parse entries");
        let rootfs: &ImageEntry =
            find_entry_by_tag(entries, &ROOTFS_MMIO_TAG).expect("container has no ROOTFS entry");
        assert_eq!(rootfs.is_readonly(), readonly);

        let start: usize = rootfs.offset as usize;
        assert_eq!(&data[start..start + rootfs.size as usize], contents);
    }
}

#[test]
fn root_container_reports_missing_image() {
    let dir: TempDir = ::tempfile::tempdir().expect("failed to create temporary directory");
    let image: PathBuf = dir.path().join("missing.img");

    let error: anyhow::Error =
        build_root_container(&image, false, &dir.path().join("container.img"))
            .expect_err("wrapping a missing image should fail");

    assert!(error.to_string().contains("missing.img"), "unexpected error: {error}");
}

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

#[test]
fn verify_mount_mode_accepts_the_requested_mode() {
    assert!(verify_mount_mode(false, ACK_OK).is_ok());
    assert!(verify_mount_mode(true, ACK_READ_ONLY).is_ok());
}

#[test]
fn verify_mount_mode_rejects_the_other_mode() {
    let writable: anyhow::Error = verify_mount_mode(false, ACK_READ_ONLY)
        .expect_err("a writable mount that rejects a write is misconfigured");
    assert!(writable.to_string().contains("should be writable"), "unexpected error: {writable}");

    let readonly: anyhow::Error = verify_mount_mode(true, ACK_OK)
        .expect_err("a read-only mount that accepts a write is misconfigured");
    assert!(readonly.to_string().contains("should be read-only"), "unexpected error: {readonly}");
}

#[test]
fn verify_mount_mode_rejects_a_failure_that_is_not_a_read_only_rejection() {
    // The guest reports every failure other than a read-only rejection as `ACK_ERR`.
    for readonly in [false, true] {
        let error: anyhow::Error = verify_mount_mode(readonly, ACK_ERR)
            .expect_err("an unrelated failure does not prove the mount mode");
        assert!(error.to_string().contains("ack=0xff"), "unexpected error: {error}");
    }
}
