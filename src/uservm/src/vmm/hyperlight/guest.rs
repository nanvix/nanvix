// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    VMEM,
    VirtualMemory,
};
use ::anyhow::Result;
use ::hyperlight_host::mem::{
    mgr::SandboxMemoryManager,
    shared_mem::ExclusiveSharedMemory,
};
use ::syslog::info;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default)]
pub struct Guest;

//==================================================================================================
// Implementations
//==================================================================================================

impl Guest {
    // Adds a credit to the virtual machine's credit pool.
    pub fn add_credit(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        let manager: &mut SandboxMemoryManager<ExclusiveSharedMemory> = &mut vmem.manager;
        let credits_offset: usize = manager.get_guest_credits_offset();
        let mut credit: u64 = manager.get_shared_mem_mut().read::<u64>(credits_offset)?;
        credit += 1_u64;
        manager
            .get_shared_mem_mut()
            .write::<u64>(credits_offset, credit)?;

        info!("Adding credit: {}", credit);
        Ok(())
    }

    // Consumes a credit from the virtual machine's credit pool.
    pub fn consume_credit() -> Result<()> {
        VMEM.get()
            .map(|vmem| vmem.blocking_lock())
            .map(|mut vmem| -> Result<()> {
                let credits_offset: usize = vmem.get_guest_credits_offset();
                let mut credit: u64 = vmem.get_shared_mem_mut().read::<u64>(credits_offset)?;

                if credit == 0_u64 {
                    return Err(anyhow::anyhow!("No credit available to consume"));
                }

                credit -= 1_u64;
                vmem.get_shared_mem_mut()
                    .write::<u64>(credits_offset, credit)?;

                info!("Consuming credit: {}", credit);
                Ok(())
            })
            .ok_or(anyhow::anyhow!("VMEM is not initialized"))?
    }

    ///
    /// # Description
    ///
    /// Requests the kernel to pause the virtual machine's execution by writing to a specific register.
    ///
    /// # Returns
    ///
    /// On success, returns empty. Otherwise, returns an error.
    ///
    #[allow(unused_variables)]
    pub fn pause_vm(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        Ok(()) // TODO: https://github.com/nanvix/nanvix/issues/791
    }

    ///
    /// # Description
    ///
    /// Writes to a specific kernel register that execution should not be paused.
    ///
    /// # Returns
    ///
    /// On success, returns empty. Otherwise, returns an error.
    ///
    #[allow(unused_variables)]
    pub fn resume_vm(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        Ok(()) // TODO: https://github.com/nanvix/nanvix/issues/791
    }
}
