// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub(crate) mod peb;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "pit")]
use crate::hal::platform::pit::Pit;
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
            peb::ProcessEnvironmentBlock,
        },
    },
    kmod::KernelModule,
};
use ::alloc::{
    collections::linked_list::LinkedList,
    string::{
        String,
        ToString,
    },
};
use ::arch::{
    mem,
    mem::PAGE_ALIGNMENT,
};
use ::hyperlight_common::mem::HyperlightPEB;
use ::sys::{
    config::memory_layout,
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
/// Writes the string `s` to the platform's standard debug device.
///
/// # Parameters
///
/// - `s`: String to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
///
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn puts(message: &str) {
    let _ = ProcessEnvironmentBlock::puts(message);
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

    PERF_VMBUS_WRITE.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let data = core::slice::from_raw_parts(addr, config::kernel::IPC_MESSAGE_SIZE);
    let _ = ProcessEnvironmentBlock::vmbus_write(data);
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

    PERF_VMBUS_READ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let data = core::slice::from_raw_parts_mut(addr, config::kernel::IPC_MESSAGE_SIZE);
    let bytes = ProcessEnvironmentBlock::vmbus_read();
    if let Ok(bytes) = bytes {
        data.copy_from_slice(&bytes);
    }
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
pub fn shutdown(_status: usize) -> ! {
    ::hyperlight_guest::exit::abort_with_code(&[::config::hyperlight::DEFAULT_VMM_SHUTDOWN_CMD]);
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
    trace!("{magic:?}, {info:?}");

    extern "C" {
        static __KERNEL_END: u8;
    }

    let peb_base: usize =
        unsafe { ::sys::mm::align_up(&__KERNEL_END as *const u8 as usize, PAGE_ALIGNMENT) };
    let peb_ptr: *mut HyperlightPEB = peb_base as *mut HyperlightPEB;

    unsafe {
        ProcessEnvironmentBlock::init(peb_ptr)?;
        ProcessEnvironmentBlock::set_guest_function_dispatch_ptr(0xdeadbeef)?;
    };

    // Read actual size and relocate only that amount
    let (initrd_base, initrd_size, (cmdline_len, cmdline)) = unsafe {
        let current_data_start = (*peb_ptr).init_data.ptr as usize;
        let total_size = (*peb_ptr).init_data.size as usize;

        let (initrd_base, actual_initrd_size, initrd_cmdline) =
            parse_initrd_image(current_data_start, total_size)?;

        (initrd_base, actual_initrd_size, initrd_cmdline)
    };

    let mut kernel_modules: LinkedList<KernelModule> = LinkedList::new();

    // Register initrd as a kernel module.

    info!(
        "initrd_base={:#010x}, initrd_size={:#010x}, cmdline_len={:?}, cmdline={:?}",
        initrd_base,
        initrd_size,
        cmdline_len,
        cmdline.as_str()
    );

    // Add kernel module to the list of kernel modules.
    let module: KernelModule =
        KernelModule::new(PhysicalAddress::from_raw_value(initrd_base)?, initrd_size, cmdline);
    kernel_modules.push_back(module);

    Ok(BootInfo::new(None, None, LinkedList::new(), LinkedList::new(), kernel_modules))
}

#[cfg(feature = "pic")]
fn register_pic_ioports(ioports: &mut IoPortAllocator) -> Result<(), Error> {
    // Register I/O ports for 8259 PIC.
    ioports.register_read_write(::arch::cpu::pic::PIC_CTRL_MASTER as u16)?;
    ioports.register_read_write(::arch::cpu::pic::PIC_DATA_MASTER as u16)?;
    ioports.register_read_write(::arch::cpu::pic::PIC_CTRL_SLAVE as u16)?;
    ioports.register_read_write(::arch::cpu::pic::PIC_DATA_SLAVE as u16)?;
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
    #[cfg(feature = "pic")]
    register_pic_ioports(ioports)?;

    extern "C" {
        static __KERNEL_END: u8;
    }
    // Register PEB structure.
    let peb_base: usize =
        ::sys::mm::align_up(unsafe { &__KERNEL_END } as *const u8 as usize, PAGE_ALIGNMENT);
    const PEB_SIZE: usize = mem::PAGE_SIZE;
    let peb: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "peb",
        VirtualAddress::from_raw_value(peb_base),
        PEB_SIZE,
        MemoryRegionType::Reserved,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(peb);

    // Register host function definitions
    let host_function_definitions_base: usize = peb_base + PEB_SIZE;
    const HOST_FUNCTION_DEFINITIONS_SIZE: usize = mem::PAGE_SIZE;
    let host_function_definitions: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "host function definitions",
        VirtualAddress::from_raw_value(host_function_definitions_base),
        HOST_FUNCTION_DEFINITIONS_SIZE,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(host_function_definitions);

    // Register input data buffer.
    let input_data_base: usize = host_function_definitions_base + HOST_FUNCTION_DEFINITIONS_SIZE;
    const INPUT_DATA_BUFFER_SIZE: usize = 4 * mem::PAGE_SIZE;
    let input_data_buffer: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "input data buffer",
        VirtualAddress::from_raw_value(input_data_base),
        INPUT_DATA_BUFFER_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(input_data_buffer);

    // Register output data buffer.
    let output_data_base: usize = input_data_base + INPUT_DATA_BUFFER_SIZE;
    const OUTPUT_DATA_BUFFER_SIZE: usize = 4 * mem::PAGE_SIZE;
    let output_data_buffer: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "output data buffer",
        VirtualAddress::from_raw_value(output_data_base),
        OUTPUT_DATA_BUFFER_SIZE,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(output_data_buffer);

    // Register reserved area for heap padding.
    let heap_padding_base: usize = output_data_base + OUTPUT_DATA_BUFFER_SIZE;
    debug!("heap_padding_base={:#010x}", heap_padding_base);
    let heap_padding_size: usize = memory_layout::KPOOL_BASE.into_raw_value() - heap_padding_base;
    let heap_padding: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "heap padding",
        VirtualAddress::from_raw_value(heap_padding_base),
        heap_padding_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(heap_padding);

    // Register kpool guard page.
    let kpool_guard_base: usize =
        memory_layout::KPOOL_BASE.into_raw_value() + config::kernel::KPOOL_SIZE;
    let kpool_guard_size: usize = mem::PAGE_SIZE;
    let kpool_guard: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "kpool guard",
        VirtualAddress::from_raw_value(kpool_guard_base),
        kpool_guard_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(kpool_guard);

    // Register hyperlight guest user stack.
    let guest_user_stack_base: usize = kpool_guard_base + kpool_guard_size;
    let guest_user_stack_size: usize = mem::PAGE_SIZE;
    let guest_user_stack: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "guest user stack",
        VirtualAddress::from_raw_value(guest_user_stack_base),
        guest_user_stack_size,
        MemoryRegionType::Reserved,
        AccessPermission::RDONLY,
    )?;
    memory_regions.push_back(guest_user_stack);

    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
        #[cfg(feature = "pit")]
        _pit: register_pit(ioports)?,
    })
}

///
/// # Description
///
/// Parses the initrd image and relocates it to the default initrd base address if needed.
///
/// # Parameters
///
/// - `init_data_start`: Address where the init data blob begins.
/// - `total_allocation_size`: Total allocation size for the init data blob.
///
/// # Return Value
///
/// On success, this function returns a tuple containing:
/// - The base address of the initrd payload.
/// - The actual size of the initrd payload.
/// - A tuple with the length of the command line and the command line string.
///
/// Otherwise, it returns an error.
///
/// # Safety
///
/// This function is unsafe because it performs unchecked pointer arithmetic and dereferences raw
/// pointers. Callers must ensure that the provided memory ranges are valid and mapped.
///
unsafe fn parse_initrd_image(
    init_data_start: usize,
    total_allocation_size: usize,
) -> Result<(usize, usize, (u8, String)), Error> {
    // Check if allocation is too small to hold the initrd header.
    if total_allocation_size < ::config::hyperlight::INITRD_SIZE_BYTES {
        let reason: &str = "insufficient initrd allocation size";
        error!("parse_initrd_image(): {reason} (total_allocation_size={total_allocation_size})");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    // Read actual size and relocate only that amount
    let initrd_header: &[u8] = core::slice::from_raw_parts(
        init_data_start as *const u8,
        ::config::hyperlight::INITRD_SIZE_BYTES,
    );
    let actual_initrd_size: usize = u64::from_le_bytes([
        initrd_header[0],
        initrd_header[1],
        initrd_header[2],
        initrd_header[3],
        initrd_header[4],
        initrd_header[5],
        initrd_header[6],
        initrd_header[7],
    ]) as usize;

    // Compute offsets and check for overflows.
    let payload_offset: usize = ::config::hyperlight::INITRD_SIZE_BYTES;
    let current_initrd_start: usize = match init_data_start.checked_add(payload_offset) {
        Some(value) => value,
        None => {
            let reason: &str = "initrd payload address overflow";
            error!("parse_initrd_image(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Check if actual size is valid.
    let required_allocation: usize = match payload_offset.checked_add(actual_initrd_size) {
        Some(value) => value,
        None => {
            let reason: &str = "initrd required allocation size overflow";
            error!("parse_initrd_image(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Check if allocation is too small to hold the actual initrd payload.
    if total_allocation_size < required_allocation {
        let reason: &str = "initrd payload truncated";
        error!(
            "parse_initrd_image(): {reason} (total_allocation_size={total_allocation_size}, \
             required_allocation={required_allocation})"
        );
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    debug!(
        "parse_initrd_image(): initrd found at 0x{current_initrd_start:08x}, actual size: \
         {actual_initrd_size} bytes (total allocation: {total_allocation_size} bytes)"
    );

    // Read initrd command line.
    let initrd_cmdline: (u8, String) =
        read_initrd_cmdline(current_initrd_start, actual_initrd_size, total_allocation_size)?;

    // Relocate initrd to default base address if needed.
    let initrd_base: usize = if current_initrd_start != ::config::hyperlight::DEFAULT_INITRD_BASE {
        let src_ptr: *const u8 = current_initrd_start as *const u8;
        let dst_ptr: *mut u8 = ::config::hyperlight::DEFAULT_INITRD_BASE as *mut u8;
        core::ptr::copy(src_ptr, dst_ptr, actual_initrd_size);

        debug!(
            "parse_initrd_image(): initrd relocated from {current_initrd_start:#010x} to {:#010x}",
            ::config::hyperlight::DEFAULT_INITRD_BASE
        );
        ::config::hyperlight::DEFAULT_INITRD_BASE
    } else {
        current_initrd_start
    };

    Ok((initrd_base, actual_initrd_size, initrd_cmdline))
}

///
/// # Description
///
/// Reads the initrd command line from the initrd payload if present.
///
/// # Parameters
///
/// - `initrd_start`: Address where the initrd payload begins.
/// - `initrd_size`: Size of the initrd payload.
/// - `total_allocation_size`: Total allocation size that includes the initrd and any extra data.
///
/// # Returns
///
/// On success, this function returns a tuple containing the length of the command line and the
/// command line string. Otherwise, it returns an error.
///
/// # Safety
///
/// This function is unsafe because it performs unchecked pointer arithmetic and dereferences raw
/// pointers. Callers must ensure that the provided ranges are valid and mapped.
///
unsafe fn read_initrd_cmdline(
    initrd_start: usize,
    initrd_size: usize,
    total_allocation_size: usize,
) -> Result<(u8, String), Error> {
    let args_section_size: usize =
        total_allocation_size.saturating_sub(::config::hyperlight::INITRD_SIZE_BYTES + initrd_size);

    // Check if initrd arguments section is missing.
    if args_section_size == 0 {
        let reason: &str = "initrd arguments section missing";
        error!("read_initrd_cmdline(): {reason}");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    // Compute offset to arguments length byte and check for overflows.
    let args_len_offset: usize = match initrd_start.checked_add(initrd_size) {
        Some(offset) => offset,
        None => {
            let reason: &str = "initrd arguments address overflow";
            error!("read_initrd_cmdline(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Compute offset to arguments payload and check for overflows.
    let args_bytes_offset: usize = match args_len_offset.checked_add(1) {
        Some(offset) => offset,
        None => {
            let reason: &str = "initrd arguments payload address overflow";
            error!("read_initrd_cmdline(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    // Check if arguments length byte is missing.
    if args_section_size < 1 {
        let reason: &str = "initrd arguments length byte missing";
        error!("read_initrd_cmdline(): {reason}");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    let args_len: u8 = *(args_len_offset as *const u8);
    let args_payload_size: usize = usize::from(args_len);

    if args_section_size < 1 + args_payload_size {
        let reason: &str = "initrd arguments truncated";
        error!(
            "read_initrd_cmdline(): {reason} (args_section_size={args_section_size}, \
             args_len={args_len})"
        );
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    let args_bytes: &[u8] =
        core::slice::from_raw_parts(args_bytes_offset as *const u8, args_payload_size);

    // Convert arguments to UTF-8 string and check for errors.
    let args_str: &str = match core::str::from_utf8(args_bytes) {
        Ok(value) => value,
        Err(_) => {
            let reason: &str = "invalid UTF-8 in initrd arguments";
            error!("read_initrd_cmdline(): {reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        },
    };

    Ok((args_len, args_str.to_string()))
}
