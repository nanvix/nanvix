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
pub use ::nanvix_sandbox_config::StandaloneConfig;
#[cfg(feature = "standalone")]
pub use client::StandaloneState;
pub use server::HttpServer;
