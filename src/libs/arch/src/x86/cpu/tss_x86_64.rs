// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Task state segment (TSS) for 64-bit x86_64.
#[repr(C, packed)]
pub struct Tss {
    pub reserved0: u32,  // Reserved.
    pub rsp0_low: u32,   // Ring 0 stack pointer (low 32 bits).
    pub rsp0_high: u32,  // Ring 0 stack pointer (high 32 bits).
    pub rsp1_low: u32,   // Ring 1 stack pointer (low 32 bits).
    pub rsp1_high: u32,  // Ring 1 stack pointer (high 32 bits).
    pub rsp2_low: u32,   // Ring 2 stack pointer (low 32 bits).
    pub rsp2_high: u32,  // Ring 2 stack pointer (high 32 bits).
    pub reserved1: u32,  // Reserved.
    pub reserved2: u32,  // Reserved.
    pub ist1_low: u32,   // IST1 (low 32 bits).
    pub ist1_high: u32,  // IST1 (high 32 bits).
    pub ist2_low: u32,   // IST2 (low 32 bits).
    pub ist2_high: u32,  // IST2 (high 32 bits).
    pub ist3_low: u32,   // IST3 (low 32 bits).
    pub ist3_high: u32,  // IST3 (high 32 bits).
    pub ist4_low: u32,   // IST4 (low 32 bits).
    pub ist4_high: u32,  // IST4 (high 32 bits).
    pub ist5_low: u32,   // IST5 (low 32 bits).
    pub ist5_high: u32,  // IST5 (high 32 bits).
    pub ist6_low: u32,   // IST6 (low 32 bits).
    pub ist6_high: u32,  // IST6 (high 32 bits).
    pub ist7_low: u32,   // IST7 (low 32 bits).
    pub ist7_high: u32,  // IST7 (high 32 bits).
    pub reserved3: u32,  // Reserved.
    pub reserved4: u32,  // Reserved.
    pub reserved5: u16,  // Reserved.
    pub iomap_base: u16, // I/O map base address.
}

// `Tss` must be 104 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Tss, 104);
