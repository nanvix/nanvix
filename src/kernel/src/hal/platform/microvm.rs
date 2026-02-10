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
            region_tags::RAMFS_MMIO_TAG,
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

//==================================================================================================
// Static Variables
//==================================================================================================

use core::sync::atomic::{
    AtomicUsize,
    Ordering,
};

/// Base address of the TX ring buffer region (0 when ring buffers are not active).
static TX_RING_BASE: AtomicUsize = AtomicUsize::new(0);

/// Base address of the RX ring buffer region (0 when ring buffers are not active).
static RX_RING_BASE: AtomicUsize = AtomicUsize::new(0);

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

    PERF_VMBUS_WRITE.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let tx_base: usize = TX_RING_BASE.load(Ordering::Relaxed);
    if tx_base != 0 {
        vmbus_write_ring(tx_base, addr);
    } else {
        use core::hint;
        #[allow(clippy::unit_arg)]
        hint::black_box(::arch::io::out32(::config::microvm::DEFAULT_STDOUT_PORT, addr as u32));
    }
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
/// Returns whether the MMIO ring buffer path is active.
///
/// # Returns
///
/// `true` when the VMM has provisioned TX and RX ring buffers and the guest has discovered them
/// during platform initialization. `false` when the legacy PMIO path should be used.
///
#[cfg(feature = "stdio")]
pub fn is_ring_buffer_active() -> bool {
    TX_RING_BASE.load(Ordering::Relaxed) != 0
}

///
/// # Description
///
/// Writes a message into the TX ring buffer and rings the doorbell to notify the VMM.
///
/// The caller must ensure that the TX ring buffer is active (`TX_RING_BASE != 0`). This function
/// spins if the ring is full, waiting for the VMM consumer to advance the tail.
///
/// # Parameters
///
/// - `tx_base`: Guest-physical (identity-mapped) base address of the TX ring.
/// - `addr`: Pointer to the message bytes to enqueue (`IPC_MESSAGE_SIZE` bytes).
///
/// # Safety
///
/// The caller must guarantee that `tx_base` points to a valid, mapped ring buffer region and that
/// `addr` points to at least `IPC_MESSAGE_SIZE` readable bytes.
///
#[cfg(feature = "stdio")]
unsafe fn vmbus_write_ring(tx_base: usize, addr: *const u8) {
    let header_ptr: *mut u32 = tx_base as *mut u32;

    // Read head (we are producer, we own head).
    let head: u32 = core::ptr::read_volatile(header_ptr);
    // Read capacity.
    let capacity: u32 = core::ptr::read_volatile(header_ptr.add(2));

    // Spin-wait until there is free space in the ring.
    loop {
        let tail: u32 = core::ptr::read_volatile(header_ptr.add(1));
        let used: u32 = head.wrapping_sub(tail);
        if used < capacity {
            break;
        }
        core::hint::spin_loop();
    }

    // Compute slot address.
    let slot_index: usize = (head % capacity) as usize;
    let slot_addr: usize = tx_base
        + ::config::microvm::RING_DATA_OFFSET
        + slot_index * ::config::kernel::IPC_MESSAGE_SIZE;

    // Copy message into the slot.
    core::ptr::copy_nonoverlapping(addr, slot_addr as *mut u8, ::config::kernel::IPC_MESSAGE_SIZE);

    // Compiler fence: ensure message data is committed before the head update becomes visible.
    core::sync::atomic::compiler_fence(Ordering::Release);

    // Increment head.
    let new_head: u32 = head.wrapping_add(1);
    core::ptr::write_volatile(header_ptr, new_head);

    // Ring the doorbell to notify the VMM.
    #[allow(clippy::unit_arg)]
    core::hint::black_box(::arch::io::out32(
        ::config::microvm::DEFAULT_STDOUT_PORT,
        ::config::microvm::DOORBELL_VALUE,
    ));
}

///
/// # Description
///
/// Attempts to read a message from the RX ring buffer.
///
/// If the ring contains at least one pending message, it is copied into the buffer at `addr` and
/// the consumer tail is advanced. Otherwise the buffer is left untouched and `false` is returned.
///
/// # Parameters
///
/// - `addr`: Pointer to a buffer of at least `IPC_MESSAGE_SIZE` bytes where the message will be
///   written.
///
/// # Returns
///
/// `true` if a message was dequeued, `false` if the ring was empty.
///
/// # Safety
///
/// The caller must guarantee that `RX_RING_BASE` has been initialized to a valid, mapped ring
/// buffer region and that `addr` points to at least `IPC_MESSAGE_SIZE` writable bytes.
///
#[cfg(feature = "stdio")]
pub unsafe fn vmbus_read_ring(addr: *mut u8) -> bool {
    use crate::PERF_VMBUS_READ;

    let rx_base: usize = RX_RING_BASE.load(Ordering::Relaxed);
    let header_ptr: *mut u32 = rx_base as *mut u32;

    // Read head (VMM producer writes this).
    let head: u32 = core::ptr::read_volatile(header_ptr);
    // Read tail (we are consumer, we own tail).
    let tail: u32 = core::ptr::read_volatile(header_ptr.add(1));

    // No data available.
    if head == tail {
        return false;
    }

    PERF_VMBUS_READ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    // Read capacity.
    let capacity: u32 = core::ptr::read_volatile(header_ptr.add(2));

    // Compiler fence: ensure we observe the head update before reading slot data.
    core::sync::atomic::compiler_fence(Ordering::Acquire);

    // Compute slot address.
    let slot_index: usize = (tail % capacity) as usize;
    let slot_addr: usize = rx_base
        + ::config::microvm::RING_DATA_OFFSET
        + slot_index * ::config::kernel::IPC_MESSAGE_SIZE;

    // Copy message from the slot.
    core::ptr::copy_nonoverlapping(
        slot_addr as *const u8,
        addr,
        ::config::kernel::IPC_MESSAGE_SIZE,
    );

    // Increment tail.
    let new_tail: u32 = tail.wrapping_add(1);
    core::ptr::write_volatile(header_ptr.add(1), new_tail);

    true
}

///
/// # Description
///
/// Reads the ring buffer base addresses and sizes from the MicroVM control registers and, when
/// present, stores them in the module-level atomics so that subsequent `vmbus_write` / `vmbus_read`
/// calls use the MMIO ring buffer path instead of PMIO.
///
fn init_ring_buffers() {
    // SAFETY: The control register page is identity-mapped and valid during platform
    // initialization.
    unsafe {
        let tx_base: usize =
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_TX_RING_BASE) as usize;
        let tx_size: usize =
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_TX_RING_SIZE) as usize;
        let rx_base: usize =
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RX_RING_BASE) as usize;
        let rx_size: usize =
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RX_RING_SIZE) as usize;

        if tx_base != 0 && tx_size != 0 && rx_base != 0 && rx_size != 0 {
            TX_RING_BASE.store(tx_base, Ordering::Relaxed);
            RX_RING_BASE.store(rx_base, Ordering::Relaxed);
            info!(
                "ring buffers active: tx_base={:#010x}, tx_size={:#x}, rx_base={:#010x}, \
                 rx_size={:#x}",
                tx_base, tx_size, rx_base, rx_size
            );
        } else {
            info!("ring buffers not available, using PMIO path");
        }
    }
}

///
/// # Description
///
/// Registers the TX and RX ring buffer regions as MMIO memory regions so that the kernel's
/// virtual memory manager maps them with read-write access.
///
/// If the VMM did not provision ring buffers (control registers read as zero), this function is a
/// no-op.
///
/// # Parameters
///
/// - `memory_regions`: List of memory regions to which the ring buffer regions will be appended.
///
/// # Returns
///
/// Upon successful completion, this function returns empty. Otherwise, it returns an error.
///
fn register_ring_buffer_regions(
    memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
) -> Result<(), Error> {
    // SAFETY: Control registers are identity-mapped and valid during platform initialization.
    let (tx_base, tx_size, rx_base, rx_size): (usize, usize, usize, usize) = unsafe {
        (
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_TX_RING_BASE) as usize,
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_TX_RING_SIZE) as usize,
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RX_RING_BASE) as usize,
            read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RX_RING_SIZE) as usize,
        )
    };

    if tx_base == 0 || tx_size == 0 || rx_base == 0 || rx_size == 0 {
        return Ok(());
    }

    // Register TX ring buffer region (guest is producer: needs read-write).
    let tx_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "vmbus-tx-ring",
        VirtualAddress::from_raw_value(tx_base),
        tx_size,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(tx_region);

    // Register RX ring buffer region (guest is consumer: needs read-write for tail updates).
    let rx_region: MemoryRegion<VirtualAddress> = MemoryRegion::new(
        "vmbus-rx-ring",
        VirtualAddress::from_raw_value(rx_base),
        rx_size,
        MemoryRegionType::Mmio,
        AccessPermission::RDWR,
    )?;
    memory_regions.push_back(rx_region);

    Ok(())
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

        let tx_ring_base_value: u32 = core::ptr::read_volatile(
            ::config::microvm::DEFAULT_MICROVM_CTRL_TX_RING_BASE as *const u32,
        );
        let tx_ring_size_value: u32 = core::ptr::read_volatile(
            ::config::microvm::DEFAULT_MICROVM_CTRL_TX_RING_SIZE as *const u32,
        );
        let rx_ring_base_value: u32 = core::ptr::read_volatile(
            ::config::microvm::DEFAULT_MICROVM_CTRL_RX_RING_BASE as *const u32,
        );
        let rx_ring_size_value: u32 = core::ptr::read_volatile(
            ::config::microvm::DEFAULT_MICROVM_CTRL_RX_RING_SIZE as *const u32,
        );

        info!(
            "microvm ctrl registers: base={:#010x}, null={:#010x}, credits={:#010x}, \
             pause={:#010x}, ramfs_base={:#010x}, ramfs_size={:#010x}, tx_ring_base={:#010x}, \
             tx_ring_size={:#010x}, rx_ring_base={:#010x}, rx_ring_size={:#010x}",
            ::config::microvm::DEFAULT_MICROVM_CTRL_BASE,
            null_value,
            credits_value,
            pause_value,
            ramfs_base_value,
            ramfs_size_value,
            tx_ring_base_value,
            tx_ring_size_value,
            rx_ring_base_value,
            rx_ring_size_value
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
            AccessPermission::RDONLY,
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
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
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
    register_ring_buffer_regions(memory_regions)?;
    init_ring_buffers();
    register_ramfs_mmio_region(ioaddresses, mmio_regions)?;

    Ok(Platform {
        arch: x86::init(ioports, ioaddresses, madt)?,
        #[cfg(feature = "pit")]
        _pit: register_pit(ioports)?,
    })
}
