// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod mmio;
mod pmio;

//==================================================================================================
// Exports
//==================================================================================================

pub use mmio::{
    IoMemoryAllocator,
    IoMemoryRegion,
};
pub use pmio::{
    AnyIoPort,
    IoPortAllocator,
    IoPortType,
    IoPortWidth,
    ReadOnlyIoPort,
    ReadWriteIoPort,
};
