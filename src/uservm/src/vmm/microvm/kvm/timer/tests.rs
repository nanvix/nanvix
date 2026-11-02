// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result as AnyResult;

/// Creates a KVM VM with a PIT2 timer attached for testing.
/// Returns the `Kvm` and `VmFd` handles alongside the `Timer` so that the KVM
/// file descriptors remain open for the lifetime of the test.
fn create_test_vm_with_timer() -> AnyResult<(Kvm, VmFd, Timer)> {
    let mut kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
    let mut vm: VmFd = kvm.create_vm().expect("failed to create VM");
    // IRQ chip is required for the PIT to function.
    vm.create_irq_chip().expect("failed to create IRQ chip");
    let timer: Timer = Timer::new(&mut kvm, &mut vm).expect("failed to create Timer");
    Ok((kvm, vm, timer))
}

/// Verifies that `save_state` produces a `TimerState` that is serializable.
#[test]
fn save_state_produces_serializable_snapshot() -> AnyResult<()> {
    let (_kvm, vm, timer): (Kvm, VmFd, Timer) = create_test_vm_with_timer()?;

    let state: TimerState = timer.save_state(&vm).expect("save_state failed");

    let encoded: Vec<u8> = ::serde_cbor::to_vec(&state).expect("TimerState should be serializable");
    assert!(!encoded.is_empty(), "serialized TimerState should not be empty");

    Ok(())
}

/// Verifies that a save → restore → save round trip succeeds without error.
/// Note: PIT counters advance between saves, so we verify operability rather than byte
/// equality.
#[test]
fn save_restore_round_trip() -> AnyResult<()> {
    let (_kvm, vm, mut timer): (Kvm, VmFd, Timer) = create_test_vm_with_timer()?;

    // Save the initial timer state.
    let state_before: TimerState = timer.save_state(&vm).expect("first save_state failed");

    // Restore it.
    timer
        .restore_state(&vm, &state_before)
        .expect("restore_state failed");

    // Save again — this must succeed, proving the timer is in a valid state after restore.
    let _state_after: TimerState = timer.save_state(&vm).expect("second save_state failed");

    Ok(())
}

/// Verifies that `restore_state` clears read-only clock flags so `set_clock` does not fail.
#[test]
fn restore_state_clears_clock_flags() -> AnyResult<()> {
    let (_kvm, vm, mut timer): (Kvm, VmFd, Timer) = create_test_vm_with_timer()?;

    let mut state: TimerState = timer.save_state(&vm).expect("save_state failed");

    // Artificially set a read-only flag that would cause EINVAL if passed through.
    state.clock_data.flags = 0xFFFF_FFFF;

    // restore_state should clear flags internally and succeed.
    timer
        .restore_state(&vm, &state)
        .expect("restore_state should succeed even with dirty clock flags");

    Ok(())
}
