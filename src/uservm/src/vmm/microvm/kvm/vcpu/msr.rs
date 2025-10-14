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
use ::serde::{
    Deserialize,
    Serialize,
};
use ::syslog::{
    error,
    trace,
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
                error!("get_state(): {reason}");
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
                error!("get_state(): {reason}");
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
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                }
            },
            Err(e) => {
                let reason: String = format!("failed mutating msrs (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        Ok(MsrsState {
            bytes: match serialize_fam_struct(&msrs) {
                Ok(msrs) => msrs,
                Err(e) => {
                    let reason: String = format!("failed serializing msrs (error={e:?})");
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                },
            },
        })
    }
}
