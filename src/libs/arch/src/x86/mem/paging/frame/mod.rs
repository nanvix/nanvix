// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod number;

//==================================================================================================
// Exports
//==================================================================================================

pub use number::FrameNumber;

// Verified spec symbol re-exported so downstream verified crates (e.g. the kernel HAL address
// layer) can reference the frame-number bound. Only present under Verus.
#[cfg(verus_keep_ghost)]
pub use number::spec_max_frame_number;
