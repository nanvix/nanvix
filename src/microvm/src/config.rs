// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Configuration
//!
//! This module provides various configuration parameters.
//!

/// Default name of the program.
pub const PROGRAM_NAME: &str = "microvm";

/// Default memory size.
pub const DEFAULT_MEMORY_SIZE: usize = 128 * 1024 * 1024;

/// Magic value that identifies the virtual machine monitor.
pub const MICROVM_MAGIC: u32 = 0x0c00ffee;

/// Base address of the RAM disk.
pub const INITRD_BASE: usize = 0x00800000;

/// I/O port that is connected to the standard output of the virtual machine.
pub const STDOUT_PORT: u16 = 0xe9;

/// I/O port that is connected to the standard input of the virtual machine.
pub const STDIN_PORT: u16 = 0xea;

/// I/O port that enables the guest to invoke functionalities of the virtual machine monitor.
pub const VMM_PORT: u16 = 0x604;
