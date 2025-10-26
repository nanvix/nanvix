// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Nanvix Daemon (nanvixd) library.
//!
//! This library provides the core functionality for the Nanvix Daemon, which manages
//! sandboxed user VM instances and Linux Daemon instances. It includes HTTP server
//! capabilities for client communication, sandbox management, and configuration handling.

//==================================================================================================
// Modules
//==================================================================================================

pub mod args;
pub mod config;
pub mod http;
pub mod message;
pub mod terminal;
