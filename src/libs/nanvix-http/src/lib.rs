// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

mod client;
mod server;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "standalone")]
pub use client::{
    StandaloneConfig,
    StandaloneState,
};
pub use server::HttpServer;
