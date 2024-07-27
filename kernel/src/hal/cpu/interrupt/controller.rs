// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::alloc::rc::Rc;
use ::core::cell::RefCell;

//==================================================================================================
// Structures
//==================================================================================================

static mut INTERRUPT_CONTROLLER: Option<InterruptController> = None;

#[derive(Clone)]
pub struct InterruptController(Rc<RefCell<arch::InterruptController>>);

impl InterruptController {
    pub fn new(controller: arch::InterruptController) -> Result<Self, Error> {
        let controller: InterruptController =
            InterruptController(Rc::new(RefCell::new(controller)));

        unsafe {
            INTERRUPT_CONTROLLER = Some(controller.clone());
        }

        Ok(controller)
    }

    pub fn ack(&self, intnum: arch::InterruptNumber) -> Result<(), Error> {
        self.0.borrow_mut().ack(intnum)
    }

    pub fn unmask(&self, intnum: arch::InterruptNumber) -> Result<(), Error> {
        self.0.borrow_mut().unmask(intnum)
    }

    pub fn get_handler(
        &self,
        intnum: arch::InterruptNumber,
    ) -> Result<Option<arch::InterruptHandler>, Error> {
        self.0.borrow().get_handler(intnum)
    }

    pub fn set_handler(
        &self,
        intnum: arch::InterruptNumber,
        handler: Option<arch::InterruptHandler>,
    ) -> Result<(), Error> {
        self.0.borrow_mut().set_handler(intnum, handler)
    }
}

impl InterruptController {
    pub fn try_get() -> Result<InterruptController, Error> {
        unsafe {
            match &INTERRUPT_CONTROLLER {
                Some(controller) => Ok(controller.clone()),
                None => {
                    Err(Error::new(ErrorCode::NoSuchDevice, "interrupt controller not initialized"))
                },
            }
        }
    }
}
