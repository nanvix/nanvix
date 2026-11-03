// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Save and restore Model-Specific Registers (MSRs) for KVM vCPUs.
//!
//! This module defines an allowlist of MSR indices — split into regular and deferred sets — and
//! expands their ranges into a flat index table at compile time. At runtime, `save_state()` filters
//! the pre-expanded table against the host-supported MSR list, reads the values, and serializes
//! them. `load_state()` deserializes the snapshot and writes the MSR values back. Deferred MSRs
//! (e.g. `IA32_TSC_DEADLINE`) are placed at the tail of the table so that their dependencies
//! (e.g. `IA32_TSC`) are always restored first.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::kvm::vcpu::serialize_fam_struct;
use ::anyhow::Result;
use ::arch::cpu::msr::MsrIndex;
use ::kvm_bindings::{
    kvm_msr_entry,
    kvm_msr_list,
};
use ::kvm_ioctls::{
    Kvm,
    VcpuFd,
};
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::collections::HashSet;
use ::vmm_sys_util::fam::FamStructWrapper;

//==================================================================================================
// Constants
//==================================================================================================

/// Allowlist of regular MSR indices that should be serialized in snapshots.
/// Each variant covers `range_count()` consecutive registers starting at its address.
/// Based on the Firecracker SERIALIZABLE_MSR_RANGES list (sorted by address).
/// Reference: <https://github.com/firecracker-microvm/firecracker/blob/main/src/vmm/src/arch/x86_64/msr.rs>
const REGULAR_MSRS: &[MsrIndex] = &[
    MsrIndex::Ia32P5McAddr,
    MsrIndex::Ia32P5McType,
    MsrIndex::Ia32Tsc,
    MsrIndex::Ia32PlatformId,
    MsrIndex::Ia32Apicbase,
    MsrIndex::Ia32EblCrPoweron,
    MsrIndex::EbcFrequencyId,
    MsrIndex::SmiCount,
    MsrIndex::Ia32FeatCtl,
    MsrIndex::Ia32TscAdjust,
    MsrIndex::Ia32SpecCtrl,
    MsrIndex::Ia32PredCmd,
    MsrIndex::Ia32UcodeWrite,
    MsrIndex::Ia32UcodeRev,
    MsrIndex::Ia32Smbase,
    MsrIndex::FsbFreq,
    MsrIndex::PlatformInfo,
    MsrIndex::PkgCstConfigControl,
    MsrIndex::Ia32Mperf,
    MsrIndex::Ia32Aperf,
    MsrIndex::MtrrCap,
    MsrIndex::Ia32ArchCapabilities,
    MsrIndex::Ia32TsxCtrl,
    MsrIndex::Ia32BblCrCtl3,
    MsrIndex::MiscFeaturesEnables,
    MsrIndex::Ia32SysenterCs,
    MsrIndex::Ia32SysenterEsp,
    MsrIndex::Ia32SysenterEip,
    MsrIndex::Ia32McgCap,
    MsrIndex::Ia32McgStatus,
    MsrIndex::Ia32PerfStatus,
    MsrIndex::Ia32MiscEnable,
    MsrIndex::MiscFeatureControl,
    MsrIndex::MiscPwrMgmt,
    MsrIndex::TurboRatioLimit,
    MsrIndex::TurboRatioLimit1,
    MsrIndex::Ia32Debugctlmsr,
    MsrIndex::Ia32Lastbranchfromip,
    MsrIndex::Ia32Lastbranchtoip,
    MsrIndex::Ia32Lastintfromip,
    MsrIndex::Ia32Lastinttoip,
    MsrIndex::Ia32PowerCtl,
    MsrIndex::Ia32MtrrPhysbase0,
    MsrIndex::CoreC3Residency,
    MsrIndex::Ia32Mc0Ctl,
    MsrIndex::RaplPowerUnit,
    MsrIndex::Pkgc3Irtl,
    MsrIndex::PkgPowerLimit,
    MsrIndex::PkgEnergyStatus,
    MsrIndex::PkgPerfStatus,
    MsrIndex::PkgPowerInfo,
    MsrIndex::DramPowerLimit,
    MsrIndex::DramEnergyStatus,
    MsrIndex::DramPerfStatus,
    MsrIndex::DramPowerInfo,
    MsrIndex::ConfigTdpNominal,
    MsrIndex::ConfigTdpLevel1,
    MsrIndex::ConfigTdpLevel2,
    MsrIndex::ConfigTdpControl,
    MsrIndex::TurboActivationRatio,
    MsrIndex::ApicBase,
    MsrIndex::KvmWallClockNew,
    MsrIndex::KvmSystemTimeNew,
    MsrIndex::KvmAsyncPfEn,
    MsrIndex::KvmStealTime,
    MsrIndex::KvmPvEoiEn,
    MsrIndex::KvmPollControl,
    MsrIndex::KvmAsyncPfInt,
    MsrIndex::Efer,
    MsrIndex::Star,
    MsrIndex::Lstar,
    MsrIndex::Cstar,
    MsrIndex::SyscallMask,
    MsrIndex::FsBase,
    MsrIndex::GsBase,
    MsrIndex::KernelGsBase,
    MsrIndex::TscAux,
    MsrIndex::K7Hwcr,
];

/// MSRs that must be restored **after** all regular MSRs because their semantics
/// depend on the values of other MSRs. For example, `Ia32TscDeadline` must be restored
/// after `Ia32Tsc`; otherwise KVM may fail to prime the APIC timer correctly.
/// Reference: <https://github.com/firecracker-microvm/firecracker/blob/main/src/vmm/src/arch/x86_64/vcpu.rs>
const DEFERRED_MSRS: &[MsrIndex] = &[MsrIndex::Ia32TscDeadline];

///
/// # Description
///
/// Returns the total number of individual MSR indices after expanding all ranges in the given
/// slice.
///
/// # Parameters
///
/// - `msrs`: Slice of MSR index variants to count.
///
/// # Returns
///
/// The sum of `range_count()` across all entries in `msrs`.
///
const fn count_expanded(msrs: &[MsrIndex]) -> usize {
    let mut total: usize = 0;
    let mut i: usize = 0;
    while i < msrs.len() {
        total += msrs[i].range_count() as usize;
        i += 1;
    }
    total
}

/// Total number of individual MSR indices in the expanded allowlist.
const EXPANDED_MSR_COUNT: usize = count_expanded(REGULAR_MSRS) + count_expanded(DEFERRED_MSRS);

///
/// # Description
///
/// Expands MSR index ranges from `src` into individual `u32` indices in `dst`, starting at
/// position `pos`.
///
/// # Parameters
///
/// - `src`: Slice of MSR index variants whose ranges will be expanded.
/// - `dst`: Destination array to write the expanded indices into.
/// - `pos`: Starting position in `dst`.
///
/// # Returns
///
/// The next write position in `dst` after all entries from `src` have been expanded.
///
const fn expand_into(
    src: &[MsrIndex],
    dst: &mut [u32; EXPANDED_MSR_COUNT],
    mut pos: usize,
) -> usize {
    let mut i: usize = 0;
    while i < src.len() {
        let base: u32 = src[i].as_u32();
        let count: u32 = src[i].range_count();
        let mut j: u32 = 0;
        while j < count {
            dst[pos] = base + j;
            pos += 1;
            j += 1;
        }
        i += 1;
    }
    pos
}

///
/// # Description
///
/// Expands `REGULAR_MSRS` followed by `DEFERRED_MSRS` into individual `u32` indices at compile
/// time. Regular MSRs come first; deferred MSRs are appended at the tail.
///
/// # Returns
///
/// A fixed-size array containing all expanded MSR indices in the correct order.
///
const fn expand_msr_indices() -> [u32; EXPANDED_MSR_COUNT] {
    let mut result: [u32; EXPANDED_MSR_COUNT] = [0u32; EXPANDED_MSR_COUNT];
    let pos: usize = expand_into(REGULAR_MSRS, &mut result, 0);
    let final_pos: usize = expand_into(DEFERRED_MSRS, &mut result, pos);
    assert!(final_pos == EXPANDED_MSR_COUNT);
    result
}

/// Pre-expanded list of all individual MSR indices to serialize, ordered so that
/// deferred MSRs appear at the tail. Built at compile time from `REGULAR_MSRS`
/// and `DEFERRED_MSRS`.
static EXPANDED_MSR_INDICES: [u32; EXPANDED_MSR_COUNT] = expand_msr_indices();

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// MSRs device.
///
#[derive(Default)]
pub struct Msrs;

///
/// # Description
///
/// MSRs state.
///
#[derive(Serialize, Deserialize)]
pub struct MsrsState {
    /// Serialized MSRs.
    pub(super) bytes: Vec<u8>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Msrs {
    ///
    /// # Description
    ///
    /// Saves the state of the MSRs device.
    ///
    /// # Parameters
    ///
    /// - `kvm`: Handle to the KVM.
    /// - `fd`: Handle to the virtual CPU.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function returns the MSRs state. Otherwise, it returns an
    /// error.
    ///
    /// # Note
    ///
    /// This function saves only MSRs present in a static allowlist, rather than saving every MSR
    /// the vCPU model supports. The allowlist is intentionally broad and not filtered by guest
    /// CPUID or CPU vendor. This is safe because `KVM_GET_MSR_INDEX_LIST` already excludes MSRs
    /// the host does not support, and reading an unused MSR simply returns zero — a harmless
    /// no-op in the snapshot. Keeping a fixed allowlist maximizes snapshot portability across
    /// host kernel versions and CPU generations without introducing fragile coupling to the
    /// guest's feature set.
    ///
    pub fn save_state(&self, kvm: &Kvm, fd: &VcpuFd) -> Result<MsrsState> {
        trace!("Saving MSR state");

        // Get the list of MSR indices supported by the host (KVM_GET_MSR_INDEX_LIST).
        let msr_index_list: FamStructWrapper<kvm_msr_list> = match kvm.get_msr_index_list() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting msr_index_list (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // Build a set of host-supported MSR indices for fast lookup.
        let kvm_supported: HashSet<u32> = msr_index_list.as_slice().iter().copied().collect();

        // Filter the compile-time-expanded allowlist to only indices the host supports.
        // EXPANDED_MSR_INDICES already has deferred MSRs at the tail.
        let msr_entries: Vec<kvm_msr_entry> = EXPANDED_MSR_INDICES
            .iter()
            .filter(|idx| kvm_supported.contains(idx))
            .map(|&idx| kvm_msr_entry {
                index: idx,
                data: 0,
                ..Default::default()
            })
            .collect();
        let mut msrs: ::kvm_bindings::Msrs = match ::kvm_bindings::Msrs::from_entries(&msr_entries)
        {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed creating msrs (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        match fd.get_msrs(&mut msrs) {
            Ok(nmsrs_read) => {
                // Sanity check.
                if nmsrs_read != msr_entries.len() {
                    let reason: String = format!(
                        "`nmsrs_read`(={}) is different from `msr_entries.len()`(={})",
                        nmsrs_read,
                        msr_entries.len(),
                    );
                    error!("save_state(): {reason}");
                    anyhow::bail!(reason)
                }
            },
            Err(e) => {
                let reason: String = format!("failed getting msrs (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // Guarantee that a restored vCPU keeps receiving timer interrupts.
        fix_zero_tsc_deadline_msr(&mut msrs);

        Ok(MsrsState {
            bytes: serialize_fam_struct(&msrs),
        })
    }

    ///
    /// # Description
    ///
    /// Restores the MSRs state from a previously saved snapshot.
    ///
    /// # Parameters
    ///
    /// - `fd`: Handle to the virtual CPU.
    /// - `state`: The MSRs state to restore.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function returns empty. Otherwise, it returns an error.
    ///
    pub fn load_state(&self, fd: &VcpuFd, state: &MsrsState) -> Result<()> {
        trace!("load_state()");

        let header_size: usize = ::std::mem::size_of::<::kvm_bindings::kvm_msrs>();
        let entry_size: usize = ::std::mem::size_of::<kvm_msr_entry>();

        if state.bytes.len() < header_size {
            let reason: &str = "msrs data too short for header";
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Read nmsrs from the raw kvm_msrs header (first 4 bytes, native endian).
        let nmsrs: usize = u32::from_ne_bytes([
            state.bytes[0],
            state.bytes[1],
            state.bytes[2],
            state.bytes[3],
        ]) as usize;

        // Reject snapshots whose declared MSR count exceeds the allowlist size. Snapshots produced
        // by `save_state()` can never exceed `EXPANDED_MSR_COUNT`, so a larger value indicates
        // corruption or tampering. This bound also prevents `Vec::with_capacity` below from
        // attempting an attacker-controlled allocation that could OOM-abort the process.
        if nmsrs > EXPANDED_MSR_COUNT {
            let reason: String =
                format!("msrs nmsrs={nmsrs} exceeds allowlist size {EXPANDED_MSR_COUNT}");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        let expected_size: usize = nmsrs
            .checked_mul(entry_size)
            .and_then(|v| v.checked_add(header_size))
            .ok_or_else(|| {
                anyhow::anyhow!("MSR data size computation overflowed (nmsrs={nmsrs})")
            })?;
        if state.bytes.len() < expected_size {
            let reason: String = format!(
                "msrs data size mismatch: expected at least {expected_size}, got {}",
                state.bytes.len()
            );
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Deserialize entries using unaligned reads (the Vec<u8> buffer does not
        // guarantee kvm_msr_entry alignment).
        let entries_bytes: &[u8] = &state.bytes[header_size..header_size + nmsrs * entry_size];
        let mut entries: Vec<kvm_msr_entry> = Vec::with_capacity(nmsrs);
        for chunk in entries_bytes.chunks_exact(entry_size) {
            // SAFETY: `chunk` has length exactly `entry_size` (by `chunks_exact`).
            // We use `read_unaligned` because the Vec<u8> buffer may not satisfy
            // kvm_msr_entry alignment requirements.
            let entry: kvm_msr_entry =
                unsafe { ::std::ptr::read_unaligned(chunk.as_ptr().cast::<kvm_msr_entry>()) };
            entries.push(entry);
        }

        let msrs: ::kvm_bindings::Msrs = match ::kvm_bindings::Msrs::from_entries(&entries) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed creating msrs (error={e:?})");
                error!("load_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        match fd.set_msrs(&msrs) {
            Ok(nmsrs_set) => {
                if nmsrs_set != nmsrs {
                    // Some MSRs from the feature index list may be read-only and cannot be set.
                    // This is expected behavior — we restore as many as the kernel accepts.
                    debug!(
                        "load_state(): partial MSR restore: set {} of {} MSRs",
                        nmsrs_set, nmsrs
                    );
                }
            },
            Err(e) => {
                let reason: String = format!("failed setting msrs (error={e:?})");
                error!("load_state(): {reason}");
                anyhow::bail!(reason)
            },
        }

        Ok(())
    }
}

//==================================================================================================
// Standalone functions
//==================================================================================================

///
/// # Description
///
/// If the `IA32_TSC_DEADLINE` MSR was read back as zero, replaces it with the `IA32_TSC` value.
///
/// When a snapshot is taken, the `IA32_TSC_DEADLINE` MSR is sometimes read as zero even though the
/// APIC timer is armed. Restoring a zero deadline leaves the vCPU without a pending timer interrupt,
/// so it may never receive TSC-deadline interrupts again after resuming. Seeding the deadline with
/// the current `IA32_TSC` value guarantees the timer fires promptly on restore. This mirrors
/// Firecracker's `fix_zero_tsc_deadline_msr`.
///
/// # Parameters
///
/// - `msrs`: The MSR entries read during snapshot save, modified in place.
///
fn fix_zero_tsc_deadline_msr(msrs: &mut ::kvm_bindings::Msrs) {
    const TSC_INDEX: u32 = MsrIndex::Ia32Tsc.as_u32();
    const TSC_DEADLINE_INDEX: u32 = MsrIndex::Ia32TscDeadline.as_u32();

    // A correctly-built snapshot contains at most one IA32_TSC entry. Defensively handle a
    // malformed list with duplicates by picking the maximum, mirroring Firecracker.
    let tsc_value: Option<u64> = msrs
        .as_slice()
        .iter()
        .filter(|msr| msr.index == TSC_INDEX)
        .map(|msr| msr.data)
        .max();

    if let Some(tsc_value) = tsc_value {
        for msr in msrs.as_mut_slice() {
            if msr.index == TSC_DEADLINE_INDEX && msr.data == 0 {
                warn!(
                    "fix_zero_tsc_deadline_msr(): IA32_TSC_DEADLINE is 0, replacing with \
                     {tsc_value:#x}"
                );
                msr.data = tsc_value;
            }
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
