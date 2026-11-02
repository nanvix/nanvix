// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result as AnyResult;
use ::kvm_bindings::KVM_MAX_CPUID_ENTRIES;
use ::kvm_ioctls::VmFd;

/// Returns `true` when `index` falls inside the half-open range
/// `[msr.as_u32(), msr.as_u32() + msr.range_count())`.
fn msr_range_contains(msr: MsrIndex, index: u32) -> bool {
    index >= msr.as_u32() && index < msr.as_u32() + msr.range_count()
}

/// Returns `true` when the MSR at `index` is in the serializable allowlist.
fn msr_should_serialize(index: u32) -> bool {
    REGULAR_MSRS
        .iter()
        .chain(DEFERRED_MSRS.iter())
        .any(|&msr| msr_range_contains(msr, index))
}

/// Creates a minimal KVM VM with one vCPU that has CPUID configured.
/// CPUID setup is required so that `get_msrs` can read feature MSRs.
/// Returns the `Kvm` and `VmFd` handles alongside the `VcpuFd` so that the KVM
/// file descriptors remain open for the lifetime of the test.
fn create_test_vcpu() -> AnyResult<(Kvm, VmFd, VcpuFd)> {
    let kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
    let vm: VmFd = kvm.create_vm().expect("failed to create VM");
    let vcpu: VcpuFd = vm.create_vcpu(0).expect("failed to create vCPU");
    // Set up CPUID so the vCPU advertises supported features to KVM.
    let cpuid = kvm
        .get_supported_cpuid(KVM_MAX_CPUID_ENTRIES)
        .expect("failed to get supported CPUID");
    vcpu.set_cpuid2(&cpuid).expect("failed to set CPUID");
    Ok((kvm, vm, vcpu))
}

/// Verifies that `save_state` produces a non-empty serialized MSR snapshot.
#[test]
fn save_state_produces_non_empty_snapshot() -> AnyResult<()> {
    let (kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
    let msrs: Msrs = Msrs;

    let state: MsrsState = msrs.save_state(&kvm, &vcpu_fd).expect("save_state failed");

    // The serialized bytes must contain at least the kvm_msrs header.
    let header_size: usize = std::mem::size_of::<::kvm_bindings::kvm_msrs>();
    assert!(state.bytes.len() >= header_size, "MSR snapshot too short (len={})", state.bytes.len());

    // nmsrs (first 4 bytes) should be non-zero on any modern host.
    let nmsrs: u32 = u32::from_ne_bytes([
        state.bytes[0],
        state.bytes[1],
        state.bytes[2],
        state.bytes[3],
    ]);
    assert!(nmsrs > 0, "MSR snapshot should contain at least one entry");

    Ok(())
}

/// Verifies that a save → load → save round trip succeeds without error.
/// Note: some MSRs are read-only, so we compare entry counts rather than exact byte equality.
#[test]
fn save_load_round_trip() -> AnyResult<()> {
    let (kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
    let msrs: Msrs = Msrs;

    // Save the initial MSR state.
    let state_before: MsrsState = msrs.save_state(&kvm, &vcpu_fd).expect("first save failed");

    // Load it back.
    msrs.load_state(&vcpu_fd, &state_before)
        .expect("load_state failed");

    // Save again.
    let state_after: MsrsState = msrs.save_state(&kvm, &vcpu_fd).expect("second save failed");

    // Both snapshots must have the same number of MSR entries.
    let nmsrs_before: u32 = u32::from_ne_bytes([
        state_before.bytes[0],
        state_before.bytes[1],
        state_before.bytes[2],
        state_before.bytes[3],
    ]);
    let nmsrs_after: u32 = u32::from_ne_bytes([
        state_after.bytes[0],
        state_after.bytes[1],
        state_after.bytes[2],
        state_after.bytes[3],
    ]);
    assert_eq!(
        nmsrs_before, nmsrs_after,
        "MSR entry count should be identical after a save-load-save round trip"
    );

    Ok(())
}

/// Verifies that `load_state` rejects data that is too short for the kvm_msrs header.
#[test]
fn load_state_rejects_truncated_header() -> AnyResult<()> {
    let (_kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
    let msrs: Msrs = Msrs;

    let bad_state: MsrsState = MsrsState {
        bytes: vec![0u8; 4],
    };
    let result: Result<()> = msrs.load_state(&vcpu_fd, &bad_state);
    assert!(result.is_err(), "load_state should reject truncated header");

    Ok(())
}

/// Verifies that `load_state` rejects a header whose `nmsrs` field implies more entries
/// than the byte vector contains.
#[test]
fn load_state_rejects_truncated_entries() -> AnyResult<()> {
    let (_kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
    let msrs: Msrs = Msrs;

    // Create a buffer with a valid-sized header but nmsrs = 100 and no entry bytes.
    let header_size: usize = std::mem::size_of::<::kvm_bindings::kvm_msrs>();
    let mut data: Vec<u8> = vec![0u8; header_size];
    let nmsrs_bytes: [u8; 4] = 100u32.to_ne_bytes();
    data[..4].copy_from_slice(&nmsrs_bytes);

    let bad_state: MsrsState = MsrsState { bytes: data };
    let result: Result<()> = msrs.load_state(&vcpu_fd, &bad_state);
    assert!(result.is_err(), "load_state should reject data with insufficient entries");

    Ok(())
}

/// Verifies that `load_state` rejects a header whose `nmsrs` field exceeds the
/// compile-time allowlist size, preventing attacker-controlled `Vec::with_capacity`
/// allocations that could OOM-abort the process.
#[test]
fn load_state_rejects_nmsrs_exceeding_allowlist() -> AnyResult<()> {
    let (_kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
    let msrs: Msrs = Msrs;

    let header_size: usize = std::mem::size_of::<::kvm_bindings::kvm_msrs>();
    let mut data: Vec<u8> = vec![0u8; header_size];
    let oversized: u32 = u32::try_from(EXPANDED_MSR_COUNT)?.saturating_add(1);
    data[..4].copy_from_slice(&oversized.to_ne_bytes());

    let bad_state: MsrsState = MsrsState { bytes: data };
    let result: Result<()> = msrs.load_state(&vcpu_fd, &bad_state);
    let err: String = result
        .err()
        .ok_or_else(|| anyhow::anyhow!("load_state should reject oversized nmsrs"))?
        .to_string();
    assert!(err.contains("exceeds allowlist size"), "unexpected error: {err}");

    Ok(())
}

// ---- Filtering and ordering tests (no KVM required) ----

/// Verifies that well-known serializable MSRs pass the filter.
#[test]
fn msr_filter_accepts_known_serializable_msrs() {
    let expected_pass: &[u32] = &[
        MsrIndex::Ia32Tsc.as_u32(),
        MsrIndex::Ia32Apicbase.as_u32(),
        MsrIndex::Ia32SysenterCs.as_u32(),
        MsrIndex::Ia32SysenterEsp.as_u32(),
        MsrIndex::Ia32SysenterEip.as_u32(),
        MsrIndex::Ia32MiscEnable.as_u32(),
        MsrIndex::Efer.as_u32(),
        MsrIndex::Star.as_u32(),
        MsrIndex::Lstar.as_u32(),
        MsrIndex::Cstar.as_u32(),
        MsrIndex::SyscallMask.as_u32(),
        MsrIndex::FsBase.as_u32(),
        MsrIndex::GsBase.as_u32(),
        MsrIndex::KernelGsBase.as_u32(),
        MsrIndex::KvmSystemTimeNew.as_u32(),
        MsrIndex::Ia32TscDeadline.as_u32(),
        MsrIndex::KvmWallClockNew.as_u32(),
    ];
    for &msr in expected_pass {
        assert!(msr_should_serialize(msr), "MSR {msr:#x} should be serializable");
    }
}

/// Verifies that MSR indices outside the allowlist are rejected.
#[test]
fn msr_filter_rejects_unknown_msrs() {
    // Use arbitrary indices that are not in REGULAR_MSRS or DEFERRED_MSRS.
    let expected_reject: &[u32] = &[
        0xFFFF_FFFF, // arbitrary unknown value
        0x0000_0005, // between P5_MC_TYPE (0x01) and TSC (0x10)
        0xDEAD_BEEF, // arbitrary high value
    ];
    for &msr in expected_reject {
        assert!(!msr_should_serialize(msr), "MSR {msr:#x} should not be serializable");
    }
}

/// Verifies that MTRR range entries are accepted by the filter.
#[test]
fn msr_filter_accepts_mtrr_range() {
    // MTRR range: 0x200..0x300
    assert!(msr_should_serialize(0x200));
    assert!(msr_should_serialize(0x201));
    assert!(msr_should_serialize(0x2FF));
    assert!(!msr_should_serialize(0x300));
}

/// Verifies that the expanded MSR index list places all deferred MSRs after
/// all regular MSRs.
#[test]
fn deferred_msrs_ordered_last_in_expanded_list() {
    let regular_count: usize = count_expanded(REGULAR_MSRS);

    // Every index in the deferred tail must belong to a DEFERRED_MSRS range.
    for &idx in &EXPANDED_MSR_INDICES[regular_count..] {
        assert!(
            DEFERRED_MSRS
                .iter()
                .any(|&msr| msr_range_contains(msr, idx)),
            "Index {idx:#x} in deferred portion is not a deferred MSR"
        );
    }

    // No deferred MSR should appear in the regular portion.
    for &idx in &EXPANDED_MSR_INDICES[..regular_count] {
        assert!(
            !DEFERRED_MSRS
                .iter()
                .any(|&msr| msr_range_contains(msr, idx)),
            "Deferred MSR {idx:#x} found in regular portion"
        );
    }

    // Ia32TscDeadline must be the very last entry.
    assert_eq!(
        EXPANDED_MSR_INDICES.last().copied(),
        Some(MsrIndex::Ia32TscDeadline.as_u32()),
        "Ia32TscDeadline must be the last entry in EXPANDED_MSR_INDICES"
    );
}

// ---- fix_zero_tsc_deadline_msr tests (no KVM required) ----

/// Builds a `kvm_bindings::Msrs` from `(index, data)` pairs.
fn build_msrs(entries: &[(u32, u64)]) -> ::kvm_bindings::Msrs {
    let entries: Vec<kvm_msr_entry> = entries
        .iter()
        .map(|&(index, data)| kvm_msr_entry {
            index,
            data,
            ..Default::default()
        })
        .collect();
    ::kvm_bindings::Msrs::from_entries(&entries).expect("failed to build msrs")
}

/// Returns the `data` for the given MSR index, if present.
fn msr_data(msrs: &::kvm_bindings::Msrs, index: u32) -> Option<u64> {
    msrs.as_slice()
        .iter()
        .find(|msr| msr.index == index)
        .map(|msr| msr.data)
}

/// Verifies that a zero `IA32_TSC_DEADLINE` is replaced with the `IA32_TSC` value.
#[test]
fn fix_zero_tsc_deadline_seeds_from_tsc() {
    let tsc: u32 = MsrIndex::Ia32Tsc.as_u32();
    let deadline: u32 = MsrIndex::Ia32TscDeadline.as_u32();
    let mut msrs: ::kvm_bindings::Msrs = build_msrs(&[(tsc, 0x1234_5678), (deadline, 0)]);

    fix_zero_tsc_deadline_msr(&mut msrs);

    assert_eq!(
        msr_data(&msrs, deadline),
        Some(0x1234_5678),
        "zero IA32_TSC_DEADLINE should be seeded with the IA32_TSC value"
    );
}

/// Verifies that a non-zero `IA32_TSC_DEADLINE` is left untouched.
#[test]
fn fix_zero_tsc_deadline_preserves_nonzero() {
    let tsc: u32 = MsrIndex::Ia32Tsc.as_u32();
    let deadline: u32 = MsrIndex::Ia32TscDeadline.as_u32();
    let mut msrs: ::kvm_bindings::Msrs = build_msrs(&[(tsc, 0x1234_5678), (deadline, 0x9999)]);

    fix_zero_tsc_deadline_msr(&mut msrs);

    assert_eq!(
        msr_data(&msrs, deadline),
        Some(0x9999),
        "non-zero IA32_TSC_DEADLINE must not be modified"
    );
}

/// Verifies that the deadline is left at zero when no `IA32_TSC` entry is present.
#[test]
fn fix_zero_tsc_deadline_no_tsc_entry() {
    let deadline: u32 = MsrIndex::Ia32TscDeadline.as_u32();
    let mut msrs: ::kvm_bindings::Msrs = build_msrs(&[(deadline, 0)]);

    fix_zero_tsc_deadline_msr(&mut msrs);

    assert_eq!(
        msr_data(&msrs, deadline),
        Some(0),
        "without an IA32_TSC entry the deadline must stay unchanged"
    );
}
