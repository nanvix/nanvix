// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod apic_base;

//==================================================================================================
// Exports
//==================================================================================================

pub use apic_base::ApicBase;

//==================================================================================================
// rdmsr
//==================================================================================================

///
/// # Description
///
/// Reads a model specific register.
///
/// # Parameters
///
/// * `msr`: Model specific register to read.
///
/// # Return Values
///
/// Returns the contents of the model specific register `msr`.
///
fn rdmsr(msr: u32) -> u64 {
    let mut eax: u32;
    let mut edx: u32;

    unsafe {
        core::arch::asm!(
            "rdmsr",
            out("edx") edx,
            out("eax") eax,
            in("ecx") msr,
            options(nomem, preserves_flags, nostack)
        )
    };

    ((edx as u64) << u32::BITS) | eax as u64
}

//==================================================================================================
// wrmsr
//==================================================================================================

///
/// # Description
///
/// Writes to a model specific register.
///
/// # Parameters
///
/// * `msr`: Model specific register to write.
/// * `value`: Value to write.
///
fn wrmsr(msr: u32, value: u64) {
    let eax: u32 = (value & u32::MAX as u64) as u32;
    let edx: u32 = ((value >> u32::BITS) & u32::MAX as u64) as u32;

    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") eax,
            in("edx") edx,
            options(nomem, preserves_flags, nostack)
        )
    };
}
