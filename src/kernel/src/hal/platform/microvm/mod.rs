// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod pvclock;
// The 16/32-bit boot entry is x86-only; x86_64 boots directly in long mode via the
// arch boot path (hal/arch/x86_64/asm/start.rs) set up by the VMM's reset64.
#[cfg(target_arch = "x86")]
mod start;
#[cfg(target_arch = "x86")]
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
pub unsafe fn setup_heap_backing_storage() -> Result<(), ::sys::error::Error> {
    crate::mm::kheap::set_backing_storage(
        HEAP_STORAGE.memory.as_mut_ptr(),
        HEAP_STORAGE.memory.len(),
    )
}

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
pub unsafe fn setup_klog_backing_storage() -> Result<(), ::sys::error::Error> {
    crate::klog::set_backing_storage(KLOG_BUFFER_STORAGE.memory.as_mut_ptr())
}

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
pub(super) unsafe fn disable_interrupts() {
    ::arch::cpu::cli();
}

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
pub(super) unsafe fn enable_interrupts() {
    ::arch::cpu::sti();
}

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
pub(super) unsafe fn wait_for_interrupt() {
    ::arch::cpu::halt();
}

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
pub unsafe fn putb(b: u8) {
    ::arch::io::out8(::config::microvm::DEFAULT_STDOUT_PORT, b);
}

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
pub unsafe fn vmbus_write(addr: *const u8) {
    use crate::PERF_VMBUS_WRITE;
    use core::hint;

    PERF_VMBUS_WRITE.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    #[allow(clippy::unit_arg)]
    hint::black_box(::arch::io::out32(::config::microvm::DEFAULT_STDOUT_PORT, addr as u32));
}

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
pub unsafe fn vmbus_read(addr: *mut u8) {
    use crate::PERF_VMBUS_READ;
    use core::hint;

    PERF_VMBUS_READ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    #[allow(clippy::unit_arg)]
    hint::black_box(::arch::io::out32(::config::microvm::DEFAULT_STDIN_PORT, addr as u32))
}

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
pub(in crate::hal::platform) fn do_shutdown(status: usize) -> ! {
    unsafe {
        let status: u16 = (status & 0xffff) as u16;
        let cmd: u16 = ::config::microvm::DEFAULT_VMM_SHUTDOWN_CMD;
        ::arch::io::out32(
            ::config::microvm::DEFAULT_VMM_PORT,
            ((cmd as u32) << 16) | (status as u32),
        )
    };
    loop {
        core::hint::spin_loop();
    }
}

///
/// # Description
///
/// Requests the VMM to create a snapshot of the virtual machine state.
/// The snapshot command is issued via a port I/O write to the VMM control port.
/// The VMM will pause the vCPU, save VM state to disk, and resume execution.
/// On restore, execution resumes from the instruction following this call.
///
pub fn snapshot() {
    // SAFETY: The port I/O write targets the VMM control port with a well-known snapshot
    // command value. The VMM is guaranteed to support this command on the microvm platform.
    unsafe {
        let cmd: u16 = ::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD;
        ::arch::io::out32(::config::microvm::DEFAULT_VMM_PORT, (cmd as u32) << 16);
    }
}

///
/// # Description
///
/// Signals the VMM that kernel startup is complete and user-space applications are about to start.
///
pub fn signal_startup_complete() {
    // SAFETY: The port I/O write targets the VMM control port with a well-known
    // boot-complete command value.
    unsafe {
        let cmd: u16 = ::config::microvm::DEFAULT_VMM_BOOT_COMPLETE_CMD;
        ::arch::io::out32(::config::microvm::DEFAULT_VMM_PORT, (cmd as u32) << 16);
    }
}

///
/// # Description
///
/// Returns the boot kernel stack top pointer.
///
/// # Returns
///
/// A pointer to the top of the boot kernel stack.
///
// Used by x86 CPU init today; consumed by x86_64 GDT/TSS bring-up (Phase 5).
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
pub fn get_kstack_top() -> *const u8 {
    unsafe extern "C" {
        static kstack: u8;
    }
    // Safety: The `kstack` symbol is defined in `start.rs` as a BSS-resident symbol representing
    // the top of the boot kernel stack. Taking its address is safe as it points to a valid memory
    // location reserved for the boot stack.
    unsafe { &kstack as *const u8 }
}

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
pub fn get_kstack_guard_base() -> usize {
    unsafe extern "C" {
        static kstack_guard: u8;
    }
    // Safety: The `kstack_guard` symbol is defined in `start.rs` as a BSS-resident symbol
    // representing the base of the boot kernel stack guard page. Taking its address is safe as it
    // points to a valid memory location reserved for the guard page.
    unsafe { &kstack_guard as *const u8 as usize }
}

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
pub fn gva_to_gpa(gva: usize) -> usize {
    gva
}

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
#[allow(dead_code)] // identity helper; callers re-wired during x86_64 bring-up
pub fn virt_to_phys(vaddr: usize) -> usize {
    vaddr
}

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
pub fn is_valid_physical_address(addr: VirtualAddress) -> bool {
    addr < VirtualAddress::from_raw_value(config::kernel::MEMORY_SIZE)
}

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
#[allow(dead_code)] // validation helper; not on the current boot path
pub fn is_valid_physical_region(start: usize, size: usize) -> bool {
    // Reject zero-length regions.
    if size == 0 {
        return false;
    }

    // Compute the exclusive end, guarding against overflow.
    match start.checked_add(size) {
        Some(end) => end <= config::kernel::MEMORY_SIZE,
        None => false,
    }
}

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
pub fn max_physical_address() -> usize {
    config::kernel::MEMORY_SIZE - 1
}

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
pub fn parse_bootinfo(magic: u32, info: usize) -> Result<BootInfo, Error> {
    // Check if magic number matches what we expect.
    if magic != ::config::microvm::DEFAULT_BOOT_MAGIC {
        let reason: &str = "invalid boot magic number";
        error!("magic={:#010x}, info={:#010x} (error={})", magic, info, reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    trace!("magic={:#010x}, info={:#010x}", magic, info);

    // Retrieve initrd information.
    // - Lower bits encode the size of the initrd.
    // - Higher bits encode the base address of the initrd.
    let nzeros: usize = ::config::microvm::DEFAULT_INITRD_BASE.trailing_zeros() as usize;
    let initrd_size: usize = info & ((1 << nzeros) - 1);
    let initrd_base: usize = info & !((1 << nzeros) - 1);

    let mut kernel_modules: LinkedList<KernelModule> = LinkedList::new();

    // Read kernel arguments from the dedicated control registers written by the VMM.
    // The length (u16 LE) is at DEFAULT_MICROVM_CTRL_KERNEL_ARGS_LEN and the UTF-8
    // data starts at DEFAULT_MICROVM_CTRL_KERNEL_ARGS_DATA.
    let kernel_args: &'static str = unsafe {
        let len_addr: usize = ::config::microvm::DEFAULT_MICROVM_CTRL_BASE
            + ::config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_LEN;
        let kernel_args_len: u16 = u16::from_le(core::ptr::read_volatile(len_addr as *const u16));
        let len: usize = kernel_args_len as usize;
        if len > 0 && len <= ::config::microvm::MAX_KERNEL_ARGS_LEN {
            let data_addr: usize = ::config::microvm::DEFAULT_MICROVM_CTRL_BASE
                + ::config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_DATA;
            let kernel_args_bytes: &'static [u8] =
                core::slice::from_raw_parts(data_addr as *const u8, len);
            match core::str::from_utf8(kernel_args_bytes) {
                Ok(s) => s,
                Err(_) => {
                    error!("parse_bootinfo(): invalid UTF-8 in kernel args control register");
                    ""
                },
            }
        } else {
            ""
        }
    };

    // Register initrd as a kernel module.
    if initrd_size != 0 {
        let total_bytes: usize = match initrd_size.checked_mul(mem::PAGE_SIZE) {
            Some(total_bytes) => total_bytes,
            None => {
                let reason: &str = "initrd size overflow";
                error!("parse_bootinfo(): {}", reason);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        // Check that the initrd region does not overlap the user mmap region.
        let initrd_end: usize = match initrd_base.checked_add(total_bytes) {
            Some(end) => end,
            None => {
                let reason: &str = "initrd bounds overflow";
                error!("parse_bootinfo(): {}", reason);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        if initrd_end > ::config::memory_layout::USER_MMAP_BASE_RAW {
            let reason: &str = "initrd region overlaps user mmap region";
            error!(
                "parse_bootinfo(): {} (initrd_end={:#010x}, mmap_base={:#010x})",
                reason,
                initrd_end,
                ::config::memory_layout::USER_MMAP_BASE_RAW
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let image_data: &'static [u8] =
            unsafe { core::slice::from_raw_parts(initrd_base as *const u8, total_bytes) };

        // Detect initrd format by checking for NVMB multibinary magic.
        if image_data.len() >= multibin::MAGIC.len()
            && image_data[..multibin::MAGIC.len()] == multibin::MAGIC
        {
            info!("parse_bootinfo(): multibinary initrd detected");
            let modules: Vec<KernelModule> = crate::multibin::parse(image_data, initrd_base)?;
            kernel_modules.extend(modules);
        } else {
            // Single ELF binary with length-prefixed args after the initrd.
            info!("parse_bootinfo(): single-binary initrd detected");
            let initrd_cmdline_len_base: usize = initrd_base + total_bytes;
            let initrd_cmdline_base: usize = initrd_cmdline_len_base + CmdlineArgsLen::WIRE_SIZE;

            // Validate that the length field fits within the known initrd allocation.
            let cmdline_len_end: usize =
                match initrd_cmdline_len_base.checked_add(CmdlineArgsLen::WIRE_SIZE) {
                    Some(end) => end,
                    None => {
                        let reason: &str = "cmdline length field address overflow";
                        error!("parse_bootinfo(): {}", reason);
                        return Err(Error::new(ErrorCode::InvalidArgument, reason));
                    },
                };
            if cmdline_len_end > ::config::memory_layout::USER_MMAP_BASE_RAW {
                let reason: &str = "cmdline length field exceeds memory bounds";
                error!("parse_bootinfo(): {}", reason);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }

            let cmdline_len: CmdlineArgsLen = unsafe {
                match CmdlineArgsLen::from_le_bytes(
                    *(initrd_cmdline_len_base as *const [u8; CmdlineArgsLen::WIRE_SIZE]),
                ) {
                    Some(v) => v,
                    None => {
                        let reason: &str = "cmdline length exceeds maximum";
                        error!("parse_bootinfo(): {}", reason);
                        return Err(Error::new(ErrorCode::InvalidArgument, reason));
                    },
                }
            };

            // Validate that the cmdline payload fits within memory bounds.
            let cmdline_end: usize = match initrd_cmdline_base.checked_add(cmdline_len.as_usize()) {
                Some(end) => end,
                None => {
                    let reason: &str = "cmdline payload address overflow";
                    error!("parse_bootinfo(): {}", reason);
                    return Err(Error::new(ErrorCode::InvalidArgument, reason));
                },
            };
            if cmdline_end > ::config::memory_layout::USER_MMAP_BASE_RAW {
                let reason: &str = "cmdline payload exceeds memory bounds";
                error!(
                    "parse_bootinfo(): {} (cmdline_end={:#010x}, mmap_base={:#010x})",
                    reason,
                    cmdline_end,
                    ::config::memory_layout::USER_MMAP_BASE_RAW
                );
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }

            // SAFETY: the cmdline bytes reside in bootloader-provided memory that persists for the
            // kernel's lifetime.
            let cmdline_bytes: &'static [u8] = unsafe {
                core::slice::from_raw_parts(
                    initrd_cmdline_base as *const u8,
                    cmdline_len.as_usize(),
                )
            };

            // Validate UTF-8.
            let module_cmdline: &'static str = match core::str::from_utf8(cmdline_bytes) {
                Ok(s) => s,
                Err(_) => {
                    let reason: &str = "invalid UTF-8 in command line";
                    error!("parse_bootinfo(): {}", reason);
                    return Err(Error::new(ErrorCode::InvalidArgument, reason));
                },
            };

            info!(
                "initrd_base={:#010x}, initrd_size={:#010x}, cmdline_len={:?}, cmdline={:?}",
                initrd_base, total_bytes, cmdline_len, module_cmdline
            );

            // Module size must cover the ELF binary AND the trailing cmdline area
            // (length field + payload) so that the HAL maps the entire region.
            let module_size: usize =
                total_bytes + CmdlineArgsLen::WIRE_SIZE + cmdline_len.as_usize();

            let module: KernelModule = KernelModule::new(
                PhysicalAddress::from_raw_value(initrd_base)?,
                module_size,
                module_cmdline,
            );
            kernel_modules.push_back(module);
        }
    }

    Ok(BootInfo::new(
        None,
        None,
        LinkedList::new(),
        LinkedList::new(),
        IoMemoryAllocator::new(),
        kernel_modules,
        kernel_args,
    ))
}

///
/// # Description
///
/// Logs the values of the MicroVM control registers.
///
fn log_control_registers() {
    // SAFETY: The MicroVM control registers reside in a read-only MMIO page that is mapped into the
    // host kernel during platform initialization, so reading them with `read_volatile` is safe.
    unsafe {
        let null_value: u32 =
            core::ptr::read_volatile(::config::microvm::DEFAULT_MICROVM_CTRL_NULL as *const u32);
        let credits_value: u32 =
            core::ptr::read_volatile(::config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as *const u32);
        let pause_value: u32 = core::ptr::read_volatile(
            ::config::microvm::DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED as *const u32,
        );
        let ramfs_base_value: u32 = core::ptr::read_volatile(
            ::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_BASE as *const u32,
        );
        let ramfs_size_value: u32 = core::ptr::read_volatile(
            ::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_SIZE as *const u32,
        );

        info!(
            "microvm ctrl registers: base={:#010x}, null={:#010x}, credits={:#010x}, \
             pause={:#010x}, ramfs_base={:#010x}, ramfs_size={:#010x}",
            ::config::microvm::DEFAULT_MICROVM_CTRL_BASE,
            null_value,
            credits_value,
            pause_value,
            ramfs_base_value,
            ramfs_size_value
        );
    }
}

fn register_ramfs_mmio_region(
    ioaddresses: &mut IoMemoryAllocator,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<(), Error> {
    if let Some((ramfs_base, ramfs_size)) = read_ramfs_registers() {
        trace!("ramfs region detected: base={:#010x}, size={:#x}", ramfs_base, ramfs_size);

        let ramfs_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new_mmio(
            RAMFS_REGION_NAME,
            PageAligned::from_raw_value(ramfs_base)?,
            ramfs_size,
            AccessPermission::RDWR,
            MmioCachePolicy::WRITE_BACK,
        )?;

        ioaddresses.register(RAMFS_MMIO_TAG, ramfs_region.clone())?;
        mmio_regions.push_back(ramfs_region);
    }

    Ok(())
}

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
fn read_ramfs_registers() -> Option<(usize, usize)> {
    // SAFETY: The control registers are memory-mapped at fixed addresses defined by the
    // MicroVM specification. These addresses are guaranteed to be valid when running on
    // the MicroVM platform.
    unsafe {
        let base_value: usize =
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_BASE) as usize;
        let size_value: usize =
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_SIZE) as usize;

        if base_value == 0 || size_value == 0 {
            None
        } else {
            Some((base_value, size_value))
        }
    }
}

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
unsafe fn read_control_register(offset: usize) -> u32 {
    let addr: *const u32 = (::config::microvm::DEFAULT_MICROVM_CTRL_BASE + offset) as *const u32;
    core::ptr::read_volatile(addr)
}

///
/// # Description
///
/// Returns the TSC base frequency in MHz as provided by the VMM via a
/// microvm control register. Returns `0` when the VMM did not populate
/// the register.
///
#[cfg(feature = "whp")]
pub fn tsc_base_frequency_mhz() -> u32 {
    // SAFETY: The control register is memory-mapped at a fixed address
    // inside the kernel's identity-mapped region and is guaranteed to be
    // valid on the microvm platform after ELF load.
    unsafe { read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_TSC_FREQ_MHZ) }
}

fn register_pic_ioports(ioports: &mut IoPortAllocator) -> Result<(), Error> {
    // Register I/O ports for 8259 PIC.
    ioports.register_read_write(pic::PIC_CTRL_MASTER as u16)?;
    ioports.register_read_write(pic::PIC_DATA_MASTER as u16)?;
    ioports.register_read_write(pic::PIC_CTRL_SLAVE as u16)?;
    ioports.register_read_write(pic::PIC_DATA_SLAVE as u16)?;
    Ok(())
}

#[cfg(all(feature = "pit", not(feature = "whp")))]
fn register_pit(ioports: &mut IoPortAllocator) -> Result<Pit, Error> {
    // Register ports for the PIT.

    ioports.register_read_write(::arch::cpu::pit::PIT_CTRL)?;
    ioports.register_read_write(::arch::cpu::pit::PIT_DATA)?;

    Pit::new(ioports, ::config::kernel::TIMER_FREQ)
}

/// Registers PIT calibration ports (channel 2 + speaker gate) so the interrupt controller can
/// allocate them during LAPIC timer calibration.
#[cfg(all(feature = "pit", feature = "whp"))]
fn register_pit_ports(ioports: &mut IoPortAllocator) -> Result<(), Error> {
    ioports.register_read_write(::arch::cpu::pit::PIT_CTRL)?;
    ioports.register_read_write(::arch::cpu::pit::PIT_DATA_CH2)?;
    ioports.register_read_write(::arch::cpu::pit::PIT_SPEAKER_GATE)?;
    Ok(())
}

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
) -> Result<Platform, Error> {
    // Ensure the CPU exposes the TSC feature (CPUID.01H:EDX[4]).
    // The pvclock subsystem and RDTSC-based timekeeping depend on this.
    if !::arch::cpu::cpuid::has_tsc() {
        let reason: &str = "CPU does not support TSC (RDTSC)";
        error!("{}", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    register_pic_ioports(ioports)?;

    // NOTE: on microvm, PLATFORM_BASE_ADDR is 0x0 and the kernel loads at 0x100000. The gap
    // between physical address 0 and __KERNEL_START is already covered by the single contiguous
    // physical memory region starting at GPA 0, so no explicit pre-kernel gap registration is
    // needed here.

    // Register MicroVM control registers.
    let scratch_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new_mmio(
        "microvm-ctrl-registers",
        PageAligned::from_raw_value(::config::microvm::DEFAULT_MICROVM_CTRL_BASE)?,
        mem::PAGE_SIZE,
        AccessPermission::RDONLY,
        MmioCachePolicy::UNCACHEABLE,
    )?;
    ioaddresses.register(MICROVM_CTRL_MMIO_TAG, scratch_region.clone())?;
    mmio_regions.push_back(scratch_region);

    // Register pvclock page so the kernel can read TSC calibration data.
    let pvclock_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new_mmio(
        "pvclock-page",
        PageAligned::from_raw_value(::config::microvm::DEFAULT_PVCLOCK_PAGE)?,
        mem::PAGE_SIZE,
        AccessPermission::RDONLY,
        MmioCachePolicy::UNCACHEABLE,
    )?;
    ioaddresses.register(PVCLOCK_MMIO_TAG, pvclock_region.clone())?;
    mmio_regions.push_back(pvclock_region);

    // Register the LAPIC MMIO page only for the WHP microvm backend.
    // The guest uses this page to enable LAPIC software delivery and to
    // acknowledge interrupts through the WHP LAPIC emulator.
    #[cfg(feature = "whp")]
    {
        let lapic_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new_mmio(
            "lapic-registers",
            PageAligned::from_raw_value(::config::microvm::DEFAULT_LAPIC_BASE)?,
            mem::PAGE_SIZE,
            AccessPermission::RDWR,
            MmioCachePolicy::UNCACHEABLE,
        )?;
        ioaddresses.register(LAPIC_MMIO_TAG, lapic_region.clone())?;
        mmio_regions.push_back(lapic_region);
    }

    log_control_registers();
    register_ramfs_mmio_region(ioaddresses, mmio_regions)?;

    // On WHP, register PIT calibration ports before arch init so the interrupt
    // controller can allocate them during LAPIC timer calibration.
    #[cfg(all(feature = "pit", feature = "whp"))]
    register_pit_ports(ioports)?;

    // Install GDT backing storage. On microvm the GDT lives in a BSS-allocated static.
    // Safety: GDT_STORAGE is a static array of Gdte entries with repr(C, align(8)),
    // so the pointer is properly aligned and valid for GDT_NUM_ENTRIES entries.
    // The static lifetime guarantees the storage outlives all GDT usage.
    // This is the only call to set_backing_storage() in the microvm init path.
    unsafe {
        gdt::Gdt::set_backing_storage(GDT_STORAGE.as_mut_ptr())?;
    }

    // Install IDT and IDTR backing storage. On microvm these live in BSS-allocated area.
    // Safety: IDT_STORAGE is a static array of Idte entries with repr(C, align(8)),
    // so the pointer is properly aligned and valid for IDT_LEN entries.
    // IDTR_STORAGE is a static Idtr with repr(C, packed).
    // The static lifetime guarantees the storage outlives all IDT usage.
    // This is the only call to Idt::set_backing_storage() in the microvm init path.
    unsafe {
        idt::Idt::set_backing_storage(IDT_STORAGE.as_mut_ptr(), &raw mut IDTR_STORAGE)?;
    }

    let arch = x86::init(ioports, ioaddresses, madt)?;

    // Build a bitmap representing the physical memory layout.
    let physical_memory_layout: Bitmap = {
        // Safety: the frame allocator storage is valid and has a static lifetime.
        let storage: RawArray<u8> = unsafe {
            let (ptr, len): (*mut u8, usize) =
                (FRAME_ALLOCATOR_STORAGE.as_mut_ptr(), FRAME_ALLOCATOR_STORAGE.len());
            RawArray::from_raw_parts(ptr, len)?
        };
        Bitmap::from_raw_array(storage)?
    };

    Ok(Platform {
        arch,
        #[cfg(all(feature = "pit", not(feature = "whp")))]
        _pit: register_pit(ioports)?,
        physical_memory_layout: Some(physical_memory_layout),
    })
}
