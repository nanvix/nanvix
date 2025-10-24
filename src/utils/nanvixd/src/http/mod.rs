// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HTTP server and client implementation for Nanvix Daemon.
//!
//! This module provides HTTP-based communication between external clients and the Nanvix Daemon.
//! It includes server functionality for handling incoming requests and client handlers for
//! processing specific operations like creating and killing sandboxes.

//==================================================================================================
// Modules
//==================================================================================================

mod client;
mod server;

//==================================================================================================
// Exports
//==================================================================================================

pub use server::HttpServer;
