// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod controller;
mod map;
mod number;
mod pic;

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    arch::x86_64::{
        cpu::idt,
        mem::gdt::SegmentSelector,
    },
    io::{
        IoMemoryAllocator,
        IoPortAllocator,
    },
    platform::madt::MadtInfo,
};
use self::{
    map::InterruptMap,
    pic::UninitPic,
};
use ::sys::error::Error;

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
/// Forges a stack frame that is suitable for returning from an interrupt with privilege transition
/// (IRETQ) on x86_64.
///
/// The IRETQ stack frame on x86_64 is (from top of stack, low address to high):
///   RIP, CS, RFLAGS, RSP, SS
///
/// # Parameters
///
/// - `kernel_stack_top`: Pointer to the top of the kernel stack.
/// - `user_stack_top`: Top address of user stack.
/// - `user_fn`: User function.
/// - `arg0`: First argument passed in to `user_fn` (placed in RDI per System V AMD64 ABI).
/// - `arg1`: Second argument passed in to `user_fn` (placed in RSI per System V AMD64 ABI).
/// - `kernel_func`: Kernel function.
/// - `enable_interrupts`: Enable interrupts?
///
/// # Returns
///
/// A pointer to the forged stack frame.
///
/// # Safety
///
/// Behavior is undefined if `kernel_stack_top` does not point to a valid location in memory.
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
    let mut kstackp: *mut u64 = kernel_stack_top as *mut u64;

    // Build IRETQ frame (pushed in reverse order, so SS is at highest address).

    // Push User SS on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = SegmentSelector::UserData as u64;

    // Push User RSP on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = user_stack_top as u64;

    // Push RFLAGS on the kernel stack.
    // Bit 9 = IF (Interrupt Flag), Bit 1 = reserved (always 1).
    let rflags: u64 = if enable_interrupts {
        (1 << 9) | (1 << 1) // IF set + reserved bit 1
    } else {
        1 << 1 // reserved bit 1 only
    };
    kstackp = kstackp.offset(-1);
    *kstackp = rflags;

    // Push User CS on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = SegmentSelector::UserCode as u64;

    // Push User RIP on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = user_fn as u64;

    // Push second argument (RSI) on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = arg1 as u64;

    // Push first argument (RDI) on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = arg0 as u64;

    // Push Kernel function address (return address) on the kernel stack.
    kstackp = kstackp.offset(-1);
    *kstackp = kernel_func as u64;

    kstackp as *mut u8
}

/// Initializes the interrupt controller.
pub fn init(
    ioports: &mut IoPortAllocator,
    _ioaddresses: &mut IoMemoryAllocator,
    _madt: &Option<MadtInfo>,
) -> Result<InterruptController, Error> {
    info!("initializing interrupt controller...");

    let pic: UninitPic = UninitPic::new(ioports, idt::INT_OFF)?;
    let intmap: InterruptMap = InterruptMap::new();
    InterruptController::new(pic, intmap)
}
