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
pub mod region_names;
pub mod region_tags;

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

#[cfg(not(feature = "x86_64"))]
pub mod acpi;
pub mod bootinfo;
#[cfg(not(feature = "x86_64"))]
pub mod madt;

// On x86_64, provide stub types for MadtInfo so that code that references it can still compile.
#[cfg(feature = "x86_64")]
pub mod madt {
    use ::alloc::collections::LinkedList;

    pub struct MadtInfo {
        pub entries: LinkedList<MadtEntry>,
    }

    pub enum MadtEntry {}

    impl MadtInfo {
        pub fn cores_count(&self) -> usize {
            1
        }

        pub const fn has_8259_pic(&self) -> bool {
            false
        }
    }

    impl Iterator for MadtInfo {
        type Item = MadtEntry;
        fn next(&mut self) -> Option<Self::Item> {
            self.entries.pop_front()
        }
    }
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
