// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(any(
    feature = "qemu-pc",
    feature = "qemu-isapc",
    feature = "qemu-baremetal"
))]
mod pc;

#[cfg(feature = "hyperlight")]
pub(crate) mod hyperlight;
#[cfg(feature = "microvm")]
mod microvm;

#[cfg(feature = "pit")]
pub mod pit;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(any(
    feature = "qemu-pc",
    feature = "qemu-isapc",
    feature = "qemu-baremetal"
))]
pub use pc::*;

#[cfg(feature = "microvm")]
pub use microvm::*;

#[cfg(feature = "hyperlight")]
pub use hyperlight::*;

pub mod acpi;
pub mod bootinfo;
pub mod madt;

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
