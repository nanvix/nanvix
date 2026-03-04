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
    #[cfg(feature = "microvm")]
    Ikc = 9,
    #[cfg(not(feature = "microvm"))]
    Free1 = 9,
    Free2 = 10,
    Free3 = 11,
    Mouse = 12,
    Coprocessor = 13,
    PrimaryAta = 14,
    SecondaryAta = 15,
}

impl InterruptNumber {
    /// IRQ 9 variant, which is `Ikc` on microvm and `Free1` otherwise.
    #[cfg(feature = "microvm")]
    const IRQ9: InterruptNumber = InterruptNumber::Ikc;
    #[cfg(not(feature = "microvm"))]
    const IRQ9: InterruptNumber = InterruptNumber::Free1;

    pub const VALUES: [InterruptNumber; 15] = [
        InterruptNumber::Timer,
        InterruptNumber::Keyboard,
        InterruptNumber::Com2,
        InterruptNumber::Com1,
        InterruptNumber::Lpt2,
        InterruptNumber::Floppy,
        InterruptNumber::Lpt1,
        InterruptNumber::Cmos,
        InterruptNumber::IRQ9,
        InterruptNumber::Free2,
        InterruptNumber::Free3,
        InterruptNumber::Mouse,
        InterruptNumber::Coprocessor,
        InterruptNumber::PrimaryAta,
        InterruptNumber::SecondaryAta,
    ];
}
