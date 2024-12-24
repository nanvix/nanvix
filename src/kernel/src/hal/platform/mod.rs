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

pub mod acpi;
pub mod bootinfo;
pub mod madt;
