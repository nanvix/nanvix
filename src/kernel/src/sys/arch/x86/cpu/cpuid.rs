// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # x86 CPUID
//!
//! This module provides helper structures and functions for the CPUID instruction.
//!

//==================================================================================================
// Constants
//==================================================================================================

pub const CPUID_FEATURES: u32 = 1;

//==================================================================================================
//  Structures
//==================================================================================================

struct EbxFeature;

impl EbxFeature {
    /// CPUID.01H:EBX[23:16] Brand Index
    pub const BRAND_INDEX_SHIFT: u32 = 0;
    /// CPUID.01H:EBX[23:16] Brand Index
    pub const BRAND_INDEX_MASK: u32 = 0xFF << Self::BRAND_INDEX_SHIFT;

    /// CPUID.01H:EBX[47:40] CLFLUSH line size
    pub const CLFLUSH_LINE_SIZE_SHIFT: u32 = 8;
    /// CPUID.01H:EBX[47:40] CLFLUSH line size
    pub const CLFLUSH_LINE_SIZE_MASK: u32 = 0xFF << Self::CLFLUSH_LINE_SIZE_SHIFT;

    /// CPUID.01H:EBX[55:48] Maximum number of addressable IDs for logical processors in this package
    pub const MAX_ADDRESSABLE_IDS_SHIFT: u32 = 16;
    /// CPUID.01H:EBX[55:48] Maximum number of addressable IDs for logical processors in this package
    pub const MAX_ADDRESSABLE_IDS_MASK: u32 = 0xFF << Self::MAX_ADDRESSABLE_IDS_SHIFT;

    /// CPUID.01H:EBX[63:56] Initial APIC ID
    pub const INITIAL_APIC_ID_SHIFT: u32 = 24;
    /// CPUID.01H:EBX[63:56] Initial APIC ID
    pub const INITIAL_APIC_ID_MASK: u32 = 0xFF << Self::INITIAL_APIC_ID_SHIFT;
}

pub enum EdxFeature {
    Fpu = 1 << 0,
    Vme = 1 << 1,
    De = 1 << 2,
    Pse = 1 << 3,
    Tsc = 1 << 4,
    Msr = 1 << 5,
    Pae = 1 << 6,
    Mce = 1 << 7,
    Cx8 = 1 << 8,
    Apic = 1 << 9,
    Sep = 1 << 11,
    Mtrr = 1 << 12,
    Pge = 1 << 13,
    Mca = 1 << 14,
    Cmov = 1 << 15,
    Pat = 1 << 16,
    Pse36 = 1 << 17,
    Psn = 1 << 18,
    Clflush = 1 << 19,
    Ds = 1 << 21,
    Acpi = 1 << 22,
    Mmx = 1 << 23,
    Fxsr = 1 << 24,
    Sse = 1 << 25,
    Sse2 = 1 << 26,
    Ss = 1 << 27,
    Htt = 1 << 28,
    Tm = 1 << 29,
    Ia64 = 1 << 30,
    Pbe = 1 << 31,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Executes the CPUID instruction.
///
/// # Parameters
///
/// - `eax`: The value to set in the EAX register.
///
/// # Return Values
///
/// A tuple containing the values of the EAX, EBX, ECX, and EDX registers is returned.
///
fn cpuid(eax: u32) -> (u32, u32, u32, u32) {
    let mut eax: u32 = eax;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;

    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") eax => eax,
            out("ebx") ebx,
            out("ecx") ecx,
            out("edx") edx,
            options(nomem, preserves_flags, nostack)
        );
    }

    (eax, ebx, ecx, edx)
}

///
/// # Description
///
/// Gets the brand index of the processor.
///
/// # Return Values
///
/// The brand index of the processor.
///
pub fn get_brand_index() -> u8 {
    let (_, ebx, _, _) = cpuid(CPUID_FEATURES);

    ((ebx & EbxFeature::BRAND_INDEX_MASK) >> EbxFeature::BRAND_INDEX_SHIFT) as u8
}

///
/// # Description
///
/// Gets the CLFLUSH line size of the processor.
///
/// # Return Values
///
/// The CLFLUSH line size of the processor.
///
pub fn get_clflush_line_size() -> u8 {
    let (_, ebx, _, _) = cpuid(CPUID_FEATURES);

    ((ebx & EbxFeature::CLFLUSH_LINE_SIZE_MASK) >> EbxFeature::CLFLUSH_LINE_SIZE_SHIFT) as u8
}

///
/// # Description
///
/// Gets the maximum number of addressable IDs for logical processors in this package.
///
/// # Return Values
///
/// The maximum number of addressable IDs for logical processors in this package.
///
pub fn get_max_addressable_ids() -> u8 {
    let (_, ebx, _, _) = cpuid(CPUID_FEATURES);

    ((ebx & EbxFeature::MAX_ADDRESSABLE_IDS_MASK) >> EbxFeature::MAX_ADDRESSABLE_IDS_SHIFT) as u8
}

///
/// # Description
///
/// Returns the default APIC ID of the processor.
///
/// # Return Values
///
/// The default APIC ID of the processor.
///
pub fn get_apic_id() -> u8 {
    let (_, ebx, _, _) = cpuid(CPUID_FEATURES);

    ((ebx & EbxFeature::INITIAL_APIC_ID_MASK) >> EbxFeature::INITIAL_APIC_ID_SHIFT) as u8
}

///
/// # Description
///
/// Checks if the CPU supports the CPUID instruction.
///
/// # Return Values
///
/// If the CPU supports the CPUID instruction, `true` is returned. Otherwise, `false` is returned.
///
#[inline(never)]
pub fn has_cpuid() -> bool {
    let result: u32;
    unsafe {
        ::core::arch::asm!(
            "pushfl",                   // Save EFLAGS
            "pushfl",                   // Store EFLAGS
            "xorl $0x200000, (%esp)",   // Invert the ID bit in stored EFLAGS
            "popfl",                    // Load stored EFLAGS (with ID bit inverted)
            "pushfl",                   // Store EFLAGS again (ID bit may or may not be inverted)
            "popl %eax",                // eax = modified EFLAGS (ID bit may or may not be inverted)
            "xorl (%esp), %eax",        // eax = whichever bits were changed
            "popfl",                    // Restore original EFLAGS
            "andl $0x200000, %eax",     // eax = zero if ID bit can't be changed, else non-zero
            "movl $0, %eax",
            "jz 1f",
            "movl $1, %eax",
            "1:",
            out("eax") result,
            options(preserves_flags, att_syntax)
        );
    }
    result != 0
}

///
/// # Description
///
/// Checks if the CPU has an FPU.
///
/// # Return Values
///
/// If the CPU has an FPU, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_fpu() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Fpu as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has VME.
///
/// # Return Values
///
/// If the CPU has VME, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_vme() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Vme as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has DE.
///
/// # Return Values
///
/// If the CPU has DE, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_de() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::De as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has PSE.
///
/// # Return Values
///
/// If the CPU has PSE, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_pse() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Pse as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has TSC.
///
/// # Return Values
///
/// If the CPU has TSC, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_tsc() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Tsc as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has MSR.
///
/// # Return Values
///
/// If the CPU has MSR, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_msr() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Msr as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has PAE.
///
/// # Return Values
///
/// If the CPU has PAE, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_pae() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Pae as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has MCE.
///
/// # Return Values
///
/// If the CPU has MCE, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_mce() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Mce as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has CX8.
///
/// # Return Values
///
/// If the CPU has CX8, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_cx8() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Cx8 as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has an APIC.
///
/// # Return Values
///
/// If the CPU has an APIC, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_apic() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Apic as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has SEP.
///
/// # Return Values
///
/// If the CPU has SEP, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_sep() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Sep as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has MTRR.
///
/// # Return Values
///
/// If the CPU has MTRR, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_mtrr() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Mtrr as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has PGE.
///
/// # Return Values
///
/// If the CPU has PGE, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_pge() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Pge as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has MCA.
///
/// # Return Values
///
/// If the CPU has MCA, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_mca() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Mca as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has CMOV.
///
/// # Return Values
///
/// If the CPU has CMOV, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_cmov() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Cmov as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has PAT.
///
/// # Return Values
///
/// If the CPU has PAT, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_pat() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Pat as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has PSE-36.
///
/// # Return Values
///
/// If the CPU has PSE-36, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_pse36() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Pse36 as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has PSN.
///
/// # Return Values
///
/// If the CPU has PSN, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_psn() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Psn as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has CLFLUSH.
///
/// # Return Values
///
/// If the CPU has CLFLUSH, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_clflush() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Clflush as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has DS.
///
/// # Return Values
///
/// If the CPU has DS, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_ds() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Ds as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has ACPI.
///
/// # Return Values
///
/// If the CPU has ACPI, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_acpi() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Acpi as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has MMX.
///
/// # Return Values
///
/// If the CPU has MMX, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_mmx() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Mmx as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has FXSR.
///
/// # Return Values
///
/// If the CPU has FXSR, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_fxsr() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Fxsr as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has SSE.
///
/// # Return Values
///
/// If the CPU has SSE, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_sse() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Sse as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has SSE2.
///
/// # Return Values
///
/// If the CPU has SSE2, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_sse2() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Sse2 as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has SS.
///
/// # Return Values
///
/// If the CPU has SS, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_ss() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Ss as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has HTT.
///
/// # Return Values
///
/// If the CPU has HTT, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_htt() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Htt as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has TM.
///
/// # Return Values
///
/// If the CPU has TM, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_tm() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Tm as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has IA64.
///
/// # Return Values
///
/// If the CPU has IA64, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_ia64() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Ia64 as u32) != 0
}

///
/// # Description
///
/// Checks if the CPU has PBE.
///
/// # Return Values
///
/// If the CPU has PBE, `true` is returned. Otherwise, `false` is returned.
///
pub fn has_pbe() -> bool {
    let (_, _, _, edx): (u32, u32, u32, u32) = cpuid(CPUID_FEATURES);

    (edx & EdxFeature::Pbe as u32) != 0
}
