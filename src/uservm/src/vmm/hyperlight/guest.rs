// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::VirtualMemory;
use ::anyhow::Result;
use ::hyperlight_host::mem::{
    mgr::SandboxMemoryManager,
    shared_mem::ExclusiveSharedMemory,
};
use ::syslog::error;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Information about the guest running inside the virtual machine.
///
#[derive(Default)]
pub struct Guest {
    /// Credits counter, account the number of messages that the VM can consume.
    credits: u64,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Guest {
    ///
    /// # Description
    ///
    /// Increments the credit register by one.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory manager of the virtual machine.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple. Otherwise, it returns an error.
    ///
    pub fn add_credit(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        let manager: &mut SandboxMemoryManager<ExclusiveSharedMemory> = &mut vmem.manager;

        // Increment credits, checking for overflow.
        self.credits = self.credits.checked_add(1).ok_or_else(|| {
            let reason: &'static str = "credit overflow";
            error!("add_credit(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let credits_offset: usize = manager.get_guest_credits_offset();
        manager
            .get_shared_mem_mut()
            .write::<u64>(credits_offset, self.credits)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Decrements the credit register by one.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory manager of the virtual machine.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple. Otherwise, it returns an error.
    ///
    pub fn consume_credit(&mut self, vmem: &mut VirtualMemory) -> Result<()> {
        let manager: &mut SandboxMemoryManager<ExclusiveSharedMemory> = &mut vmem.manager;

        // Decrement credits, checking for underflow.
        self.credits = self.credits.checked_sub(1).ok_or_else(|| {
            let reason: &'static str = "credit underflow";
            error!("consume_credit(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let credits_offset: usize = manager.get_guest_credits_offset();
        manager
            .get_shared_mem_mut()
            .write::<u64>(credits_offset, self.credits)?;

        Ok(())
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
