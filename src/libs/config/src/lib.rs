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
// Platform-Specific Constants
//==================================================================================================

pub mod platform {
    cfg_if::cfg_if! {
        if #[cfg(feature = "pc")] {
            /// Number of extra boot page tables for memory-mapped I/O regions above physical
            /// memory. On PC platforms, the LAPIC and IOAPIC are above physical memory and share
            /// a single 4 MB page table block.
            pub const NUM_MMIO_BOOT_PAGE_TABLES: usize = 1;
        } else {
            /// Number of extra boot page tables for memory-mapped I/O regions above physical
            /// memory. On microvm and hyperlight, all MMIO is within physical memory, so no
            /// extra page tables are needed.
            pub const NUM_MMIO_BOOT_PAGE_TABLES: usize = 0;
        }
    }
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
    pub const KPOOL_BASE_RAW: usize = crate::kernel::KPOOL_BASE_RAW;

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
    /// Minimum number of stack bytes mapped at process creation.
    /// Additional pages up to [`USER_STACK_SIZE`] are demand-paged on stack growth faults.
    ///
    /// # Notes:
    ///
    /// - This size should be a multiple of a page size.
    ///
    pub const USER_STACK_MIN_SIZE: usize = 32 * crate::constants::KILOBYTE;

    ///
    /// # Description
    ///
    /// Number of entries in the user stack. This should be a multiple of 8.
    ///
    pub const NUM_USER_STACK_ENTRIES: usize = 8;

    ///
    /// # Description
    ///
    /// Base address for the unified mmap region.
    ///
    /// All dynamic memory allocations (heap, shared libraries, and explicit memory mappings) are
    /// backed by this unified region.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_MMAP_BASE_RAW: usize = 0x6000_0000;

    ///
    /// # Description
    ///
    /// End address for the unified mmap region.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_MMAP_END_RAW: usize = 0xd000_0000;

    ///
    /// # Description
    ///
    /// Size of the unified mmap region in bytes.
    ///
    pub const USER_MMAP_SIZE: usize = USER_MMAP_END_RAW - USER_MMAP_BASE_RAW;

    ///
    /// # Description
    ///
    /// Maximum capacity of the user heap in bytes. The heap is backed by the unified mmap region
    /// and grows lazily on demand.
    ///
    pub const USER_HEAP_CAPACITY: usize = 32 * crate::constants::MEGABYTE;
}

//==================================================================================================
// Hardware Abstraction Layer
//==================================================================================================

#[cfg(feature = "microvm")]
pub mod microvm {
    /// Magic value that identifies the virtual machine monitor.
    pub const DEFAULT_BOOT_MAGIC: u32 = 0x0c00ffee;

    /// Base address of the RAM disk.
    pub const DEFAULT_INITRD_BASE: usize = 0x00c00000;

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

    /// Default VMM snapshot command. Triggers a guest-initiated VM snapshot.
    pub const DEFAULT_VMM_SNAPSHOT_CMD: u16 = 0x4000;

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

    /// Guest physical address of the pvclock page (page-aligned, 4KB).
    /// KVM populates this page with the `KvmPvclockVcpuTimeInfo` structure
    /// when the `MSR_KVM_SYSTEM_TIME_NEW` MSR is enabled.
    pub const DEFAULT_PVCLOCK_PAGE: usize = 0x00001000;

    /// Offset within the pvclock page for the boot time in nanoseconds since
    /// the Unix epoch (u64). The VMM writes this value during VM initialization.
    pub const PVCLOCK_BOOT_TIME_NS_OFFSET: usize = 0x20;
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

// Hyperlight build-time constants are generated from hyperlight_config.toml.
#[cfg(feature = "hyperlight")]
include!(concat!(env!("OUT_DIR"), "/hyperlight_config.rs"));
