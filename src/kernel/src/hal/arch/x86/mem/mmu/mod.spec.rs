// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Paging-enable bit in CR0.
const ENV_INTERACTION_CR0_PG: usize = 1 << 31;

// Equivalent to the replaced instruction because it installs the same value in CR3.
unsafe fn env_interaction_write_cr3(cr3: usize) {
    unsafe {
        arch::asm!(
            "mov {0}, %eax",
            "mov %eax, %cr3",
            in(reg) cr3,
            options(nostack, att_syntax)
        );
    }
}

// Equivalent to the replaced instruction because it returns the current CR0 value.
unsafe fn env_interaction_read_cr0() -> usize {
    let cr0: usize;
    unsafe {
        arch::asm!("mov %cr0, {0}", out(reg) cr0, options(nostack, att_syntax));
    }
    cr0
}

// Equivalent to the replaced instruction because it installs the same value in CR0.
unsafe fn env_interaction_write_cr0(cr0: usize) {
    unsafe {
        arch::asm!("mov {0}, %cr0", in(reg) cr0, options(nostack, att_syntax));
    }
}
