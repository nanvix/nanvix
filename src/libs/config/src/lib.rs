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

// Linuxd build-time constants are generated in a similar fashion to kernel variables.
include!(concat!(env!("OUT_DIR"), "/linuxd_config.rs"));

//==================================================================================================
// System
//==================================================================================================

pub mod system {
    /// Default system name.
    pub const DEFAULT_SYSTEM_NAME: &str = "nanvix";

    cfg_if::cfg_if! {
        if #[cfg(feature = "microvm")] {
            /// Default machine name.
            pub const DEFAULT_MACHINE_NAME: &str = "microvm";
        } else if #[cfg(feature = "pc")] {
            /// Default machine name.
            pub const DEFAULT_MACHINE_NAME: &str = "pc";
        } else if #[cfg(feature = "hyperlight")] {
            /// Default machine name.
            pub const DEFAULT_MACHINE_NAME: &str = "hyperlight";
        } else {
            /// Default machine name.
            pub const DEFAULT_MACHINE_NAME: &str = "unknown";
        }
    }

    /// Default node name.
    pub const DEFAULT_NODE_NAME: &str = "localhost";
}

//==================================================================================================
// User Memory Layout
//==================================================================================================

pub mod memory_layout {

    ///
    /// # Description
    ///
    /// Provides the raw value for [`KERNEL_BASE`], which can be used in constant-value expressions.
    ///
    pub const KERNEL_BASE_RAW: usize = 0x0000_0000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`KPOOL_END`], which can be used in constant-value expressions.
    ///
    pub const KERNEL_END_RAW: usize = 0x4000_0000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`KPOOL_BASE`], which can be used in constant-value expressions.
    ///
    pub const KPOOL_BASE_RAW: usize = 0x00400000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`KPOOL_END`], which can be used in constant-value expressions.
    ///
    pub const USER_BASE_RAW: usize = KERNEL_END_RAW;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`USER_END`], which can be used in constant-value expressions.
    ///
    pub const USER_END_RAW: usize = 0xf0000000;

    ///
    /// # Description
    ///
    /// Base address of user stack.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_STACK_BASE_RAW: usize = USER_END_RAW;

    ///
    /// # Description
    ///
    /// End address of the user stack
    ///
    pub const USER_STACK_TOP_RAW: usize =
        USER_STACK_BASE_RAW - USER_STACK_SIZE * NUM_USER_STACK_ENTRIES;

    ///
    /// # Description
    ///
    /// Size of the user stack.
    ///
    /// # Notes:
    ///
    /// - This size should be a multiple of a page size.
    ///
    pub const USER_STACK_SIZE: usize = 512 * crate::constants::KILOBYTE;

    ///
    /// # Description
    ///
    /// Number of entries in the user stack. This should be a multiple of 8.
    ///
    pub const NUM_USER_STACK_ENTRIES: usize = 8;

    ///
    /// # Description
    ///
    /// Base address for memory-mapped objects.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_MMAP_BASE_RAW: usize = 0x6000_0000;

    /// # Description
    ///
    /// End address for memory-mapped objects.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_MMAP_END_RAW: usize = 0xa000_0000;

    ///
    /// # Description
    ///
    /// Base address for shared libraries.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_LIBS_BASE_RAW: usize = USER_MMAP_END_RAW;

    ///
    /// # Description
    ///
    /// End address for shared libraries.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_LIBS_END_RAW: usize = 0xb0000000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`USER_HEAP_BASE`], which can be used in constant-value expressions.
    ///
    pub const USER_HEAP_BASE_RAW: usize = USER_LIBS_END_RAW;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`USER_HEAP_END`], which can be used in constant-value expressions.
    ///
    pub const USER_HEAP_END_RAW: usize = USER_HEAP_BASE_RAW + USER_HEAP_SIZE;

    ///
    /// # Description
    ///
    /// Size of the user heap.
    ///
    pub const USER_HEAP_SIZE: usize = 32 * crate::constants::MEGABYTE;
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

    /// Default VMM pause command. This MUST be a value that's never an exit code.
    pub const DEFAULT_VMM_PAUSE_CMD: u16 = 0x3000;

    /// Default base address for MicroVM control registers.
    pub const DEFAULT_MICROVM_CTRL_BASE: usize = 0x00000000;

    /// Default base address for MicroVM null register. (32-bit wide read-only register)
    pub const DEFAULT_MICROVM_CTRL_NULL: usize = 0x00000000;

    /// Default base address for MicroVM credits register (32-bit wide read-only register)
    pub const DEFAULT_MICROVM_CTRL_CREDITS: usize = 0x00000004;

    /// Default base address for MicroVM pause-requested register (32-bit wide read-only register)
    pub const DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED: usize = 0x00000008;

    /// Default base address for RAMFS base register (32-bit wide read-only register)
    pub const DEFAULT_MICROVM_CTRL_RAMFS_BASE: usize = 0x0000000c;

    /// Default base address for RAMFS size register (32-bit wide read-only register)
    pub const DEFAULT_MICROVM_CTRL_RAMFS_SIZE: usize = 0x00000010;

    /// Magic value that identifies the running state in the pause-requested register.
    pub const RUNNING: u32 = 0x00000000;

    /// Magic value that flags that the VMM requested the guest OS to pause MicroVM execution.
    pub const PAUSE_REQUEST: u32 = 0x00000001;
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
    /// Base address of the RAM disk.
    pub const DEFAULT_INITRD_BASE: usize = 0x00802000;
    /// Number of bytes used to store initrd's size.
    pub const INITRD_SIZE_BYTES: usize = 8;
    /// Default VMM shutdown command
    pub const DEFAULT_VMM_SHUTDOWN_CMD: u8 = 0x20;
}
