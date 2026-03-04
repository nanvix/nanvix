// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Task State Segment (TSS) for x86_64 long mode.
///
/// In long mode, the TSS does not store general-purpose registers
/// and is not used for hardware task switching. It holds:
/// - RSP values for privilege level transitions (RSP0, RSP1, RSP2)
/// - Interrupt Stack Table (IST) pointers (IST1-IST7)
/// - I/O permission bitmap offset
#[repr(C, packed)]
pub struct Tss {
    /// Reserved.
    pub reserved0: u32,
    /// Stack pointer for ring 0.
    pub rsp0: u64,
    /// Stack pointer for ring 1.
    pub rsp1: u64,
    /// Stack pointer for ring 2.
    pub rsp2: u64,
    /// Reserved.
    pub reserved1: u64,
    /// Interrupt Stack Table pointer 1.
    pub ist1: u64,
    /// Interrupt Stack Table pointer 2.
    pub ist2: u64,
    /// Interrupt Stack Table pointer 3.
    pub ist3: u64,
    /// Interrupt Stack Table pointer 4.
    pub ist4: u64,
    /// Interrupt Stack Table pointer 5.
    pub ist5: u64,
    /// Interrupt Stack Table pointer 6.
    pub ist6: u64,
    /// Interrupt Stack Table pointer 7.
    pub ist7: u64,
    /// Reserved.
    pub reserved2: u64,
    /// Reserved.
    pub reserved3: u16,
    /// I/O map base address (offset from TSS base).
    pub iomap_base: u16,
}

// `Tss` must be 104 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Tss, 104);
