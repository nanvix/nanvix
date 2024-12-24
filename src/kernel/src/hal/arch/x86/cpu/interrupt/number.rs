// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum InterruptNumber {
    Timer = 0,
    Keyboard = 1,
    Com2 = 3,
    Com1 = 4,
    Lpt2 = 5,
    Floppy = 6,
    Lpt1 = 7,
    Cmos = 8,
    Free1 = 9,
    Free2 = 10,
    Free3 = 11,
    Mouse = 12,
    Coprocessor = 13,
    PrimaryAta = 14,
    SecondaryAta = 15,
}

impl InterruptNumber {
    pub const VALUES: [InterruptNumber; 15] = [
        InterruptNumber::Timer,
        InterruptNumber::Keyboard,
        InterruptNumber::Com2,
        InterruptNumber::Com1,
        InterruptNumber::Lpt2,
        InterruptNumber::Floppy,
        InterruptNumber::Lpt1,
        InterruptNumber::Cmos,
        InterruptNumber::Free1,
        InterruptNumber::Free2,
        InterruptNumber::Free3,
        InterruptNumber::Mouse,
        InterruptNumber::Coprocessor,
        InterruptNumber::PrimaryAta,
        InterruptNumber::SecondaryAta,
    ];
}
