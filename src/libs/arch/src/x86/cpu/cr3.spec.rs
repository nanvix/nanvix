// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::vstd::prelude::*;

verus! {

/// Mask for the physical paging-root address in CR3.
pub const CR3_ROOT_MASK: u64 = 0x000F_FFFF_FFFF_F000;

/// Mask for the page-level cache controls represented by [`Cr3Register`].
pub const CR3_CACHE_CONTROL_MASK: u64 = (1 << 3) | (1 << 4);

/// Returns whether `value` is a valid CR3 value under Nanvix's current no-PCID configuration.
pub open spec fn valid_cr3_environment_value(value: u64) -> bool {
    &&& value & !(CR3_ROOT_MASK | CR3_CACHE_CONTROL_MASK) == 0
    &&& value & CR3_ROOT_MASK != 0
}

} // verus!

// Equivalent to the replaced instruction because it returns the same 32-bit CR3 value.
#[cfg(target_arch = "x86")]
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        valid_cr3_environment_value(result as u64),
)]
unsafe fn env_interaction_read_cr3() -> u32 {
    let value: u32;
    unsafe {
        asm!("mov {0:e}, cr3", out(reg) value);
    }
    value
}

// Equivalent to the replaced instruction because it returns the same 64-bit CR3 value.
#[cfg(target_arch = "x86_64")]
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        valid_cr3_environment_value(result),
)]
unsafe fn env_interaction_read_cr3() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {0:r}, cr3", out(reg) value);
    }
    value
}

// Equivalent to the replaced instruction because it writes the same 32-bit CR3 value.
#[cfg(target_arch = "x86")]
#[verus_verify(external_body)]
#[verus_spec(
    requires
        valid_cr3_environment_value(value as u64),
)]
unsafe fn env_interaction_write_cr3(value: u32) {
    unsafe {
        asm!("mov cr3, {0:e}", in(reg) value);
    }
}

// Equivalent to the replaced instruction because it writes the same 64-bit CR3 value.
#[cfg(target_arch = "x86_64")]
#[verus_verify(external_body)]
#[verus_spec(
    requires
        valid_cr3_environment_value(value),
)]
unsafe fn env_interaction_write_cr3(value: u64) {
    unsafe {
        asm!("mov cr3, {0:r}", in(reg) value);
    }
}
