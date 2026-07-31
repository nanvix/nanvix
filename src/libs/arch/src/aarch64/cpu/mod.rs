// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[path = "../../x86/cpu/excp.rs"]
pub mod excp;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Yields execution to another hardware thread.
///
#[inline]
pub fn pause() {
    unsafe {
        core::arch::asm!("yield", options(nomem, nostack, preserves_flags));
    }
}

///
/// # Description
///
/// Waits for an interrupt.
///
/// # Safety
///
/// This function may only be called from privileged code whose interrupt state is known.
///
#[inline]
pub unsafe fn halt() {
    core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
}

///
/// # Description
///
/// Masks IRQ exceptions.
///
/// # Safety
///
/// This function may only be called from EL1.
///
#[inline]
pub unsafe fn cli() {
    core::arch::asm!("msr daifset, #2", options(nomem, nostack, preserves_flags));
}

///
/// # Description
///
/// Unmasks IRQ exceptions.
///
/// # Safety
///
/// This function may only be called from EL1 after the interrupt controller is initialized.
///
#[inline]
pub unsafe fn sti() {
    core::arch::asm!("msr daifclr, #2", options(nomem, nostack, preserves_flags));
}

///
/// # Description
///
/// Reads the ARM generic timer virtual count register.
///
/// # Returns
///
/// The current virtual counter value.
///
#[inline]
pub fn rdtsc() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "isb",
            "mrs {value}, cntvct_el0",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

///
/// # Description
///
/// Cleans writes to a memory range to the point of unification.
///
/// AArch64 does not require the data and instruction caches to be coherent for self-modifying code.
/// This routine cleans each affected data-cache line and issues the required completion barrier.
///
/// # Parameters
///
/// - `start`: Start of the modified memory range.
/// - `len`: Length of the modified memory range in bytes.
///
/// # Safety
///
/// The caller must execute at EL1 and ensure that `[start, start + len)` is a valid mapped range
/// that does not wrap the address space.
///
#[inline]
pub unsafe fn clean_data_cache_to_pou(start: *const u8, len: usize) {
    if len == 0 {
        return;
    }

    let ctr_el0: u64;
    unsafe {
        core::arch::asm!(
            "mrs {ctr_el0}, ctr_el0",
            ctr_el0 = out(reg) ctr_el0,
            options(nomem, nostack, preserves_flags)
        );
    }

    let data_line_size: usize = 4usize << ((ctr_el0 >> 16) & 0xf);
    let start: usize = start as usize;
    let end: usize = start + len;

    let mut address: usize = start & !(data_line_size - 1);
    while address < end {
        unsafe {
            core::arch::asm!(
                "dc cvau, {address}",
                address = in(reg) address,
                options(nostack, preserves_flags)
            );
        }
        address += data_line_size;
    }

    unsafe {
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
    }
}

///
/// # Description
///
/// Invalidates instruction-cache lines for a virtual-address range.
///
/// # Safety
///
/// The caller must execute at EL1, clean modified data to the point of unification first, and
/// ensure that `[start, start + len)` is a valid mapped range that does not wrap the address space.
///
#[inline]
unsafe fn invalidate_instruction_cache_range(start: *const u8, len: usize) {
    if len == 0 {
        return;
    }

    let ctr_el0: u64;
    unsafe {
        core::arch::asm!(
            "mrs {ctr_el0}, ctr_el0",
            ctr_el0 = out(reg) ctr_el0,
            options(nomem, nostack, preserves_flags)
        );
    }

    let instruction_line_size: usize = 4usize << (ctr_el0 & 0xf);
    let start: usize = start as usize;
    let end: usize = start + len;
    let mut address: usize = start & !(instruction_line_size - 1);
    while address < end {
        unsafe {
            core::arch::asm!(
                "ic ivau, {address}",
                address = in(reg) address,
                options(nostack, preserves_flags)
            );
        }
        address += instruction_line_size;
    }

    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

///
/// # Description
///
/// Invalidates all instruction caches in the inner-shareable domain.
///
/// This is required when data was modified through an alias other than the virtual address from
/// which the instructions will execute.
///
/// # Safety
///
/// The caller must execute at EL1 and clean all modified data to the point of unification first.
///
#[inline]
pub unsafe fn invalidate_instruction_cache_all() {
    unsafe {
        core::arch::asm!(
            "dsb ish",
            "ic ialluis",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

///
/// # Description
///
/// Makes writes to a memory range visible to subsequent instruction fetches through the same
/// virtual addresses.
///
/// # Safety
///
/// The caller must execute at EL1 and ensure that `[start, start + len)` is a valid mapped range
/// that does not wrap the address space.
///
#[inline]
pub unsafe fn synchronize_instruction_cache(start: *const u8, len: usize) {
    unsafe {
        clean_data_cache_to_pou(start, len);
        invalidate_instruction_cache_range(start, len);
    }
}
