// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::arch::mem::PAGE_SIZE;
use ::core::arch::global_asm;

const CLEAR_BSS: u32 = if cfg!(feature = "microvm") { 0 } else { 1 };

global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,
    ".balign 16",
    ".global _do_start",
    "_do_start:",
    "    mov x19, x0",
    "    mov x20, x1",
    ".if {CLEAR_BSS}",
    "    adrp x2, __BSS_START",
    "    add  x2, x2, :lo12:__BSS_START",
    "    adrp x3, __BSS_END",
    "    add  x3, x3, :lo12:__BSS_END",
    "1:",
    "    cmp x2, x3",
    "    b.hs 2f",
    "    str xzr, [x2], #8",
    "    b 1b",
    "2:",
    ".endif",
    "    adrp x2, __aarch64_vector_table",
    "    add  x2, x2, :lo12:__aarch64_vector_table",
    "    msr vbar_el1, x2",
    "    isb",
    "    adrp x2, kstack",
    "    add  x2, x2, :lo12:kstack",
    "    mov sp, x2",
    "    sub sp, sp, #16",
    "    str x19, [sp]",
    "    str x20, [sp, #8]",
    "    mov x0, sp",
    "    bl kmain",
    "3:",
    "    wfi",
    "    b 3b",
    CLEAR_BSS = const CLEAR_BSS,
);

global_asm!(
    ".section .data",
    ".balign {PAGE_SIZE}",
    ".global kstack_guard",
    "kstack_guard:",
    ".fill ({PAGE_SIZE} / 4), 4, {KSTACK_GUARD_PATTERN}",
    ".fill ({KSTACK_SIZE} - {PAGE_SIZE}), 1, 0",
    ".global kstack",
    "kstack:",
    PAGE_SIZE = const PAGE_SIZE,
    KSTACK_SIZE = const ::config::kernel::KSTACK_SIZE,
    KSTACK_GUARD_PATTERN = const ::config::kernel::KSTACK_GUARD_PATTERN,
);

global_asm!(
    ".section .bss",
    ".balign {PAGE_SIZE}",
    ".global kredzone",
    "kredzone:",
    ".space {KREDZONE_SIZE}",
    PAGE_SIZE = const PAGE_SIZE,
    KREDZONE_SIZE = const ::config::kernel::KREDZONE_SIZE,
);
