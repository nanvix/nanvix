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
mod hyperlight;
#[cfg(feature = "microvm")]
mod microvm;

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
