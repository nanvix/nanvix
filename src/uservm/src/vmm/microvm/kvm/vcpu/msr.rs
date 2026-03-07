// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::kvm::vcpu::serialize_fam_struct;
use ::anyhow::Result;
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
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::vmm_sys_util::fam::FamStructWrapper;

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
    bytes: Vec<u8>,
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
    pub fn save_state(&self, kvm: &Kvm, fd: &VcpuFd) -> Result<MsrsState> {
        trace!("Saving MSR state");

        // Build `Msrs` out of entries.
        let msr_index_list: FamStructWrapper<kvm_msr_list> = match kvm.get_msr_feature_index_list()
        {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting msr_index_list (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let msr_entries: Vec<kvm_msr_entry> = msr_index_list
            .as_slice()
            .iter()
            .map(|idx| kvm_msr_entry {
                index: *idx,
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

        let expected_size: usize = header_size + nmsrs * entry_size;
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
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::anyhow::Result as AnyResult;
    use ::kvm_bindings::KVM_MAX_CPUID_ENTRIES;
    use ::kvm_ioctls::VmFd;

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
        assert!(
            state.bytes.len() >= header_size,
            "MSR snapshot too short (len={})",
            state.bytes.len()
        );

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
}
