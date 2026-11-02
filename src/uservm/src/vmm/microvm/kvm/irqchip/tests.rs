// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result as AnyResult;

/// Distinctive IMR fingerprint written to the PIC slave before snapshotting. The reset
/// value is `0x00`, so any non-zero pattern reliably detects whether the chip was actually
/// captured and restored.
const SLAVE_IMR_FINGERPRINT: u8 = 0x5A;

/// Distinctive redirection-entry fingerprint written to the IOAPIC before snapshotting.
/// Bit 16 (interrupt mask) is intentionally set so KVM accepts the value verbatim — this
/// register field has no cross-state validation in `KVM_SET_IRQCHIP`.
const IOAPIC_REDIRTBL0_FINGERPRINT: u64 = 0x0000_0000_0001_00A3;

/// Creates a minimal KVM VM with an attached in-kernel IRQ chip for testing.
fn create_test_irqchip() -> AnyResult<(Kvm, VmFd, IrqChip)> {
    let mut kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
    let mut vm: VmFd = kvm.create_vm().expect("failed to create VM");
    let irqchip: IrqChip = IrqChip::new(&mut kvm, &mut vm).expect("failed to create IrqChip");
    Ok((kvm, vm, irqchip))
}

/// Reads the current state of a single chip identified by `chip_id`.
fn read_chip(vm: &VmFd, chip_id: u32) -> kvm_irqchip {
    let mut chip: kvm_irqchip = kvm_irqchip {
        chip_id,
        ..Default::default()
    };
    vm.get_irqchip(&mut chip).expect("get_irqchip failed");
    chip
}

/// Reads, mutates with `f`, and writes back a single chip.
fn rmw_chip<F: FnOnce(&mut kvm_irqchip)>(vm: &VmFd, chip_id: u32, f: F) {
    let mut chip: kvm_irqchip = read_chip(vm, chip_id);
    f(&mut chip);
    vm.set_irqchip(&chip).expect("set_irqchip failed");
}

/// Verifies that `save_state` followed by `restore_state` preserves state across all three
/// in-kernel chips (PIC master, PIC slave, IOAPIC).
///
/// Guards against the failure mode where only the master PIC (`chip_id = 0`) is captured
/// because `KVM_GET_IRQCHIP` is a per-chip ioctl driven by the caller-supplied `chip_id`.
/// In that case the PIC slave and IOAPIC come back zeroed on restore, scrambling IRQ
/// delivery and triggering an early kernel exception in the guest.
#[test]
fn restore_state_preserves_all_three_chips() -> AnyResult<()> {
    let (_kvm, vm, mut irqchip): (Kvm, VmFd, IrqChip) = create_test_irqchip()?;

    // Stamp a fingerprint into PIC slave and IOAPIC redirtbl[0]. The master PIC is left at
    // reset values — the existing (pre-fix) save path would have captured only the master,
    // so the slave/IOAPIC fingerprints are what discriminate the bug.
    // Writes to fields of a `Copy` union variant are safe in Rust; the variant is
    // implicitly selected by the field path (`pic` for PIC chips, `ioapic` for IOAPIC).
    rmw_chip(&vm, KVM_IRQCHIP_PIC_SLAVE, |chip| {
        chip.chip.pic.imr = SLAVE_IMR_FINGERPRINT;
    });
    rmw_chip(&vm, KVM_IRQCHIP_IOAPIC, |chip| {
        // SAFETY: chip_id = IOAPIC selects the `ioapic` union variant, whose memory was
        // initialized by the preceding `get_irqchip` call.
        unsafe {
            chip.chip.ioapic.redirtbl[0].bits = IOAPIC_REDIRTBL0_FINGERPRINT;
        }
    });

    // Capture the (stamped) state.
    let state: IrqChipState = irqchip.save_state(&vm).expect("save_state failed");

    // Wipe the fingerprints to prove the restore is what brings them back.
    rmw_chip(&vm, KVM_IRQCHIP_PIC_SLAVE, |chip| {
        chip.chip.pic.imr = 0x00;
    });
    rmw_chip(&vm, KVM_IRQCHIP_IOAPIC, |chip| {
        // SAFETY: see the matching write above.
        unsafe {
            chip.chip.ioapic.redirtbl[0].bits = 0;
        }
    });

    // Sanity-check the wipe.
    let slave_wiped: kvm_irqchip = read_chip(&vm, KVM_IRQCHIP_PIC_SLAVE);
    // SAFETY: see comment above.
    assert_eq!(unsafe { slave_wiped.chip.pic.imr }, 0x00, "PIC slave wipe should clear IMR");
    let ioapic_wiped: kvm_irqchip = read_chip(&vm, KVM_IRQCHIP_IOAPIC);
    assert_eq!(
        // SAFETY: see comment above.
        unsafe { ioapic_wiped.chip.ioapic.redirtbl[0].bits },
        0,
        "IOAPIC wipe should clear redirtbl[0]"
    );

    // Restore and verify each chip's fingerprint is back.
    irqchip
        .restore_state(&vm, &state)
        .expect("restore_state failed");

    let slave_restored: kvm_irqchip = read_chip(&vm, KVM_IRQCHIP_PIC_SLAVE);
    assert_eq!(
        // SAFETY: see comment above.
        unsafe { slave_restored.chip.pic.imr },
        SLAVE_IMR_FINGERPRINT,
        "PIC slave IMR was not restored — save/restore is dropping chip_id = 1"
    );

    let ioapic_restored: kvm_irqchip = read_chip(&vm, KVM_IRQCHIP_IOAPIC);
    assert_eq!(
        // SAFETY: see comment above.
        unsafe { ioapic_restored.chip.ioapic.redirtbl[0].bits },
        IOAPIC_REDIRTBL0_FINGERPRINT,
        "IOAPIC redirtbl[0] was not restored — save/restore is dropping chip_id = 2"
    );

    Ok(())
}

/// Verifies that `save_state` produces a serialized blob large enough to hold all three
/// chips. A single-chip regression would produce a blob roughly one-third the size.
///
/// Each `kvm_irqchip` is 520 bytes (`chip_id: u32 + pad: u32 + 512-byte union`). Three
/// chips therefore require at least 1536 bytes of payload, plus modest CBOR framing.
#[test]
fn save_state_serialization_covers_all_three_chips() -> AnyResult<()> {
    let (_kvm, vm, irqchip): (Kvm, VmFd, IrqChip) = create_test_irqchip()?;

    let state: IrqChipState = irqchip.save_state(&vm).expect("save_state failed");
    let encoded: Vec<u8> =
        ::serde_cbor::to_vec(&state).expect("IrqChipState should be serializable");

    let min_three_chip_payload: usize = 3 * 512;
    assert!(
        encoded.len() >= min_three_chip_payload,
        "IrqChipState serialization is {} bytes; expected at least {} (only the PIC master would \
         be captured by a single-chip regression)",
        encoded.len(),
        min_three_chip_payload,
    );

    Ok(())
}
