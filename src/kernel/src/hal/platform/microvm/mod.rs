// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod frame;
pub mod kpool;
pub mod pvclock;

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
            AccessPermission,
            Address,
            MemoryRegion,
            MemoryRegionType,
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
    string::ToString,
};
use ::arch::{
    cpu::pic,
    mem,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

#[cfg(feature = "whp")]
use crate::hal::platform::region_tags::LAPIC_MMIO_TAG;

#[cfg(all(feature = "pit", not(feature = "whp")))]
use crate::hal::platform::pit::Pit;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Platform {
    pub arch: Arch,
    #[cfg(all(feature = "pit", not(feature = "whp")))]
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
/// Signals the VMM that the kernel has finished booting and user-space
/// applications are about to start. The VMM uses this to enable
/// host-side services (e.g., pvclock timer) that should not run during
/// the boot phase.
///
pub fn signal_boot_complete() {
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

    // Register initrd as a kernel module.
    if initrd_size != 0 {
        let total_bytes: usize = initrd_size * mem::PAGE_SIZE;
        let image_data: &[u8] =
            unsafe { core::slice::from_raw_parts(initrd_base as *const u8, total_bytes) };

        // Detect initrd format by checking for NVMB multibinary magic.
        if image_data.len() >= multibin::MAGIC.len()
            && image_data[..multibin::MAGIC.len()] == multibin::MAGIC
        {
            info!("parse_bootinfo(): multibinary initrd detected");
            kernel_modules.extend(crate::multibin::parse(image_data, initrd_base)?);
        } else {
            // Single ELF binary with length-prefixed args after the initrd.
            info!("parse_bootinfo(): single-binary initrd detected");
            let initrd_cmdline_len_base: usize = initrd_base + total_bytes;
            let initrd_cmdline_base: usize = initrd_cmdline_len_base + core::mem::size_of::<u8>();

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
                initrd_base, total_bytes, cmdline_len, cmdline
            );

            let module: KernelModule = KernelModule::new(
                PhysicalAddress::from_raw_value(initrd_base)?,
                total_bytes,
                cmdline.to_string(),
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

        let ramfs_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new(
            RAMFS_REGION_NAME,
            PageAligned::from_raw_value(ramfs_base)?,
            ramfs_size,
            MemoryRegionType::Mmio,
            AccessPermission::RDWR,
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

    // Register MicroVM control registers.
    let scratch_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new(
        "microvm-ctrl-registers",
        PageAligned::from_raw_value(::config::microvm::DEFAULT_MICROVM_CTRL_BASE)?,
        mem::PAGE_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDONLY,
    )?;
    ioaddresses.register(MICROVM_CTRL_MMIO_TAG, scratch_region.clone())?;
    mmio_regions.push_back(scratch_region);

    // Register pvclock page so the kernel can read TSC calibration data.
    let pvclock_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new(
        "pvclock-page",
        PageAligned::from_raw_value(::config::microvm::DEFAULT_PVCLOCK_PAGE)?,
        mem::PAGE_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDONLY,
    )?;
    ioaddresses.register(PVCLOCK_MMIO_TAG, pvclock_region.clone())?;
    mmio_regions.push_back(pvclock_region);

    // Register the LAPIC MMIO page only for the WHP microvm backend.
    // The guest uses this page to enable LAPIC software delivery and to
    // acknowledge interrupts through the WHP LAPIC emulator.
    #[cfg(feature = "whp")]
    {
        let lapic_region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new(
            "lapic-registers",
            PageAligned::from_raw_value(::config::microvm::DEFAULT_LAPIC_BASE)?,
            mem::PAGE_SIZE,
            MemoryRegionType::Mmio,
            AccessPermission::RDWR,
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

    let arch = x86::init(ioports, ioaddresses, madt)?;

    Ok(Platform {
        arch,
        #[cfg(all(feature = "pit", not(feature = "whp")))]
        _pit: register_pit(ioports)?,
    })
}
