// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "single-process"))]
mod multi_process;
#[cfg(feature = "single-process")]
mod single_process;

pub mod config;
pub mod tag;
pub mod tcp_port;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(not(feature = "single-process"))]
pub use self::multi_process::*;
#[cfg(feature = "single-process")]
pub use self::single_process::*;
