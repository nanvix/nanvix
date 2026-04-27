// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{
    hal::mem::FrameAddress,
    mm::phys::frame,
};

const PTE_PRESENT: u32 = 1 << 0;
const PTE_RW: u32 = 1 << 1;
const PTE_COW: u32 = 1 << 9;
const PTE_ADDR_MASK: u32 = 0xFFFFF000;
const PF_ERR_WRITE: u32 = 1 << 1;

/// Rust CoW page fault handler called from the `_cow_pf_handler_rust`
/// assembly stub after HAL init.  Uses Nanvix's frame allocator instead
/// of the bootstrap bump allocator.
///
/// Returns 1 if the fault was a CoW fault and was resolved, 0 otherwise.
///
/// # Safety
///
/// Called from an assembly exception stub with interrupts disabled on a
/// single-core system.  The frame allocator singleton is safe to access
/// under these conditions.
#[unsafe(no_mangle)]
pub extern "C" fn cow_handle_page_fault(fault_addr: u32, error_code: u32) -> u32 {
    if error_code & PTE_PRESENT == 0 || error_code & PF_ERR_WRITE == 0 {
        return 0;
    }

    let cr3: u32;
    unsafe {
        core::arch::asm!("mov {:e}, cr3", out(reg) cr3, options(nostack, nomem));
    }
    let pd_base = (cr3 & PTE_ADDR_MASK) as *const u32;

    // Page directory lookup.
    let pd_idx = ((fault_addr >> 22) & 0x3FF) as usize;
    let pde = unsafe { pd_base.add(pd_idx).read_volatile() };
    if pde & PTE_PRESENT == 0 {
        return 0;
    }

    // Page table lookup.
    let pt_base = (pde & PTE_ADDR_MASK) as *const u32;
    let pt_idx = ((fault_addr >> 12) & 0x3FF) as usize;
    let pte_ptr = unsafe { pt_base.add(pt_idx) } as *mut u32;
    let pte = unsafe { pte_ptr.read_volatile() };

    if pte & (PTE_COW | PTE_RW) == 0 {
        return 0;
    }

    // Allocate a fresh frame from Nanvix's frame allocator.
    let new_frame: FrameAddress = match frame::alloc() {
        Ok(f) => f,
        Err(_) => {
            unsafe {
                core::arch::asm!(
                    "mov dx, 102",
                    "mov al, 43",
                    "out dx, al",
                    options(nomem, nostack),
                );
                loop {
                    core::arch::asm!("hlt", options(nomem, nostack));
                }
            }
        },
    };

    let new_frame_addr = new_frame.into_raw_value() as u32;
    let fault_page = fault_addr & PTE_ADDR_MASK;

    // Copy 4 KB from the faulting (read-only) page to the new frame.
    unsafe {
        core::ptr::copy_nonoverlapping(fault_page as *const u8, new_frame_addr as *mut u8, 4096);
    }

    // Build the new PTE: original flags with COW cleared and RW set,
    // pointing to the freshly allocated frame.
    let new_pte = (pte & 0x00000FFF & !PTE_COW) | PTE_RW | (new_frame_addr & PTE_ADDR_MASK);
    unsafe {
        pte_ptr.write_volatile(new_pte);
    }

    // Invalidate the stale TLB entry.
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) fault_page, options(nostack, preserves_flags));
    }

    1
}
