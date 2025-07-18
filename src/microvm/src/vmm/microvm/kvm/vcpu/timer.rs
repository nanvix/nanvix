// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::kvm::partition::VirtualPartition;
use ::anyhow::Result;
use ::kvm_bindings::{
    KVM_PIT_SPEAKER_DUMMY,
    kvm_pit_config,
};
use ::std::sync::{
    Arc,
    Mutex,
    MutexGuard,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Timer;

//==================================================================================================
// Implementations
//==================================================================================================

impl Timer {
    ///
    /// # Description
    ///
    /// Creates a programmable interrupt timer and attaches it to a virtual partition.
    ///
    /// # Parameters
    ///
    /// - `partition`: Handle to the virtual partition.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn new(partition: &Arc<Mutex<VirtualPartition>>) -> Result<Timer> {
        trace!("setup_pit()");

        let lock: MutexGuard<'_, VirtualPartition> = partition
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?;

        // Enable the emulation of a dummy speaker port stub so that writing to port 0x61
        // does not cause a KVM_EXIT event.
        let pit_config: kvm_pit_config = kvm_pit_config {
            flags: KVM_PIT_SPEAKER_DUMMY,
            ..Default::default()
        };

        lock.vm().create_pit2(pit_config)?;
        Ok(Self)
    }
}
