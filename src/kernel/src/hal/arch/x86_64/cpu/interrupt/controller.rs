// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86_64::cpu::interrupt::{
    map::InterruptMap,
    InterruptNumber,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Interrupt Vector
//==================================================================================================

/// Number of entries in the interrupt vector.
pub const INTERRUPT_VECTOR_LENGTH: usize = 256;

///
/// # Description
///
/// A type that represents an interrupt handler.
///
pub type InterruptHandler = unsafe fn(InterruptNumber);

#[unsafe(no_mangle)]
static mut INTERRUPT_VECTOR: [Option<InterruptHandler>; INTERRUPT_VECTOR_LENGTH] =
    [None; INTERRUPT_VECTOR_LENGTH];

//==================================================================================================
// Interrupt Controller
//==================================================================================================

// TODO: Add PIC/xAPIC/IOAPIC support when the x86_64 arch library provides those modules.
pub struct InterruptController {
    intmap: InterruptMap,
}

impl InterruptController {
    pub fn new(intmap: InterruptMap) -> Result<Self, Error> {
        Ok(Self { intmap })
    }

    pub fn ack(&mut self, _intnum: InterruptNumber) -> Result<(), Error> {
        // TODO: Implement interrupt acknowledgement via xAPIC/PIC.
        Ok(())
    }

    pub fn unmask(&mut self, _intnum: InterruptNumber) -> Result<(), Error> {
        // TODO: Implement interrupt unmasking via IOAPIC/PIC.
        Ok(())
    }

    pub fn set_handler(
        &mut self,
        intnum: InterruptNumber,
        handler: Option<InterruptHandler>,
    ) -> Result<(), Error> {
        let intnum: u8 = self.intmap[intnum];
        unsafe { INTERRUPT_VECTOR[intnum as usize] = handler };
        Ok(())
    }

    pub fn get_handler(&self, intnum: InterruptNumber) -> Result<Option<InterruptHandler>, Error> {
        let intnum: u8 = self.intmap[intnum];
        unsafe { Ok(INTERRUPT_VECTOR[intnum as usize]) }
    }
}
