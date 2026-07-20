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

pub use ::nanvix_sandbox_config::StandaloneConfig;
pub use client::StandaloneState;
pub use server::HttpServer;
