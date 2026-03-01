// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "acpi")]
pub mod acpi;
#[cfg(feature = "cpuid")]
pub mod cpuid;
pub mod cr0;
pub mod cr4;
pub mod eflags;
pub mod excp;
pub mod idt;
pub mod idtr;
#[cfg(feature = "ioapic")]
pub mod ioapic;
#[cfg(feature = "madt")]
pub mod madt;
#[cfg(feature = "msr")]
pub mod msr;
pub mod mxcrs;
#[cfg(feature = "pic")]
pub mod pic;
#[cfg(feature = "pit")]
pub mod pit;
pub mod ring;
pub mod tss;
#[cfg(feature = "xapic")]
pub mod xapic;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Issues the `pause` instruction.
///
pub fn pause() {
    unsafe {
        core::arch::asm!("pause");
    }
}

///
/// # Description
///
/// Issues the `hlt` instruction.
///
/// # Safety
///
/// This function is unsafe because `hlt` is a privileged instruction.
///
pub unsafe fn halt() {
    core::arch::asm!("hlt");
}

///
/// # Description
///
/// Issues the `cli` instruction.
///
/// # Safety
///
/// This function is unsafe because `cli` is a privileged instruction.
///
pub unsafe fn cli() {
    core::arch::asm!("cli");
}

///
/// # Description
///
/// Issues the `sti` instruction.
///
/// # Safety
///
/// This function is unsafe because `sti` is a privileged instruction.
///
pub unsafe fn sti() {
    core::arch::asm!("sti");
}

///
/// # Description
///
/// Issues the `rdtsc` instruction.
///
/// # Returns
///
/// The value of the `rdtsc` instruction.
///
/// # Note
///
/// An `lfence` is issued before `rdtsc` to serialize the instruction stream,
/// preventing speculative execution from reordering the TSC read relative to
/// preceding memory accesses.
///
pub fn rdtsc() -> u64 {
    let mut low: u32;
    let mut high: u32;

    unsafe {
        core::arch::asm!(
            "lfence",
            "rdtsc",
            out("edx") high,
            out("eax") low,
            options(nostack, nomem, preserves_flags)
        );
    }

    ((high as u64) << 32) | (low as u64)
}
