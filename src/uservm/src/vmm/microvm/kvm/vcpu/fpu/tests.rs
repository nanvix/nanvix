// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result as AnyResult;
use ::kvm_ioctls::VmFd;

/// Creates a minimal KVM VM with one vCPU for testing.
/// Returns the `Kvm` and `VmFd` handles alongside the `VcpuFd` so that the KVM
/// file descriptors remain open for the lifetime of the test.
fn create_test_vcpu() -> AnyResult<(Kvm, VmFd, VcpuFd)> {
    let kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
    let vm: VmFd = kvm.create_vm().expect("failed to create VM");
    let vcpu: VcpuFd = vm.create_vcpu(0).expect("failed to create vCPU");
    Ok((kvm, vm, vcpu))
}

/// Verifies that `save_state` followed by `load_state` followed by another `save_state`
/// produces identical FPU state bytes (round-trip invariant).
#[test]
fn save_load_round_trip() -> AnyResult<()> {
    let (kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
    let fpu: Fpu = Fpu::new(&kvm, &vcpu_fd).expect("failed to create FPU");

    // Save the initial FPU state.
    let state_before: FpuState = fpu.save_state(&vcpu_fd).expect("first save_state failed");
    assert!(!state_before.xsave_bytes.is_empty(), "saved FPU state should not be empty");

    // Load the saved state back.
    fpu.load_state(&vcpu_fd, &state_before)
        .expect("load_state failed");

    // Save again and verify the bytes are identical.
    let state_after: FpuState = fpu.save_state(&vcpu_fd).expect("second save_state failed");
    assert_eq!(
        state_before.xsave_bytes, state_after.xsave_bytes,
        "FPU state should be identical after a save-load-save round trip"
    );

    Ok(())
}

/// Verifies that `load_state` rejects data that is too short for the xsave header.
#[test]
fn load_state_rejects_truncated_data() -> AnyResult<()> {
    let (kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
    let fpu: Fpu = Fpu::new(&kvm, &vcpu_fd).expect("failed to create FPU");

    let bad_state: FpuState = FpuState {
        xsave_bytes: vec![0u8; 4],
    };
    let result: AnyResult<()> = fpu.load_state(&vcpu_fd, &bad_state);
    assert!(result.is_err(), "load_state should reject truncated data");

    Ok(())
}
