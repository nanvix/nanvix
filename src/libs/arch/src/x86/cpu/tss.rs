// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[cfg(target_arch = "x86")]
/// Task state segment (TSS) for 32-bit x86.
#[repr(C, packed)]
pub struct Tss {
    pub link: u32,   // Previous TSS in the list.
    pub esp0: u32,   // Ring 0 stack pointer.
    pub ss0: u32,    // Ring 0 stack segment.
    pub esp1: u32,   // Ring 1 stack pointer.
    pub ss1: u32,    // Ring 1 stack segment.
    pub esp2: u32,   // Ring 2 stack pointer.
    pub ss2: u32,    // Ring 2 stack segment.
    pub cr3: u32,    // cr3.
    pub eip: u32,    // eip.
    pub eflags: u32, // eflags.
    pub eax: u32,    // eax.
    pub ecx: u32,    // ecx.
    pub edx: u32,    // edx.
    pub ebx: u32,    // ebx.
    pub esp: u32,    // esp.
    pub ebp: u32,    // ebp.
    pub esi: u32,    // esi.
    pub edi: u32,    // edi.
    pub es: u32,     // es.
    pub cs: u32,     // cs.
    pub ss: u32,     // ss.
    pub ds: u32,     // ds.
    pub fs: u32,     // fs.
    pub gs: u32,     // gs.
    pub ldtr: u32,   // LDT selector.
    pub iomap: u32,  // IO map.
}

#[cfg(target_arch = "x86")]
// `Tss` must be 104 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Tss, 104);

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
// `Tss` must be 104 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Tss, 104);
