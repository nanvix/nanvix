// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[path = "../../../shared/cpu/interrupt/controller.rs"]
mod controller;
#[path = "../../../shared/cpu/interrupt/ioapic.rs"]
mod ioapic;
#[path = "../../../shared/cpu/interrupt/map.rs"]
mod map;
#[path = "../../../shared/cpu/interrupt/number.rs"]
mod number;
#[path = "../../../shared/cpu/interrupt/pic.rs"]
mod pic;
#[path = "../../../shared/cpu/interrupt/xapic.rs"]
mod xapic;

//==================================================================================================
// Imports
//==================================================================================================

use self::{
    ioapic::UninitIoapic,
    map::InterruptMap,
    pic::UninitPic,
    xapic::UninitXapic,
};
use crate::hal::{
    arch::x86::{
        cpu::idt,
        mem::gdt::SegmentSelector,
    },
    io::{
        IoMemoryAllocator,
        IoMemoryRegion,
        IoPortAllocator,
    },
    mem::Address,
    platform::{
        madt::MadtInfo,
        region_tags::{
            IOAPIC_MMIO_TAG,
            LAPIC_MMIO_TAG,
        },
    },
};
use ::alloc::collections::LinkedList;
use ::arch::cpu::{
    eflags::{
        self,
        EflagsRegister,
    },
    madt::{
        MadtEntryIoApicSourceOverride,
        MadtEntryLocalApic,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use controller::{
    InterruptController,
    InterruptHandler,
};
pub use number::InterruptNumber;
pub use xapic::{
    Xapic,
    XapicTimer,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Forges a stack frame that is suitable for returning from an interrupt with privilege transition.
///
/// # Parameters
///
/// - `kernel_stack_top`: Pointer to the top of the kernel stack.
/// - `user_stack_top`: Top address of user stack.
/// - `user_fn`: User function.
/// - `arg0`: First argument passed in to `user_fn`.
/// - `arg1`: Second argument passed in to `user_fn`.
/// - `kernel_func`: Kernel function.
/// - `enable_interrupts`: Enable interrupts?
///
/// # Returns
///
/// A pointer to the forged stack frame.
///
/// # Safety
///
/// Behavior is undefined if any of the following conditions are violated:
///
/// - `kernel_stack_top` must point to a valid location in memory.
///
pub unsafe fn forge_user_stack(
    kernel_stack_top: *mut u8,
    user_stack_top: usize,
    user_fn: usize,
    arg0: usize,
    arg1: usize,
    kernel_func: usize,
    enable_interrupts: bool,
) -> *mut u8 {
    // Get pointer to kernel stack.
    let mut kstackp: *mut u32 = kernel_stack_top as *mut u32;

    // Push User Function on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = arg0 as u32;

    // Push User DS on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = SegmentSelector::UserData as u32;

    // Push User ESP on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = (user_stack_top) as u32;

    // Push EFLAGS on the kernel stack.
    let mut eflags: EflagsRegister = eflags::EflagsRegister::default();
    eflags.interrupt = if enable_interrupts {
        eflags::InterruptFlag::Set
    } else {
        eflags::InterruptFlag::Clear
    };
    kstackp = kstackp.offset(-1);
    *kstackp = eflags.into_raw_value();

    // Push User CS on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = SegmentSelector::UserCode as u32;

    // Push User EIP on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = user_fn as u32;

    // Push the segment selector for the users-space thread data area on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = SegmentSelector::UserThreadDataArea as u32;

    // Push first argument to user function on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = arg0 as u32;

    // Push second argument to user function on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = arg1 as u32;

    // Push Kernel EIP on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = kernel_func as u32;

    kstackp as *mut u8
}

fn build_interrupt_map(madt: &MadtInfo) -> InterruptMap {
    let interrupt_override: LinkedList<&MadtEntryIoApicSourceOverride> =
        madt.get_ioapic_source_override();
    let mut intmap: InterruptMap = InterruptMap::new();

    // Build the interrupt map.
    for entry in interrupt_override {
        intmap.remap(entry.source, entry.global_sys_int as u8);
    }

    intmap
}

/// Initializes the interrupt controller.
///
/// If the platform registered a LAPIC MMIO region (via [`LAPIC_MMIO_TAG`]) and no MADT xAPIC
/// consumed it, this function creates an [`XapicTimer`] (calibrated via PIT channel 2) and
/// returns it alongside the interrupt controller. The controller receives an EOI handle extracted
/// from the timer.
pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: &Option<MadtInfo>,
) -> Result<(InterruptController, Option<XapicTimer>), Error> {
    info!("initializing interrupt controller...");
    match madt {
        // MADT is present.
        Some(madt) => {
            info!("retrieving information from madt");

            // Check if the 8259 PIC is present.
            let pic: Option<UninitPic> = match madt.has_8259_pic() {
                true => {
                    info!("8259 pic found");
                    Some(UninitPic::new(ioports, idt::INT_OFF)?)
                },
                false => {
                    info!("8259 pic not found");
                    None
                },
            };

            // Check if the I/O APIC is present.
            let ioapic: Option<UninitIoapic> = match madt.get_ioapic_info() {
                Some(ioapic_info) => {
                    info!("ioapic found");

                    let id: u8 = ioapic_info.io_apic_id;
                    let addr: u32 = ioapic_info.io_apic_addr;
                    let gsi: u32 = ioapic_info.global_sys_int_base;
                    let base: IoMemoryRegion = ioaddresses.allocate(IOAPIC_MMIO_TAG)?;

                    // Ensure that the allocated region matches the parsed MADT information.
                    if base.base().into_raw_value() != addr as usize {
                        let reason: &str = "ioapic region does not match madt";
                        error!(
                            "{reason} (expected={:#x}, found={:#x})",
                            addr,
                            base.base().into_raw_value()
                        );
                        return Err(Error::new(ErrorCode::InvalidArgument, reason));
                    }

                    Some(UninitIoapic::new(idt::INT_OFF, id, base, gsi))
                },
                None => {
                    info!("ioapic not found");
                    None
                },
            };

            // Check if local APIC is present.
            let xapic: Option<UninitXapic> = match madt.get_lapic_info() {
                Some(local_apic_info) => {
                    info!("xapic found");

                    if (local_apic_info.flags & MadtEntryLocalApic::ENABLED) != 0 {
                        if (local_apic_info.flags & MadtEntryLocalApic::ONLINE_CAPABLE) == 0 {
                            info!("cpu is enabled")
                        } else {
                            // This should not happen if MADT is consistent to the spec.
                            unreachable!("xapic is malfunctioning")
                        }
                    } else if (local_apic_info.flags & MadtEntryLocalApic::ONLINE_CAPABLE) != 0 {
                        info!("cpu is online capable")
                    } else {
                        info!("cpu is disabled")
                    }

                    // TODO: remove the following assert when we handle multiple local APICs.
                    // CPU 0 must be enabled.
                    assert!(
                        local_apic_info.apic_id == 0
                            || (local_apic_info.flags & MadtEntryLocalApic::ENABLED) != 0
                    );

                    let base: IoMemoryRegion = ioaddresses.allocate(LAPIC_MMIO_TAG)?;

                    // Ensure that the allocated region matches the parsed MADT information.
                    if base.base().into_raw_value() != madt.local_apic_addr as usize {
                        let reason: &str = "lapic region does not match madt";
                        error!(
                            "{reason} (expected={:#x}, found={:#x})",
                            madt.local_apic_addr,
                            base.base().into_raw_value()
                        );
                        return Err(Error::new(ErrorCode::InvalidArgument, reason));
                    }

                    Some(UninitXapic::new(local_apic_info.apic_id, base))
                },
                None => {
                    info!("xapic not found");
                    None
                },
            };

            let intmap: InterruptMap = build_interrupt_map(madt);

            let (eoi_xapic, xapic_timer) = if xapic.is_some() {
                // MADT xAPIC path already consumed the LAPIC MMIO region; skip timer probe.
                (None, None)
            } else {
                // Try to create an xAPIC timer from a platform-registered LAPIC MMIO region.
                // This succeeds only when the MADT xAPIC path did not consume the region
                // (e.g., no xAPIC in MADT but platform registered the region for timer use).
                try_init_xapic_timer(ioports, ioaddresses)?
            };

            // When xAPIC-only mode is active (eoi_xapic is Some), the PIC, xAPIC,
            // and IOAPIC must not be passed to the controller — it handles timer
            // delivery and EOI entirely through the LAPIC.
            let controller = if eoi_xapic.is_some() {
                InterruptController::new(None, None, None, intmap, eoi_xapic)?
            } else {
                InterruptController::new(pic, xapic, ioapic, intmap, eoi_xapic)?
            };
            Ok((controller, xapic_timer))
        },

        // MADT is not present.
        None => {
            info!("madt not present, falling back to 8259 pic");

            match UninitPic::new(ioports, idt::INT_OFF) {
                Ok(pic) => {
                    let intmap: InterruptMap = InterruptMap::new();
                    // Try to create an xAPIC timer from a platform-registered LAPIC MMIO region.
                    let (eoi_xapic, xapic_timer) = try_init_xapic_timer(ioports, ioaddresses)?;
                    // When xAPIC-only mode is active (eoi_xapic is Some), skip the
                    // PIC — the LAPIC handles timer delivery and EOI entirely.
                    let controller = if eoi_xapic.is_some() {
                        InterruptController::new(None, None, None, intmap, eoi_xapic)?
                    } else {
                        InterruptController::new(Some(pic), None, None, intmap, eoi_xapic)?
                    };
                    Ok((controller, xapic_timer))
                },
                Err(e) => {
                    warn!("failed to initialize 8259 pic (error={:?})", e);
                    let controller =
                        InterruptController::new(None, None, None, InterruptMap::new(), None)?;
                    Ok((controller, None))
                },
            }
        },
    }
}

/// Tries to allocate a LAPIC MMIO region for xAPIC timer use. If the platform registered
/// [`LAPIC_MMIO_TAG`], this creates an [`UninitXapicTimer`], calibrates it via PIT, and returns
/// the initialized [`XapicTimer`] along with an EOI handle for the interrupt controller.
///
/// Returns `(None, None)` if the LAPIC MMIO region is not available or the current configuration
/// does not support PIT-based LAPIC calibration.
fn try_init_xapic_timer(
    _ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
) -> Result<(Option<Xapic>, Option<XapicTimer>), Error> {
    // For the WHP + PIT + microvm configuration, we attempt to allocate the LAPIC MMIO
    // region and initialize an xAPIC timer. In other configurations, we avoid touching
    // the allocator so we don't emit spurious error logs when LAPIC_MMIO_TAG is absent.
    #[cfg(all(feature = "pit", feature = "microvm", feature = "whp"))]
    {
        use self::xapic::UninitXapicTimer;

        let lapic_region: IoMemoryRegion = match ioaddresses.allocate(LAPIC_MMIO_TAG) {
            Ok(region) => region,
            Err(_) => return Ok((None, None)),
        };

        info!("lapic mmio region available for xapic timer");

        let uninit_timer: UninitXapicTimer = UninitXapicTimer::new(lapic_region);

        // Calibrate the LAPIC timer (RDTSC-based when TSC frequency is available,
        // PIT channel 2 fallback otherwise) and program it in periodic mode.
        use crate::hal::platform::pit::Pit;

        const LAPIC_CALIBRATION_MS: u32 = 1;

        let mut pit: Pit = Pit::new(_ioports, ::config::kernel::TIMER_FREQ)?;
        let xapic_timer: XapicTimer = uninit_timer.init(&mut pit, LAPIC_CALIBRATION_MS);

        // SAFETY: Single-core system; the Xapic handle shares the same LAPIC MMIO page.
        let eoi_xapic: Xapic = unsafe { xapic_timer.create_eoi_handle() };
        Ok((Some(eoi_xapic), Some(xapic_timer)))
    }

    #[cfg(not(all(feature = "pit", feature = "microvm", feature = "whp")))]
    {
        // In configurations without PIT-based LAPIC calibration support, we cannot
        // initialize the xAPIC timer. Avoid probing the LAPIC MMIO region so we don't
        // trigger allocator error logging on platforms that don't register LAPIC_MMIO_TAG.
        let _ = ioaddresses;
        Ok((None, None))
    }
}
