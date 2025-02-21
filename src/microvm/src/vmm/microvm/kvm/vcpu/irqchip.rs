// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::kvm::partition::VirtualPartition;
use ::anyhow::Result;
use ::std::sync::{
    Arc,
    Mutex,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct IrqChip;

//==================================================================================================
// Implementations
//==================================================================================================

impl IrqChip {
    ///
    /// # Description
    ///
    /// Creates a interrupt controller and attaches it to a virtual partition.
    ///
    /// # Parameters
    ///
    /// - `partition`: Handle to the virtual partition.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn new(partition: &Arc<Mutex<VirtualPartition>>) -> Result<Self> {
        trace!("new()");

        partition
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?
            .vm()
            .create_irq_chip()?;

        Ok(Self)
    }
}
