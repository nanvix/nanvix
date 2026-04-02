// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    info,
    trace,
    warn,
};
use windows::Win32::System::Hypervisor::{
    WHV_CAPABILITY,
    WHV_PARTITION_HANDLE,
    WHV_PARTITION_PROPERTY,
    WHV_X64_CPUID_RESULT,
    WHvCapabilityCodeProcessorClockFrequency,
    WHvCreatePartition,
    WHvDeletePartition,
    WHvGetCapability,
    WHvPartitionPropertyCodeCpuidResultList,
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

        // Override CPUID leaf 0x16 (Processor Frequency Information) so
        // the guest kernel can use RDTSC-based LAPIC timer calibration
        // instead of the PIT busy-wait loop (~908 VM exits eliminated).
        // Hyper-V zeros out leaf 0x16 even when the host CPU supports it.
        unsafe {
            let freq_hz: u64 = Self::query_processor_clock_frequency();
            let freq_mhz: u32 = (freq_hz / 1_000_000) as u32;
            if freq_mhz > 0 {
                let cpuid_entry = WHV_X64_CPUID_RESULT {
                    Function: 0x16,
                    Reserved: [0; 3],
                    Eax: freq_mhz,
                    Ebx: freq_mhz,
                    Ecx: 0,
                    Edx: 0,
                };
                let entry_size: u32 = u32::try_from(std::mem::size_of::<WHV_X64_CPUID_RESULT>())
                    .map_err(|_| anyhow::anyhow!("CPUID result size overflow"))?;
                WHvSetPartitionProperty(
                    handle,
                    WHvPartitionPropertyCodeCpuidResultList,
                    (&cpuid_entry as *const WHV_X64_CPUID_RESULT).cast::<std::ffi::c_void>(),
                    entry_size,
                )
                .map_err(|e| {
                    let reason: String = format!("failed to set CPUID result list (error={e:?})");
                    error!("WhpPartition::new(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
                info!("overriding CPUID leaf 0x16: base_freq={}MHz (from {}Hz)", freq_mhz, freq_hz);
            } else {
                warn!(
                    "could not query processor clock frequency; guest will use PIT-based \
                     calibration"
                );
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

    /// Queries the host processor TSC clock frequency via WHP.
    /// Returns 0 if the query fails or the frequency is unavailable.
    ///
    /// # Safety
    ///
    /// Calls WHP FFI (`WHvGetCapability`). Safe to call at any time;
    /// the capability query does not require a valid partition handle.
    pub unsafe fn query_processor_clock_frequency() -> u64 {
        let mut cap: WHV_CAPABILITY = unsafe { std::mem::zeroed() };
        let cap_size: u32 = match u32::try_from(std::mem::size_of::<WHV_CAPABILITY>()) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        let result = unsafe {
            WHvGetCapability(
                WHvCapabilityCodeProcessorClockFrequency,
                (&mut cap as *mut WHV_CAPABILITY).cast::<std::ffi::c_void>(),
                cap_size,
                None,
            )
        };
        match result {
            Ok(()) => unsafe { cap.ProcessorClockFrequency },
            Err(e) => {
                warn!("WHvGetCapability(ProcessorClockFrequency) failed: {e:?}");
                0
            },
        }
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
