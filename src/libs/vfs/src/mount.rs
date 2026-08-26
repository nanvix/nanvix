// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Mount table and path resolution.

//==================================================================================================
// Modules
//==================================================================================================

mod mount_point;
mod path_cache;
mod vfs;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub(crate) use self::vfs::{
    anchor_path,
    normalize_absolute,
    normalize_anchored,
};
pub use self::{
    mount_point::Mount,
    vfs::Vfs,
};
