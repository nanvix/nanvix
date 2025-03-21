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
use ::sys::{
    arch::{
        cpu::pic,
        mem,
    },
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
    ::sys::arch::io::out8(::config::microvm::DEFAULT_STDOUT_PORT, b);
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
    use core::hint;

    #[allow(clippy::unit_arg)]
    hint::black_box(::sys::arch::io::out32(::config::microvm::DEFAULT_STDOUT_PORT, addr as u32));
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
    use core::hint;

    #[allow(clippy::unit_arg)]
    hint::black_box(::sys::arch::io::out32(::config::microvm::DEFAULT_STDIN_PORT, addr as u32))
}

///
/// # Description
///
/// Shutdowns the machine.
///
/// # Return
///
/// This function never returns.
///
pub fn shutdown() -> ! {
    unsafe {
        ::sys::arch::io::out16(
            ::config::microvm::DEFAULT_VMM_PORT,
            ::config::microvm::DEFAULT_VMM_SHUTDOWN_CMD,
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
        error!("parse_bootinfo(): magic={:#010x}, info={:#010x} (error={})", magic, info, reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    trace!("parse_bootinfo(): magic={:#010x}, info={:#010x}", magic, info);

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
                error!("parse_bootinfo(): invalid UTF-8 in command line");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        info!(
            "parse_bootinfo(): initrd_base={:#010x}, initrd_size={:#010x}, cmdline_len={:?}, \
             cmdline={:?}",
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

#[cfg(feature = "pic")]
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

    ioports.register_read_write(::sys::arch::cpu::pit::PIT_CTRL)?;
    ioports.register_read_write(::sys::arch::cpu::pit::PIT_DATA)?;

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
    #[cfg(feature = "pic")]
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

    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
        #[cfg(feature = "pit")]
        _pit: register_pit(ioports)?,
    })
}
