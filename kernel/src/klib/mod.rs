// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Exports
//==================================================================================================

pub mod bitmap;
pub mod raw_array;
pub mod slab;

// TODO: review this re-export once system architecture is consolidated.
pub use ::sys::mm::{
    align_down,
    align_up,
    Alignment,
};
