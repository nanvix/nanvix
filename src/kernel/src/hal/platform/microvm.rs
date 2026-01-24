// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::{
            self,
            Arch,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
        },
        mem::{
            MemoryRegion,
            MemoryRegionType,
            PhysicalAddress,
            TruncatedMemoryRegion,
        },
        platform::{
            bootinfo::BootInfo,
            madt::MadtInfo,
        },
    },
    kmod::KernelModule,
};
use ::alloc::{
    collections::linked_list::LinkedList,
    string::ToString,
};
use ::arch::{
    cpu::pic,
    mem,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
};

#[cfg(feature = "pit")]
use crate::hal::platform::pit::Pit;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Platform {
    pub arch: Arch,
    #[cfg(feature = "pit")]
    pub _pit: Pit,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

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
/// Shutdowns the machine.
///
/// # Parameters
///
/// - `status`: The shutdown status code.
///
/// # Return
///
/// This function never returns.
///
pub fn shutdown(status: usize) -> ! {
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
    let initrd_cmdline_len_base: usize = initrd_base + (initrd_size * mem::PAGE_SIZE);
    let initrd_cmdline_base: usize = initrd_cmdline_len_base + core::mem::size_of::<u8>();

    let mut kernel_modules: LinkedList<KernelModule> = LinkedList::new();

    // Register initrd as a kernel module.
    if initrd_size != 0 {
        let cmdline_len: u8 = unsafe { *(initrd_cmdline_len_base as *const u8) };
        let cmdline_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(initrd_cmdline_base as *const u8, cmdline_len as usize)
        };
        let cmdline: &str = match core::str::from_utf8(cmdline_bytes) {
            Ok(s) => s,
            Err(_) => {
                let reason: &str = "invalid UTF-8 in command line";
                error!("invalid UTF-8 in command line");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        info!(
            "initrd_base={:#010x}, initrd_size={:#010x}, cmdline_len={:?}, cmdline={:?}",
            initrd_base,
            (initrd_size * mem::PAGE_SIZE),
            cmdline_len,
            cmdline
        );

        // Add kernel module to the list of kernel modules.
        let module: KernelModule = KernelModule::new(
            PhysicalAddress::from_raw_value(initrd_base)?,
            initrd_size * mem::PAGE_SIZE,
            cmdline.to_string(),
        );
        kernel_modules.push_back(module);
    }

    Ok(BootInfo::new(None, None, LinkedList::new(), LinkedList::new(), kernel_modules))
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

fn register_pic_ioports(ioports: &mut IoPortAllocator) -> Result<(), Error> {
    // Register I/O ports for 8259 PIC.
    ioports.register_read_write(pic::PIC_CTRL_MASTER as u16)?;
    ioports.register_read_write(pic::PIC_DATA_MASTER as u16)?;
    ioports.register_read_write(pic::PIC_CTRL_SLAVE as u16)?;
    ioports.register_read_write(pic::PIC_DATA_SLAVE as u16)?;
    Ok(())
}

#[cfg(feature = "pit")]
fn register_pit(ioports: &mut IoPortAllocator) -> Result<Pit, Error> {
    // Register ports for the PIT.

    ioports.register_read_write(::arch::cpu::pit::PIT_CTRL)?;
    ioports.register_read_write(::arch::cpu::pit::PIT_DATA)?;

    Pit::new(ioports, ::config::kernel::TIMER_FREQ)
}

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
    _mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    madt: &Option<MadtInfo>,
    _mem_lower: Option<usize>,
) -> Result<Platform, Error> {
    register_pic_ioports(ioports)?;

    // Register MicroVM control registers.
    let scratch_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "microvm-ctrl-registers",
        VirtualAddress::from_raw_value(::config::microvm::DEFAULT_MICROVM_CTRL_BASE),
        mem::PAGE_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(scratch_region);

    log_control_registers();

    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
        #[cfg(feature = "pit")]
        _pit: register_pit(ioports)?,
    })
}
