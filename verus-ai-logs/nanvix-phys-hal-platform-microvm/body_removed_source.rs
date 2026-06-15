// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("mod.spec.rs");
#[cfg(verus_keep_ghost)]
include!("mod.proof.rs");

pub mod pvclock;
mod start;
mod start16;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::RawArray,
    hal::{
        arch::x86::{
            self,
            cpu::idt,
            mem::gdt,
            Arch,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
        },
        mem::{
            AccessPermission,
            Address,
            MemoryRegion,
            MmioCachePolicy,
            PageAligned,
            PhysicalAddress,
            TruncatedMemoryRegion,
            VirtualAddress,
        },
        platform::{
            bootinfo::BootInfo,
            madt::MadtInfo,
            region_names::RAMFS_REGION_NAME,
            region_tags::{
                MICROVM_CTRL_MMIO_TAG,
                PVCLOCK_MMIO_TAG,
                RAMFS_MMIO_TAG,
            },
        },
    },
    kmod::KernelModule,
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::arch::{
    cpu::{
        idt::Idte,
        idtr::Idtr,
        pic,
    },
    mem,
    mem::gdt::Gdte,
};
use ::bitmap::Bitmap;
use ::config::system::CmdlineArgsLen;
use ::sys::error::{
    Error,
    ErrorCode,
};

#[cfg(feature = "whp")]
use crate::hal::platform::region_tags::LAPIC_MMIO_TAG;

#[cfg(all(feature = "pit", not(feature = "whp")))]
use crate::hal::platform::pit::Pit;

//==================================================================================================
// Constants
//==================================================================================================

/// Number of page tables needed for identity-mapping physical memory regions.
///
/// On microvm all physical memory is contiguous starting at GPA 0, so the base count
/// (one page table per `PGTAB_SIZE` bytes) is sufficient. When the WHP backend is enabled,
/// an additional page table is needed for the LAPIC MMIO region at `0xFEE0_0000`, which
/// lies outside the identity-mapped physical memory range.
///
#[cfg(feature = "whp")]
pub const NUM_PAGE_TABLES: usize = config::kernel::MEMORY_SIZE / mem::PGTAB_SIZE + 1;
#[cfg(not(feature = "whp"))]
pub const NUM_PAGE_TABLES: usize = config::kernel::MEMORY_SIZE / mem::PGTAB_SIZE;

/// Total number of physical frames covered by the configured machine memory size.
pub const NFRAMES: usize = config::kernel::MEMORY_SIZE / mem::FRAME_SIZE;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Platform {
    pub arch: Arch,
    #[cfg(all(feature = "pit", not(feature = "whp")))]
    pub _pit: Pit,
    /// A bitmap representing the physical memory layout, owned by the platform and consumed
    /// by the memory manager during system initialization.
    pub physical_memory_layout: Option<Bitmap>,
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Frame allocator storage.
static mut FRAME_ALLOCATOR_STORAGE: [u8; NFRAMES / u8::BITS as usize] =
    [0; NFRAMES / u8::BITS as usize];

/// GDT backing storage, allocated in BSS.
static mut GDT_STORAGE: [Gdte; gdt::GDT_NUM_ENTRIES] = gdt::DEFAULT_ENTRIES;

/// IDT backing storage, allocated in BSS.
static mut IDT_STORAGE: [Idte; idt::IDT_LEN] = unsafe { core::mem::zeroed() };

/// IDTR backing storage, allocated in BSS.
static mut IDTR_STORAGE: Idtr = unsafe { core::mem::zeroed() };

/// Heap backing storage, allocated in BSS.
#[repr(align(4096))]
struct HeapStorage {
    memory: [u8; crate::mm::kheap::MIN_HEAP_SIZE],
}

::static_assert::assert_eq_align!(HeapStorage, mem::PAGE_SIZE);

/// Heap backing storage.
static mut HEAP_STORAGE: HeapStorage = HeapStorage {
    memory: [0; crate::mm::kheap::MIN_HEAP_SIZE],
};

/// Klog buffer backing storage, allocated in BSS.
/// Only present in single-core builds where the klog buffer is active.
#[cfg(not(feature = "smp"))]
#[repr(align(8))]
struct KlogBufferStorage {
    memory: [u8; crate::klog::KLOG_BUFFER_STORAGE_SIZE],
}

#[cfg(not(feature = "smp"))]
static mut KLOG_BUFFER_STORAGE: KlogBufferStorage = KlogBufferStorage {
    memory: [0; crate::klog::KLOG_BUFFER_STORAGE_SIZE],
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Points the kernel heap at the BSS-resident `HEAP_STORAGE` buffer.
///
/// Must be called before [`crate::mm::kheap::init()`].
///
/// # Safety
///
/// This function accesses the `HEAP_STORAGE` static mutable.
///
pub unsafe fn setup_heap_backing_storage() -> Result<(), ::sys::error::Error> { ... }

///
/// # Description
///
/// Points the kernel log buffer at the BSS-resident `KLOG_BUFFER_STORAGE` buffer.
///
/// Must be called before the first logging macro invocation.
///
/// # Safety
///
/// This function accesses the `KLOG_BUFFER_STORAGE` static mutable.
///
#[cfg(not(feature = "smp"))]
pub unsafe fn setup_klog_backing_storage() -> Result<(), ::sys::error::Error> { ... }

///
/// # Description
///
/// Disables all interrupts on the calling core.
///
/// # Safety
///
/// This function is unsafe because it modifies the CPU state.
///
/// It is safe to call this function only when the CPU is in a state where interrupts can be
/// disabled.
///
pub(super) unsafe fn disable_interrupts() { ... }

///
/// # Description
///
/// Enables all interrupts on the calling core.
///
/// # Safety
///
/// This function is unsafe because it modifies the CPU state.
///
/// It is safe to call this function only when the CPU is in a state where interrupts can be
/// enabled.
///
pub(super) unsafe fn enable_interrupts() { ... }

///
/// # Description
///
/// Waits for an interrupt to happen.
///
/// # Safety
///
/// This function is unsafe because it modifies the CPU state.
///
/// It is safe to call this function only when the CPU is able to receive interrupts.
///
pub(super) unsafe fn wait_for_interrupt() { ... }

///
/// # Description
///
/// Writes the 8-bit value `b` to the platform's standard output device.
///
/// # Parameters
///
/// - `b`: Value to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn putb(b: u8) { ... }

///
/// # Description
///
/// Places a write request to the platform's standard output device.
///
/// # Parameters
///
/// - `addr`: Address where data should be written from.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
#[cfg(feature = "stdio")]
pub unsafe fn vmbus_write(addr: *const u8) { ... }

///
/// # Description
///
/// Places a read request to the platform's standard input device.
///
/// # Parameters
///
/// - `addr`: Address where data should be read into.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard input device is present.
/// - It assumes that the standard input device was properly initialized.
/// - It does not prevent concurrent access to the standard input device.
///
#[cfg(feature = "stdio")]
pub unsafe fn vmbus_read(addr: *mut u8) { ... }

///
/// # Description
///
/// Shuts down the machine.
///
/// # Parameters
///
/// - `status`: The shutdown status code.
///
/// # Returns
///
/// This function never returns.
///
pub(in crate::hal::platform) fn do_shutdown(status: usize) -> ! { ... }

///
/// # Description
///
/// Requests the VMM to create a snapshot of the virtual machine state.
/// The snapshot command is issued via a port I/O write to the VMM control port.
/// The VMM will pause the vCPU, save VM state to disk, and resume execution.
/// On restore, execution resumes from the instruction following this call.
///
pub fn snapshot() { ... }

///
/// # Description
///
/// Signals the VMM that kernel startup is complete and user-space applications are about to start.
///
pub fn signal_startup_complete() { ... }

///
/// # Description
///
/// Returns the boot kernel stack top pointer.
///
/// # Returns
///
/// A pointer to the top of the boot kernel stack.
///
pub fn get_kstack_top() -> *const u8 { ... }

///
/// # Description
///
/// Returns the base address of the boot kernel stack guard page.
///
/// # Returns
///
/// The base address of the boot kernel stack guard page.
///
#[cfg(all(debug_assertions, feature = "exception-stack-guard"))]
pub fn get_kstack_guard_base() -> usize { ... }

///
/// # Description
///
/// Translates a guest virtual address to a guest physical address.
///
/// # Returns
///
/// The guest physical address corresponding to the given guest virtual address.
///
///
#[inline(always)]
pub fn gva_to_gpa(gva: usize) -> usize { ... }

///
/// # Description
///
/// Translates a virtual address to a physical address.
///
/// # Returns
///
/// The physical address corresponding to the given virtual address.
///
#[inline(always)]
pub fn virt_to_phys(vaddr: usize) -> usize { ... }

///
/// # Description
///
/// Checks whether the given virtual address corresponds to a valid physical address on the Microvm
/// platform.
///
/// # Parameters
///
/// - `addr`: The virtual address to validate.
///
/// # Returns
///
/// `true` if `addr` falls within the physical address space, `false` otherwise.
///
#[inline(always)]
pub fn is_valid_physical_address(addr: VirtualAddress) -> bool { ... }

///
/// # Description
///
/// Checks whether the given physical memory region lies entirely within physical memory on the
/// Microvm platform.
///
/// # Parameters
///
/// - `start`: Starting physical address of the region.
/// - `size`: Size of the region in bytes.
///
/// # Returns
///
/// `true` if the entire region lies within physical memory, `false` otherwise.
///
#[inline(always)]
pub fn is_valid_physical_region(start: usize, size: usize) -> bool { ... }

///
/// # Description
///
/// Returns the maximum physical address on the Microvm platform.
///
/// All physical memory is contiguous starting at GPA 0 up to `MEMORY_SIZE`.
///
/// # Returns
///
/// The maximum physical address value.
///
#[inline(always)]
pub fn max_physical_address() -> usize { ... }

///
/// # Description
///
/// Parses boot information.
///
/// # Parameters
///
/// - `magic`: Magic number.
/// - `info`:  Address of the boot information.
///
/// # Returns
///
/// A new boot information structure.
///
pub fn parse_bootinfo(magic: u32, info: usize) -> Result<BootInfo, Error> { ... }

///
/// # Description
///
/// Logs the values of the MicroVM control registers.
///
fn log_control_registers() { ... }

fn register_ramfs_mmio_region(
    ioaddresses: &mut IoMemoryAllocator,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<(), Error> { ... }

///
/// # Description
///
/// Reads the RAMFS base and size from MicroVM control registers.
///
/// # Safety
///
/// This function reads from memory-mapped control registers at addresses defined by
/// `DEFAULT_MICROVM_CTRL_BASE`. The caller must ensure that the MicroVM platform is
/// initialized and these addresses are valid and mapped.
///
fn read_ramfs_registers() -> Option<(usize, usize)> { ... }

///
/// # Description
///
/// Reads a 32-bit value from a MicroVM control register.
///
/// # Parameters
///
/// - `offset`: Offset from `DEFAULT_MICROVM_CTRL_BASE` to read.
///
/// # Returns
///
/// The 32-bit value at the specified control register.
///
/// # Safety
///
/// The caller must ensure that `DEFAULT_MICROVM_CTRL_BASE + offset` points to a valid,
/// mapped memory-mapped I/O register. This function performs a volatile read.
///
unsafe fn read_control_register(offset: usize) -> u32 { ... }

///
/// # Description
///
/// Returns the TSC base frequency in MHz as provided by the VMM via a
/// microvm control register. Returns `0` when the VMM did not populate
/// the register.
///
#[cfg(feature = "whp")]
pub fn tsc_base_frequency_mhz() -> u32 { ... }

fn register_pic_ioports(ioports: &mut IoPortAllocator) -> Result<(), Error> { ... }

#[cfg(all(feature = "pit", not(feature = "whp")))]
fn register_pit(ioports: &mut IoPortAllocator) -> Result<Pit, Error> { ... }

/// Registers PIT calibration ports (channel 2 + speaker gate) so the interrupt controller can
/// allocate them during LAPIC timer calibration.
#[cfg(all(feature = "pit", feature = "whp"))]
fn register_pit_ports(ioports: &mut IoPortAllocator) -> Result<(), Error> { ... }

///
/// # Description
///
/// Initializes the microvm platform.
///
/// # Parameters
///
/// - `ioports`: I/O port allocator.
/// - `ioaddresses`: I/O memory allocator.
/// - `_memory_regions`: Memory regions.
/// - `mmio_regions`: MMIO regions.
/// - `madt`: MADT information.
/// - `_mem_lower`: Lower memory size.
///
/// # Returns
///
/// Upon success, the initialized platform is returned. Upon failure, an error is returned instead.
///
pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    _memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    madt: &Option<MadtInfo>,
    _mem_lower: Option<usize>,
) -> Result<Platform, Error> { ... }
