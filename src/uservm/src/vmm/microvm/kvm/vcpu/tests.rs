// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result as AnyResult;

/// Creates a minimal KVM VM with a fully initialized `VirtualProcessor` for testing.
/// Returns the `Kvm` and `VmFd` handles alongside the `VirtualProcessor` so that the KVM
/// file descriptors remain open for the lifetime of the test.
fn create_test_vcpu() -> AnyResult<(Kvm, VmFd, VirtualProcessor)> {
    let mut kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
    let mut vm: VmFd = kvm.create_vm().expect("failed to create VM");
    // Create an in-kernel IRQ chip — required for LAPIC save/restore.
    vm.create_irq_chip().expect("failed to create IRQ chip");
    let vcpu: VirtualProcessor =
        VirtualProcessor::new(&mut kvm, &mut vm, 0).expect("failed to create VirtualProcessor");
    Ok((kvm, vm, vcpu))
}

/// Verifies that `save_state` produces a non-empty `VirtualProcessorState` with populated
/// register fields and serializable CPUID/MSR/FPU byte vectors.
#[test]
fn save_state_produces_valid_snapshot() -> AnyResult<()> {
    let (kvm, _vm, vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

    let state: VirtualProcessorState = vcpu.save_state(&kvm).expect("save_state failed");

    // CPUID bytes must contain at least the kvm_cpuid2 header.
    assert!(
        state.cpuid.len() >= mem::size_of::<kvm_cpuid2>(),
        "CPUID snapshot too short (len={})",
        state.cpuid.len()
    );

    // TSC frequency must be non-zero on any modern host.
    assert!(state.tsc_khz > 0, "TSC frequency should be non-zero");

    // The state must be serializable (snapshot pipeline uses serde_cbor).
    let encoded: Vec<u8> =
        ::serde_cbor::to_vec(&state).expect("VirtualProcessorState should be serializable");
    assert!(!encoded.is_empty(), "serialized VirtualProcessorState should not be empty");

    Ok(())
}

/// MSR indices of volatile registers whose values may change between consecutive
/// reads even without guest execution (hardware counters, timers, energy meters).
/// These are excluded from the byte-for-byte round-trip comparison.
const VOLATILE_MSR_INDICES: &[u32] = &[
    0x0010, // IA32_TSC
    0x0034, // SMI_COUNT
    0x00E7, // IA32_MPERF
    0x00E8, // IA32_APERF
    0x0198, // IA32_PERF_STATUS
    0x0611, // PKG_ENERGY_STATUS
    0x0613, // PKG_PERF_STATUS
    0x0619, // DRAM_ENERGY_STATUS
    0x061B, // DRAM_PERF_STATUS
    0x06E0, // IA32_TSC_DEADLINE
];

/// Zeroes out the `data` field of every MSR entry whose index is in
/// [`VOLATILE_MSR_INDICES`]. This allows two snapshots to be compared
/// byte-for-byte even when volatile MSR values have changed between reads.
fn zero_volatile_msr_values(state: &mut msr::MsrsState) {
    let header_size: usize = mem::size_of::<::kvm_bindings::kvm_msrs>();
    let entry_size: usize = mem::size_of::<kvm_msr_entry>();

    if state.bytes.len() < header_size {
        return;
    }

    let nmsrs: usize = u32::from_ne_bytes([
        state.bytes[0],
        state.bytes[1],
        state.bytes[2],
        state.bytes[3],
    ]) as usize;

    for i in 0..nmsrs {
        let entry_offset: usize = header_size + i * entry_size;
        if entry_offset + entry_size > state.bytes.len() {
            break;
        }
        // kvm_msr_entry layout: index (u32), reserved (u32), data (u64).
        let index: u32 = u32::from_ne_bytes([
            state.bytes[entry_offset],
            state.bytes[entry_offset + 1],
            state.bytes[entry_offset + 2],
            state.bytes[entry_offset + 3],
        ]);
        if VOLATILE_MSR_INDICES.contains(&index) {
            // Zero out the data field (bytes 8..16 within the entry).
            let data_offset: usize = entry_offset + 8;
            state.bytes[data_offset..data_offset + 8].fill(0);
        }
    }
}

/// Verifies that a save → load → save round trip produces identical vCPU state.
#[test]
fn save_load_round_trip() -> AnyResult<()> {
    let (kvm, _vm, mut vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

    // Save the initial state.
    let mut state_before: VirtualProcessorState =
        vcpu.save_state(&kvm).expect("first save_state failed");

    // Load it back.
    vcpu.load_state(&state_before).expect("load_state failed");
    assert!(vcpu.is_online(), "vCPU should be online after load_state");

    // Save again and compare the two snapshots.
    let mut state_after: VirtualProcessorState =
        vcpu.save_state(&kvm).expect("second save_state failed");

    // Zero out volatile MSR values (e.g. IA32_TSC, IA32_MPERF) before comparing,
    // as they are updated by hardware/KVM between consecutive reads.
    zero_volatile_msr_values(&mut state_before.msrs_state);
    zero_volatile_msr_values(&mut state_after.msrs_state);

    // Serialize both snapshots and compare bytes — this covers all fields including
    // the opaque CPUID, FPU, and MSR byte vectors (with volatile MSRs neutralized).
    let bytes_before: Vec<u8> =
        ::serde_cbor::to_vec(&state_before).expect("serialization of state_before failed");
    let bytes_after: Vec<u8> =
        ::serde_cbor::to_vec(&state_after).expect("serialization of state_after failed");
    assert_eq!(
        bytes_before, bytes_after,
        "vCPU state should be identical after a save-load-save round trip"
    );

    Ok(())
}

/// Verifies that `deserialize_cpuid` rejects a byte vector that is too short for the header.
#[test]
fn deserialize_cpuid_rejects_truncated_header() {
    let short_data: Vec<u8> = vec![0u8; 4];
    let result: Result<CpuId> = deserialize_cpuid(&short_data);
    assert!(result.is_err(), "deserialize_cpuid should reject truncated header");
}

/// Verifies that `deserialize_cpuid` rejects a header whose `nent` field implies more entries
/// than the byte vector contains.
#[test]
fn deserialize_cpuid_rejects_truncated_entries() {
    // Create a header with nent = 10 but provide no entry bytes.
    let header_size: usize = mem::size_of::<kvm_cpuid2>();
    let mut data: Vec<u8> = vec![0u8; header_size];
    // Write nent = 10 at offset 0 (native endian).
    let nent_bytes: [u8; 4] = 10u32.to_ne_bytes();
    data[..4].copy_from_slice(&nent_bytes);

    let result: Result<CpuId> = deserialize_cpuid(&data);
    assert!(result.is_err(), "deserialize_cpuid should reject data with insufficient entries");
}

/// Verifies that the CPUID serialize → deserialize round trip preserves all entries.
#[test]
fn cpuid_serialize_deserialize_round_trip() -> AnyResult<()> {
    let (_kvm, _vm, vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

    let original: FamStructWrapper<kvm_cpuid2> = vcpu
        .fd
        .get_cpuid2(KVM_MAX_CPUID_ENTRIES)
        .expect("get_cpuid2 failed");

    let serialized: Vec<u8> = serialize_fam_struct(&original);
    let restored: CpuId = deserialize_cpuid(&serialized).expect("deserialize_cpuid failed");

    assert_eq!(original.as_slice().len(), restored.as_slice().len(), "CPUID entry count mismatch");

    for (i, (orig, rest)) in original
        .as_slice()
        .iter()
        .zip(restored.as_slice().iter())
        .enumerate()
    {
        assert_eq!(orig.function, rest.function, "CPUID entry {i}: function mismatch");
        assert_eq!(orig.index, rest.index, "CPUID entry {i}: index mismatch");
        assert_eq!(orig.eax, rest.eax, "CPUID entry {i}: eax mismatch");
        assert_eq!(orig.ebx, rest.ebx, "CPUID entry {i}: ebx mismatch");
        assert_eq!(orig.ecx, rest.ecx, "CPUID entry {i}: ecx mismatch");
        assert_eq!(orig.edx, rest.edx, "CPUID entry {i}: edx mismatch");
    }

    Ok(())
}

/// LAPIC register array offset of the LVT timer entry.
const LAPIC_LVT_TIMER_OFFSET: usize = 0x320;

/// LVT timer value placing the timer in TSC-deadline mode (bits 17:18 = 0b10) with vector
/// `0x40` and the mask bit cleared. The mode bits in this register are the canonical signal
/// that `KVM_SET_LAPIC` was actually applied to the in-kernel APIC.
const LVT_TIMER_TSC_DEADLINE_MODE: u32 = 0x0004_0040;

/// Reads the four LVT-timer bytes back from the in-kernel LAPIC.
fn read_lvt_timer(fd: &::kvm_ioctls::VcpuFd) -> u32 {
    let lapic: ::kvm_bindings::kvm_lapic_state = fd.get_lapic().expect("get_lapic failed");
    let bytes: [u8; 4] = [
        lapic.regs[LAPIC_LVT_TIMER_OFFSET] as u8,
        lapic.regs[LAPIC_LVT_TIMER_OFFSET + 1] as u8,
        lapic.regs[LAPIC_LVT_TIMER_OFFSET + 2] as u8,
        lapic.regs[LAPIC_LVT_TIMER_OFFSET + 3] as u8,
    ];
    u32::from_ne_bytes(bytes)
}

/// Contract test: `load_state` must invoke `KVM_SET_LAPIC` as part of the restore pipeline,
/// and the LAPIC bytes carried by the snapshot must survive every subsequent vCPU ioctl in
/// the sequence. This guards against accidental removal/reordering of `set_lapic` and against
/// any future ioctl in `load_state` overwriting LAPIC state.
///
/// Note: this test does NOT specifically detect a `SET_MSRS`-before-`SET_LAPIC` ordering
/// regression — its symptom (spurious early timer IRQ on resume) only surfaces while the
/// guest is executing. The integration/benchmark-level coverage in
/// `nanvix-bench -benchmark snapshot-restore` is the authoritative regression guard for it.
#[test]
fn load_state_applies_lapic_after_other_vcpu_ioctls() -> AnyResult<()> {
    let (kvm, _vm, mut vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

    let mut state: VirtualProcessorState = vcpu.save_state(&kvm).expect("save_state failed");

    // Stamp the LVT timer register with a distinctive value in the snapshot.
    let lvt_bytes: [u8; 4] = LVT_TIMER_TSC_DEADLINE_MODE.to_ne_bytes();
    for (i, b) in lvt_bytes.iter().enumerate() {
        state.lapic.regs[LAPIC_LVT_TIMER_OFFSET + i] = (*b).cast_signed();
    }

    // Sanity-check: ensure the LVT is *not* already in this configuration on a fresh vCPU.
    // Otherwise the test cannot tell whether `load_state` actually wrote the LAPIC.
    let lvt_before: u32 = read_lvt_timer(&vcpu.fd);
    assert_ne!(
        lvt_before, LVT_TIMER_TSC_DEADLINE_MODE,
        "test setup is invalid: fresh vCPU already has the fingerprint LVT value"
    );

    vcpu.load_state(&state).expect("load_state failed");

    let lvt_after: u32 = read_lvt_timer(&vcpu.fd);
    assert_eq!(
        lvt_after, LVT_TIMER_TSC_DEADLINE_MODE,
        "LAPIC LVT-timer fingerprint not present after load_state — KVM_SET_LAPIC was either \
         skipped or overwritten by a subsequent ioctl"
    );

    Ok(())
}
