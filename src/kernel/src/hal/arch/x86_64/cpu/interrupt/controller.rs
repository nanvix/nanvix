// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86_64::cpu::interrupt::{
    map::InterruptMap,
    pic::{
        Pic,
        UninitPic,
    },
    InterruptNumber,
};
use ::sys::error::Error;

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

pub struct InterruptController {
    #[allow(dead_code)]
    intmap: InterruptMap,
    pic: Pic,
}

impl InterruptController {
    pub fn new(pic: UninitPic, intmap: InterruptMap) -> Result<Self, Error> {
        let mut pic = pic;
        let pic: Pic = pic.init()?;
        Ok(Self { intmap, pic })
    }

    pub fn ack(&mut self, intnum: InterruptNumber) -> Result<(), Error> {
        self.pic.ack(intnum as u32);
        Ok(())
    }

    pub fn unmask(&mut self, intnum: InterruptNumber) -> Result<(), Error> {
        self.pic.unmask(intnum as u16);
        Ok(())
    }

    pub fn set_handler(
        &mut self,
        intnum: InterruptNumber,
        handler: Option<InterruptHandler>,
    ) -> Result<(), Error> {
        let intnum: u8 = intnum as u8;
        unsafe { INTERRUPT_VECTOR[intnum as usize] = handler };
        Ok(())
    }

    pub fn get_handler(&self, intnum: InterruptNumber) -> Result<Option<InterruptHandler>, Error> {
        let intnum: u8 = intnum as u8;
        unsafe { Ok(INTERRUPT_VECTOR[intnum as usize]) }
    }
}
