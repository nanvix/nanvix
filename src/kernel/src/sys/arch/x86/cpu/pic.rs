// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Constants
//==================================================================================================

pub const PIC_NUM_IRQS: u8 = 8; // Number of IRQs

/// Master PIC Registers
pub const PIC_CTRL_MASTER: u8 = 0x20; // Control
pub const PIC_DATA_MASTER: u8 = 0x21; // Data

/// Slave PIC Registers
pub const PIC_CTRL_SLAVE: u8 = 0xa0; // Control register
pub const PIC_DATA_SLAVE: u8 = 0xa1; // Data register

/// Initialization Command Word 1
pub mod icw1 {
    /// ICW4 (not) needed
    pub enum Ic4 {
        /// ICW4 (not) needed
        NotNeeded = 0x10,
        /// ICW4 needed
        Needed = (1 << 0) | 0x10,
    }

    /// Single or Cascade mode
    pub enum Single {
        /// Single (Cascade) mode
        Cascade = 0x10,
        /// Single mode
        Single = (1 << 1) | 0x10,
    }

    /// Call Address Interval
    pub enum Interval {
        /// Call Address Interval 4 (8)
        Interval4 = 0x10,
        /// Call Address Interval 8 (16)
        Interval8 = (1 << 2) | 0x10,
    }

    /// Level triggered (edge) mode
    pub enum Level {
        /// Level triggered (edge) mode
        Edge = 0x10,
        /// Level triggered mode
        Level = (1 << 3) | 0x10,
    }
}

/// Initialization Command Word 4
pub mod icw4 {
    /// Microprocessor Mode
    pub enum MicroprocessorMode {
        /// MCS-80/85 Mode
        Mode8085 = 0,
        /// 8086/8088 Mode
        Mode8086 = (1 << 0),
    }

    /// Auto End of Interrupt
    pub enum AutoEoi {
        /// Normal EOI
        Normal = 0,
        /// Auto EOI
        Auto = (1 << 1),
    }

    /// Buffer Mode
    pub enum BufMode {
        /// Non-Buffered Mode
        NonBuffered = 0,
        /// Buffered Mode (Slave)
        BufferedSlave = (0b10 << 2),
        /// Buffered Mode (Master)
        BufferedMaster = (0b11 << 2),
    }

    /// Special Fully Nested Mode
    pub enum SpecialFullyNested {
        /// Not Special Fully Nested Mode
        NotSpecialFullyNested = 0,
        /// Special Fully Nested Mode
        SpecialFullyNested = (1 << 4),
    }
}

/// Operation Command Word 2
pub mod ocw2 {
    pub enum Eoi {
        /// Non-specific EOI
        NonSpecific = (0b001 << 5),
    }
}
