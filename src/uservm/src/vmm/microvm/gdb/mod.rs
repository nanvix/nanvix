// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! GDB Remote Serial Protocol server for the microvm machine type.
//!
//! When the `gdb` feature is enabled and `-gdb-port <port>` is supplied on the command line, this
//! module provides a TCP-based GDB stub that allows a remote GDB client to debug the Nanvix guest.
//! The stub runs synchronously in the vCPU thread and supports:
//!
//! - Reading and writing general-purpose and segment registers.
//! - Reading and writing guest physical memory.
//! - Software breakpoints (via `INT3` patching).
//! - Single-stepping.
//! - Continue (resume execution).

mod event_loop;
mod target;

pub(crate) use event_loop::run_gdb_server;
