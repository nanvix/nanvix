// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! System Communication Library
//!
//! This library provides asynchronous socket communication abstractions for both TCP and Unix
//! domain sockets. It offers a unified interface for creating, binding, connecting, and
//! communicating over sockets, abstracting away the differences between socket types.
//!

//==================================================================================================
// Modules
//==================================================================================================

mod socket_address;
mod socket_extensions;
mod socket_listener;
mod socket_stream;
mod socket_stream_reader;
mod socket_stream_writer;
mod socket_type;
mod socket_unbound;

#[cfg(test)]
mod tests;

//==================================================================================================
// Exports
//==================================================================================================

pub use socket_address::*;
pub use socket_extensions::*;
pub use socket_listener::*;
pub use socket_stream::*;
pub use socket_stream_reader::*;
pub use socket_stream_writer::*;
pub use socket_type::*;
pub use socket_unbound::*;

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
