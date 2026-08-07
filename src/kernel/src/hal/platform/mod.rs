// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "microvm")]
mod microvm;
pub mod region_names;
pub mod region_tags;

#[cfg(all(feature = "pit", any(target_arch = "x86", target_arch = "x86_64")))]
pub mod pit;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "microvm")]
pub use microvm::*;

#[cfg(feature = "microvm")]
use microvm::do_shutdown;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod acpi;
pub mod bootinfo;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod madt;
#[cfg(target_arch = "aarch64")]
#[path = "madt_aarch64.rs"]
pub mod madt;

//==================================================================================================
// Shutdown
//==================================================================================================

///
/// # Description
///
/// Shuts down the machine. Flushes the kernel log buffer to ensure all buffered output is emitted,
/// then delegates to the platform-specific shutdown implementation.
///
/// # Parameters
///
/// - `status`: The shutdown status code.
///
/// # Returns
///
/// This function never returns.
///
pub fn shutdown(status: usize) -> ! {
    // SAFETY: the standard output device is present, initialized, and accessed exclusively from a
    // single core with interrupts disabled.
    unsafe { crate::klog::flush() };
    do_shutdown(status);
}

//==================================================================================================
// Interrupts
//==================================================================================================

///
/// # Description
///
/// A structure used to enable of hardware interrupts within a limited scope.
///
pub struct Interrupts;

impl Interrupts {
    ///
    /// # Description
    ///
    /// Enables hardware interrupts and returns an `Interrupts` instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because enabling interrupts can lead to race conditions or undefined
    /// behavior if not used carefully.
    ///
    pub unsafe fn enable() -> Self {
        enable_interrupts();

        Self {}
    }

    /// Waits for a hardware interrupt to occur.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it assumes that interrupts are enabled
    /// and that the system is in a state where waiting for an interrupt is safe.
    pub unsafe fn wait(&self) {
        wait_for_interrupt();
    }
}

impl Drop for Interrupts {
    /// Disables hardware interrupts when the `Interrupts` instance goes out of scope.
    fn drop(&mut self) {
        unsafe {
            disable_interrupts();
        }
    }
}
