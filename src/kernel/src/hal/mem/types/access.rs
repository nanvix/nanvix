// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![cfg_attr(feature = "microvm", allow(unused))]

//==================================================================================================
// Exports
//==================================================================================================

// TODO: review this re-export once system architecture is consolidated.
pub use ::sys::mm::{
    AccessPermission,
    ExecutePermission,
    ReadPermission,
    WritePermission,
};
