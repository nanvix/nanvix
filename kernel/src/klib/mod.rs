// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Modules
//==================================================================================================

mod constants;

//==================================================================================================
// Exports
//==================================================================================================

pub mod bitmap;
pub mod raw_array;
pub mod slab;
pub use constants::{
    KILOBYTE,
    MEGABYTE,
};

// TODO: review this re-export once system architecture is consolidated.
pub use ::sys::mm::{
    align_down,
    align_up,
    is_aligned,
    Alignment,
};
