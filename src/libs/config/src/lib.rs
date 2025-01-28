// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod constants;

//==================================================================================================
// Kernel
//==================================================================================================

// Note: Kernel constants are generated from kernel_config.toml via a build.rs file.
// This is to allow these values to be easily used outside Rust code (e.g., in a Shell script).
include!(concat!(env!("OUT_DIR"), "/kernel_config.rs"));

//==================================================================================================
// User Memory Layout
//==================================================================================================

pub mod memory_layout {
    ///
    /// # Description
    ///
    /// Provides the raw value for [`KPOOL_BASE`], which can be used in constant-value expressions.
    ///
    pub const KPOLL_BASE_RAW: usize = 0x00400000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`KPOOL_END`], which can be used in constant-value expressions.
    ///
    pub const USER_BASE_RAW: usize = 0x40000000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`USER_END`], which can be used in constant-value expressions.
    ///
    pub const USER_END_RAW: usize = 0xf0000000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`USER_HEAP_BASE`], which can be used in constant-value expressions.
    ///
    pub const USER_HEAP_BASE_RAW: usize = 0xa0000000;
}

//==================================================================================================
// Hardware Abstraction Layer
//==================================================================================================

#[cfg(feature = "microvm")]
pub mod microvm {
    /// Magic value that identifies the virtual machine monitor.
    pub const DEFAULT_BOOT_MAGIC: u32 = 0x0c00ffee;

    /// Base address of the RAM disk.
    pub const DEFAULT_INITRD_BASE: usize = 0x00800000;

    /// I/O port that is connected to the standard output of the virtual machine.
    pub const DEFAULT_STDOUT_PORT: u16 = 0xe9;

    /// I/O port that is connected to the standard input of the virtual machine.
    pub const DEFAULT_STDIN_PORT: u16 = 0xea;

    /// I/O port that enables the guest to invoke functionalities of the virtual machine monitor.
    pub const DEFAULT_VMM_PORT: u16 = 0x604;

    /// Default VMM shutdown command
    pub const DEFAULT_VMM_SHUTDOWN_CMD: u16 = 0x2000;

    /// Default base address for MicroVM control registers.
    pub const DEFAULT_MICROVM_CTRL_BASE: usize = 0x00000000;

    /// Default base address for MicroVM null register. (32-bit wide read-only register)
    pub const DEFAULT_MICROVM_CTRL_NULL: usize = 0x00000000;

    /// Default base address for MicroVM credits register (32-bit wide read-only register)
    pub const DEFAULT_MICROVM_CTRL_CREDITS: usize = 0x00000004;
}

#[cfg(feature = "pc")]
pub mod pc {
    /// I/O port that is connected to the standard output of the virtual machine.
    pub const DEFAULT_STDOUT_PORT: u16 = 0xe9;

    /// I/O port that enables the guest to invoke functionalities of the virtual machine monitor.
    pub const DEFAULT_VMM_PORT: u16 = 0x604;

    /// Default VMM shutdown command
    pub const DEFAULT_VMM_SHUTDOWN_CMD: u16 = 0x2000;
}

#[cfg(feature = "hyperlight")]
pub mod hyperlight {
    /// Magic value that identifies the virtual machine monitor.
    pub const DEFAULT_BOOT_MAGIC: u32 = 0x0c00ffee;
}
