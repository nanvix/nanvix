// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "acpi")]
pub mod acpi;
#[cfg(feature = "cpuid")]
pub mod cpuid;
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
pub fn rdtsc() -> u64 {
    let mut low: u32;
    let mut high: u32;

    unsafe {
        core::arch::asm!("rdtsc", out("edx") high, out("eax") low);
    }

    ((high as u64) << 32) | (low as u64)
}
