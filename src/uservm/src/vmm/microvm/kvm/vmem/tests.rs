// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result as AnyResult;
use ::std::{
    env,
    fs,
    path::PathBuf,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

/// Minimum VM memory size used by tests (1 page).
const TEST_MEM_SIZE: usize = PAGE_SIZE;

/// Creates a KVM VM with a `VirtualMemory` region for testing.
/// Returns the `Kvm` and `VmFd` handles alongside the `VirtualMemory` so that the KVM
/// file descriptors remain open for the lifetime of the test.
fn create_test_vmem() -> AnyResult<(Kvm, VmFd, VirtualMemory)> {
    let mut kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
    let mut vm: VmFd = kvm.create_vm().expect("failed to create VM");
    let vmem: VirtualMemory =
        VirtualMemory::new(&mut kvm, &mut vm, TEST_MEM_SIZE).expect("failed to create vmem");
    Ok((kvm, vm, vmem))
}

/// Returns a unique temporary file path for snapshot tests.
fn unique_snapshot_path(suffix: &str) -> PathBuf {
    let nanos: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("failed to compute timestamp")
        .as_nanos();
    let mut path: PathBuf = env::temp_dir();
    path.push(format!("nanvix-vmem-{suffix}-{nanos}.vmem"));
    path
}

/// Verifies that `save_snapshot` creates a file with the expected size.
#[test]
fn save_snapshot_creates_correctly_sized_file() -> AnyResult<()> {
    let (_kvm, _vm, vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
    let path: PathBuf = unique_snapshot_path("save-size");

    vmem.save_snapshot(&path).expect("save_snapshot failed");

    let file_len: u64 = fs::metadata(&path).expect("failed to read metadata").len();
    let expected: u64 = (SNAPSHOT_DATA_OFFSET + TEST_MEM_SIZE) as u64;
    assert_eq!(file_len, expected, "snapshot file should be header + padding + memory");

    fs::remove_file(&path).ok();
    Ok(())
}

/// Verifies that a save → load round trip preserves memory contents.
#[test]
fn save_load_round_trip_preserves_contents() -> AnyResult<()> {
    let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
    let path: PathBuf = unique_snapshot_path("round-trip");

    // Write a recognizable pattern into guest memory.
    // NOTE: 251 is prime and fits in u8, so (i % 251) is always in [0, 250].
    let pattern: Vec<u8> = (0..TEST_MEM_SIZE)
        .map(|i| u8::try_from(i % 251).expect("i % 251 always fits in u8"))
        .collect();
    vmem.write_bytes(0, &pattern).expect("write_bytes failed");

    // Save snapshot.
    vmem.save_snapshot(&path).expect("save_snapshot failed");

    // Zero out memory so we can confirm load restores it.
    let zeros: Vec<u8> = vec![0u8; TEST_MEM_SIZE];
    vmem.write_bytes(0, &zeros)
        .expect("write_bytes (zero) failed");

    // Load snapshot.
    vmem.load_snapshot(&path).expect("load_snapshot failed");

    // Read back and verify.
    let mut readback: Vec<u8> = vec![0u8; TEST_MEM_SIZE];
    vmem.read_bytes(0, &mut readback)
        .expect("read_bytes failed");
    assert_eq!(readback, pattern, "memory contents should match after save-load round trip");

    fs::remove_file(&path).ok();
    Ok(())
}

/// Verifies that `load_snapshot` fails gracefully when the file does not exist.
#[test]
fn load_snapshot_rejects_missing_file() -> AnyResult<()> {
    let path: PathBuf = unique_snapshot_path("missing");

    let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
    let result: AnyResult<()> = vmem.load_snapshot(&path);
    assert!(result.is_err(), "load_snapshot should fail for a non-existent file");

    Ok(())
}

/// Verifies that `load_snapshot` rejects a snapshot file that is smaller than the guest
/// memory mapping it must back. This guards against MAP_FIXED-on-short-file successes that
/// would otherwise cause SIGBUS on later guest accesses past EOF. The boundary case
/// (`required_size - 1`) is exercised to pin the strict `<` comparison in the size check.
#[test]
fn load_snapshot_rejects_undersized_file() -> AnyResult<()> {
    let (_kvm, _vm, mut vmem): (Kvm, VmFd, VirtualMemory) = create_test_vmem()?;
    let required_size: usize = SNAPSHOT_DATA_OFFSET + TEST_MEM_SIZE;

    // Case 1: file containing only the data-offset padding, no memory contents.
    let path_empty: PathBuf = unique_snapshot_path("undersized-empty");
    let truncated_empty: Vec<u8> = vec![0u8; SNAPSHOT_DATA_OFFSET];
    fs::write(&path_empty, &truncated_empty).expect("failed to write undersized snapshot file");
    let result_empty: AnyResult<()> = vmem.load_snapshot(&path_empty);
    fs::remove_file(&path_empty).ok();
    assert!(result_empty.is_err(), "load_snapshot should fail for an empty-data file");

    // Case 2: boundary — exactly one byte short of the required size.
    let path_boundary: PathBuf = unique_snapshot_path("undersized-boundary");
    let truncated_boundary: Vec<u8> = vec![0u8; required_size - 1];
    fs::write(&path_boundary, &truncated_boundary)
        .expect("failed to write boundary-sized snapshot file");
    let result_boundary: AnyResult<()> = vmem.load_snapshot(&path_boundary);
    fs::remove_file(&path_boundary).ok();
    assert!(
        result_boundary.is_err(),
        "load_snapshot should fail for a file one byte short of required size"
    );

    Ok(())
}
