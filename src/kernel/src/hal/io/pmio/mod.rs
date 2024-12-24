// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod allocator;
mod info;
mod ioport;
mod typ;
mod width;

//==================================================================================================
// Exports
//==================================================================================================

pub use allocator::{
    AnyIoPort,
    IoPortAllocator,
    ReadOnlyIoPort,
    ReadWriteIoPort,
};
pub use typ::IoPortType;
pub use width::IoPortWidth;
