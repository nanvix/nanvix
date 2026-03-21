// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::VirtualMemory;
use ::anyhow::Result;
use ::log::error;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Information about the guest running inside the virtual machine.
///
#[derive(Default)]
pub struct Guest;

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
        vmem.counter
            .increment()
            .map_err(|e| {
                error!("add_credit(): failed to increment counter: {e:?}");
                anyhow::anyhow!("{e:?}")
            })
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
        vmem.counter
            .decrement()
            .map_err(|e| {
                error!("consume_credit(): failed to decrement counter: {e:?}");
                anyhow::anyhow!("{e:?}")
            })
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
