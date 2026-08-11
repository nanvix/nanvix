// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Equivalent to the replaced instruction because it returns the same 32-bit CR3 value.
#[cfg(target_arch = "x86")]
unsafe fn env_interaction_read_cr3() -> u32 {
    let value: u32;
    unsafe {
        asm!("mov {0:e}, cr3", out(reg) value);
    }
    value
}

// Equivalent to the replaced instruction because it returns the same 64-bit CR3 value.
#[cfg(target_arch = "x86_64")]
unsafe fn env_interaction_read_cr3() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {0:r}, cr3", out(reg) value);
    }
    value
}

// Equivalent to the replaced instruction because it writes the same 32-bit CR3 value.
#[cfg(target_arch = "x86")]
unsafe fn env_interaction_write_cr3(value: u32) {
    unsafe {
        asm!("mov cr3, {0:e}", in(reg) value);
    }
}

// Equivalent to the replaced instruction because it writes the same 64-bit CR3 value.
#[cfg(target_arch = "x86_64")]
unsafe fn env_interaction_write_cr3(value: u64) {
    unsafe {
        asm!("mov cr3, {0:r}", in(reg) value);
    }
}
