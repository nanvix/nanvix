// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use windows::Win32::System::Hypervisor::{
    WHV_PARTITION_HANDLE,
    WHV_PARTITION_PROPERTY,
    WHvCreatePartition,
    WHvDeletePartition,
    WHvPartitionPropertyCodeLocalApicEmulationMode,
    WHvPartitionPropertyCodeProcessorCount,
    WHvSetPartitionProperty,
    WHvSetupPartition,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A wrapper around a Windows Hypervisor Platform partition handle.
///
pub struct WhpPartition {
    /// Handle to the WHP partition.
    handle: WHV_PARTITION_HANDLE,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl WhpPartition {
    ///
    /// # Description
    ///
    /// Creates a new WHP partition with one virtual processor.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function returns the new partition. Otherwise, it returns
    /// an error.
    ///
    pub fn new() -> Result<Self> {
        trace!("WhpPartition::new()");

        // Create the partition.
        let handle: WHV_PARTITION_HANDLE = unsafe {
            WHvCreatePartition().map_err(|e| {
                let reason: String = format!("failed to create WHP partition (error={e:?})");
                error!("WhpPartition::new(): {reason}");
                anyhow::anyhow!(reason)
            })?
        };

        // Set the processor count to 1.
        unsafe {
            let mut property: WHV_PARTITION_PROPERTY = std::mem::zeroed();
            property.ProcessorCount = 1;
            let property_size: u32 = u32::try_from(std::mem::size_of::<u32>())
                .map_err(|_| anyhow::anyhow!("size_of::<u32>() does not fit in u32"))?;
            WHvSetPartitionProperty(
                handle,
                WHvPartitionPropertyCodeProcessorCount,
                (&property as *const WHV_PARTITION_PROPERTY).cast::<std::ffi::c_void>(),
                property_size,
            )
            .map_err(|e| {
                let reason: String = format!("failed to set processor count (error={e:?})");
                error!("WhpPartition::new(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Enable LAPIC emulation in XApic mode. This is required so:
        //   - The kernel's LAPIC MMIO accesses work during boot.
        //   - HLT is handled internally by the LAPIC emulator
        //     (no VM exit), which is fine because the guest signals
        //     its idle state via the PV idle port (0xED) before HLT.
        unsafe {
            let mut property: WHV_PARTITION_PROPERTY = std::mem::zeroed();
            property.LocalApicEmulationMode =
                windows::Win32::System::Hypervisor::WHV_X64_LOCAL_APIC_EMULATION_MODE(1);
            let property_size: u32 = u32::try_from(std::mem::size_of::<
                windows::Win32::System::Hypervisor::WHV_X64_LOCAL_APIC_EMULATION_MODE,
            >())
            .map_err(|_| anyhow::anyhow!("emulation mode size does not fit in u32"))?;
            WHvSetPartitionProperty(
                handle,
                WHvPartitionPropertyCodeLocalApicEmulationMode,
                (&property as *const WHV_PARTITION_PROPERTY).cast::<std::ffi::c_void>(),
                property_size,
            )
            .map_err(|e| {
                let reason: String = format!("failed to set LAPIC emulation mode (error={e:?})");
                error!("WhpPartition::new(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Setup the partition (finalizes configuration).
        unsafe {
            WHvSetupPartition(handle).map_err(|e| {
                let reason: String = format!("failed to setup WHP partition (error={e:?})");
                error!("WhpPartition::new(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        Ok(Self { handle })
    }

    ///
    /// # Description
    ///
    /// Returns the raw WHP partition handle.
    ///
    pub fn handle(&self) -> WHV_PARTITION_HANDLE {
        self.handle
    }
}

impl Drop for WhpPartition {
    fn drop(&mut self) {
        trace!("WhpPartition::drop()");
        unsafe {
            if let Err(e) = WHvDeletePartition(self.handle) {
                error!("WhpPartition::drop(): failed to delete partition (error={e:?})");
            }
        }
    }
}

// SAFETY: `WhpPartition` wraps an opaque `WHV_PARTITION_HANDLE` managed by the Windows
// Hypervisor Platform. The WHP APIs are designed to allow a partition handle to be used from any
// thread; all internal synchronisation is performed by the Windows kernel/hypervisor.
// `WhpPartition` does not add any unsynchronised interior mutability beyond the underlying OS
// object; moving it between threads or sharing references cannot create data races.
unsafe impl Send for WhpPartition {}
unsafe impl Sync for WhpPartition {}
