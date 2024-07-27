// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Task state segment (TSS).
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

// `Tss` must be 104 bytes long. This must match the hardware specification.
static_assert_size!(Tss, 104);
