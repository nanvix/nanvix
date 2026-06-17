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
    let mut kstackp: *mut u64 = kernel_stack_top as *mut u64;

    // Push arg0 above the iretq frame. On x86_64 the iretq SS slot is 8 bytes, and this
    // value occupies the slot that the x86 (32-bit) version uses for the user function
    // argument passed via the stack. It is consumed by the caller of forge_user_stack when
    // it reads the forged stack layout, mirroring the x86 convention.
    kstackp = kstackp.offset(-1);
    *kstackp = arg0 as u64;

    // Push User SS on the kernel stack (iretq frame).
    kstackp = kstackp.offset(-1);
    *kstackp = SegmentSelector::UserData as u64;

    // Push User RSP on the kernel stack (iretq frame).
    //
    // The System V AMD64 ABI requires that, at the point control is transferred to a
    // function's entry, `(%rsp + 8)` is a multiple of 16 (i.e. `%rsp` is congruent to 8
    // modulo 16), reflecting the return address a `call` would have pushed. Thread entry
    // points are dispatched directly through this forged `iretq` frame without a crt0
    // prologue to realign the stack, so the kernel must hand them an ABI-aligned stack.
    // The user stack top is page-aligned (congruent to 0 modulo 16), so bias it down to
    // the nearest `8 (mod 16)` boundary. Omitting this makes compiler-emitted aligned SSE
    // stores (e.g. `movaps`) on the entry frame fault with a #GP.
    let user_rsp: usize = (user_stack_top & !0xf).wrapping_sub(8);
    kstackp = kstackp.offset(-1);
    *kstackp = user_rsp as u64;

    // Push RFLAGS on the kernel stack (iretq frame).
    let mut eflags: EflagsRegister = eflags::EflagsRegister::default();
    eflags.interrupt = if enable_interrupts {
        eflags::InterruptFlag::Set
    } else {
        eflags::InterruptFlag::Clear
    };
    kstackp = kstackp.offset(-1);
    *kstackp = eflags.into_raw_value() as u64;

    // Push User CS on the kernel stack (iretq frame).
    kstackp = kstackp.offset(-1);
    *kstackp = SegmentSelector::UserCode as u64;

    // Push User RIP on the kernel stack (iretq frame).
    kstackp = kstackp.offset(-1);
    *kstackp = user_fn as u64;

    // Push first argument to user function on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = arg0 as u64;

    // Push second argument to user function on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = arg1 as u64;

    // Push Kernel RIP on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = kernel_func as u64;

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
pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: &Option<MadtInfo>,
) -> Result<InterruptController, Error> {
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
            InterruptController::new(pic, xapic, ioapic, intmap)
        },

        // MADT is not present.
        None => {
            info!("madt not present, falling back to 8259 pic");
            match UninitPic::new(ioports, idt::INT_OFF) {
                Ok(pic) => {
                    let intmap: InterruptMap = InterruptMap::new();
                    Ok(InterruptController::new(Some(pic), None, None, intmap)?)
                },
                Err(e) => {
                    warn!("failed to initialize 8259 pic (error={:?})", e);
                    Ok(InterruptController::new(None, None, None, InterruptMap::new())?)
                },
            }
        },
    }
}
