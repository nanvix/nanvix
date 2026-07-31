// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_arch = "aarch64")]
use crate::vmm::microvm::whp::arm64::{
    Arm64IcGicV3Parameters,
    Arm64IcParameters,
    PARTITION_PROPERTY_ARM64_IC_PARAMETERS,
};
use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use windows::Win32::System::Hypervisor::{
    WHV_PARTITION_HANDLE,
    WHvCreatePartition,
    WHvDeletePartition,
    WHvPartitionPropertyCodeProcessorCount,
    WHvSetPartitionProperty,
    WHvSetupPartition,
};
#[cfg(target_arch = "x86_64")]
use windows::Win32::System::Hypervisor::{
    WHV_PARTITION_PROPERTY,
    WHvPartitionPropertyCodeLocalApicEmulationMode,
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
            let processor_count: u32 = 1;
            let property_size: u32 = u32::try_from(std::mem::size_of::<u32>())
                .map_err(|_| anyhow::anyhow!("size_of::<u32>() does not fit in u32"))?;
            WHvSetPartitionProperty(
                handle,
                WHvPartitionPropertyCodeProcessorCount,
                (&processor_count as *const u32).cast::<std::ffi::c_void>(),
                property_size,
            )
            .map_err(|e| {
                let reason: String = format!("failed to set processor count (error={e:?})");
                error!("WhpPartition::new(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        #[cfg(target_arch = "x86_64")]
        {
            // Enable LAPIC emulation in XApic mode.
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
                    let reason: String =
                        format!("failed to set LAPIC emulation mode (error={e:?})");
                    error!("WhpPartition::new(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            let interrupt_controller = Arm64IcParameters {
                emulation_mode: 1,
                gic_v3_parameters: Arm64IcGicV3Parameters {
                    gicd_base_address: ::config::microvm::DEFAULT_GICD_BASE as u64,
                    gits_translater_base_address: ::config::microvm::DEFAULT_GITS_BASE as u64,
                    gic_lpi_int_id_bits: 0,
                    gic_ppi_overflow_interrupt_from_cntv:
                        ::config::microvm::DEFAULT_ARM_TIMER_INTERRUPT,
                    gic_ppi_performance_monitors_interrupt: 23,
                    ..Arm64IcGicV3Parameters::default()
                },
                ..Arm64IcParameters::default()
            };
            unsafe {
                WHvSetPartitionProperty(
                    handle,
                    PARTITION_PROPERTY_ARM64_IC_PARAMETERS,
                    (&interrupt_controller as *const Arm64IcParameters).cast::<std::ffi::c_void>(),
                    std::mem::size_of::<Arm64IcParameters>() as u32,
                )
                .map_err(|error| {
                    let reason: String =
                        format!("failed to configure ARM64 GICv3 (error={error:?})");
                    error!("WhpPartition::new(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
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
