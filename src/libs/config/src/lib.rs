// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod constants;
pub mod daemons;
pub mod fds;
pub mod region_tags;

//==================================================================================================
// Kernel
//==================================================================================================

// Note: Kernel constants are generated from kernel_config.toml via a build.rs file.
// This is to allow these values to be easily used outside Rust code (e.g., in a Shell script).
include!(concat!(env!("OUT_DIR"), "/kernel_config.rs"));

// Linuxd build-time constants are generated in a similar fashion to kernel variables.
include!(concat!(env!("OUT_DIR"), "/linuxd_config.rs"));

// Compile-time assertions on kernel build-time constants.
//
// Mirrored from `build.rs` so misuse of the generated constants is caught at the use site too.
static_assert::assert_eq!(kernel::MAX_PROCESSES >= 1);
static_assert::assert_eq!(kernel::MAX_PROCESSES <= u8::MAX as usize);

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
        } else {
            /// Default machine name.
            pub const DEFAULT_MACHINE_NAME: &str = "unknown";
        }
    }

    /// Default node name.
    pub const DEFAULT_NODE_NAME: &str = "localhost";

    /// Maximum length (in bytes) of guest command-line arguments.
    ///
    /// This value must not exceed `PAGE_SIZE - 1` (4095 on i686) because the kernel
    /// allocates a single page for the argument string plus its null terminator.
    /// We use 4092 to leave a small margin.
    pub const MAX_CMDLINE_ARGS_LEN: usize = 4092;

    ///
    /// # Description
    ///
    /// Strongly-typed length of guest command-line arguments.
    ///
    /// This wrapper guarantees that the value fits in a `u16` and does not exceed
    /// [`MAX_CMDLINE_ARGS_LEN`]. It is used as the wire type in the guest memory protocol.
    ///
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CmdlineArgsLen(u16);

    impl CmdlineArgsLen {
        /// Wire size of the length field in guest memory (2 bytes, little-endian).
        pub const WIRE_SIZE: usize = core::mem::size_of::<u16>();

        /// Creates a new [`CmdlineArgsLen`] from the byte length of a command-line string.
        ///
        /// # Returns
        ///
        /// Returns `Some(Self)` if `len <= MAX_CMDLINE_ARGS_LEN`, otherwise `None`.
        pub const fn new(len: usize) -> Option<Self> {
            if len > MAX_CMDLINE_ARGS_LEN {
                None
            } else {
                Some(Self(len as u16))
            }
        }

        /// Returns the length as a `usize`.
        pub const fn as_usize(self) -> usize {
            self.0 as usize
        }

        /// Encodes the length as little-endian bytes for the guest memory protocol.
        pub const fn to_le_bytes(self) -> [u8; Self::WIRE_SIZE] {
            self.0.to_le_bytes()
        }

        /// Decodes a length from little-endian bytes read from guest memory.
        ///
        /// Returns `None` if the decoded value exceeds [`MAX_CMDLINE_ARGS_LEN`].
        pub const fn from_le_bytes(bytes: [u8; Self::WIRE_SIZE]) -> Option<Self> {
            let val = u16::from_le_bytes(bytes);
            if (val as usize) > MAX_CMDLINE_ARGS_LEN {
                None
            } else {
                Some(Self(val))
            }
        }
    }

    impl core::fmt::Display for CmdlineArgsLen {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{}", self.0)
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
    /// Provides the raw value for [`KERNEL_END`], which can be used in constant-value expressions.
    ///
    pub const KERNEL_END_RAW: usize = 0x4000_0000;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`USER_BASE`], which can be used in constant-value expressions.
    ///
    pub const USER_BASE_RAW: usize = KERNEL_END_RAW;

    ///
    /// # Description
    ///
    /// Exclusive upper bound of the user virtual address space.
    ///
    pub const USER_END_RAW: usize = 0xf0000000;

    // Verify that USER_END_RAW stays above the kernel region and fixed user-space regions.
    const _: () = assert!(USER_END_RAW > USER_BASE_RAW, "USER_END_RAW underflows into kernel");
    const _: () =
        assert!(USER_END_RAW > USER_MMAP_END_RAW, "USER_END_RAW overlaps fixed user-space regions");

    ///
    /// # Description
    ///
    /// High address of the user stack region (initial stack pointer).
    /// The stack grows downward from this address.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_STACK_BASE_RAW: usize = USER_END_RAW;

    ///
    /// # Description
    ///
    /// Low address of the user stack region.
    /// The stack occupies [`USER_STACK_TOP_RAW`, [`USER_STACK_BASE_RAW`).
    ///
    pub const USER_STACK_TOP_RAW: usize = USER_STACK_BASE_RAW - USER_STACK_SIZE;

    ///
    /// # Description
    ///
    /// Size of the user stack.
    ///
    /// # Notes:
    ///
    /// - This size should be a multiple of a page size.
    ///
    pub const USER_STACK_SIZE: usize = 4 * crate::constants::MEGABYTE;

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
    /// Default stack size for spawned threads.
    ///
    /// Unlike the main thread whose stack is demand-paged within the [`USER_STACK_SIZE`] virtual
    /// region, additional threads have their stacks heap-allocated at this fixed size.
    ///
    /// # Notes:
    ///
    /// - This size should be a multiple of a page size.
    /// - Must be at least [`USER_STACK_MIN_SIZE`].
    ///
    pub const USER_THREAD_STACK_SIZE: usize = 512 * crate::constants::KILOBYTE;

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
    /// Derived as half of the VM's physical memory (`MEMORY_SIZE`) to leave room for the kernel,
    /// page tables, user stacks, and program text while still allowing large single-operation
    /// allocations.
    ///
    pub const USER_HEAP_CAPACITY: usize = crate::kernel::MEMORY_SIZE / 2;

    // Compile-time assertion: USER_HEAP_CAPACITY must be strictly less than MEMORY_SIZE.
    static_assert::assert_eq!(USER_HEAP_CAPACITY < crate::kernel::MEMORY_SIZE);
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

    /// Timer period in microseconds, derived from the kernel timer frequency.
    pub const TIMER_PERIOD_US: u64 =
        (crate::constants::MICROSECONDS_PER_SECOND as u64) / (crate::kernel::TIMER_FREQ as u64);

    /// I/O port that enables the guest to invoke functionalities of the virtual machine monitor.
    pub const DEFAULT_VMM_PORT: u16 = 0x604;

    /// Default VMM shutdown command
    pub const DEFAULT_VMM_SHUTDOWN_CMD: u16 = 0x2000;

    /// Default VMM pause command. This MUST be a value that's never an exit code.
    pub const DEFAULT_VMM_PAUSE_CMD: u16 = 0x3000;

    /// Default VMM snapshot command. Triggers a guest-initiated VM snapshot.
    pub const DEFAULT_VMM_SNAPSHOT_CMD: u16 = 0x4000;

    /// Default VMM boot-complete command. Sent by the kernel after boot finishes.
    pub const DEFAULT_VMM_BOOT_COMPLETE_CMD: u16 = 0x5000;

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

    /// Default base address for TSC base frequency register (32-bit wide read-only register).
    /// The VMM writes the host TSC base frequency (in MHz) here before boot so the kernel
    /// can use RDTSC-based LAPIC timer calibration without requiring CPUID leaf 0x16.
    pub const DEFAULT_MICROVM_CTRL_TSC_FREQ_MHZ: usize = 0x00000014;

    /// Offset of the kernel arguments length field (u16, little-endian).
    pub const DEFAULT_MICROVM_CTRL_KERNEL_ARGS_LEN: usize = 0x00000efc;

    /// Offset where the kernel arguments string data begins.
    pub const DEFAULT_MICROVM_CTRL_KERNEL_ARGS_DATA: usize = 0x00000f00;

    /// Maximum length (in bytes) of the kernel arguments.
    pub const MAX_KERNEL_ARGS_LEN: usize =
        DEFAULT_PVCLOCK_PAGE - DEFAULT_MICROVM_CTRL_KERNEL_ARGS_DATA;

    // Ensure MAX_KERNEL_ARGS_LEN fits in the u16 length field written to guest memory.
    ::static_assert::assert_eq!(MAX_KERNEL_ARGS_LEN <= u16::MAX as usize);

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

    /// Base address of the local APIC MMIO register page.
    #[cfg(feature = "whp")]
    pub const DEFAULT_LAPIC_BASE: usize = 0xFEE0_0000;
}
