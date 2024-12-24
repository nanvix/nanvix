// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch;
use ::alloc::rc::Rc;
use ::core::cell::RefCell;
use ::sys::error::{
    Error,
    ErrorCode,
};

#[cfg(feature = "smp")]
pub use ::sys::mm::VirtualAddress;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone)]
pub struct InterruptController(Rc<RefCell<arch::InterruptController>>);

//==================================================================================================
// Global Variables
//==================================================================================================

static mut INTERRUPT_CONTROLLER: Option<InterruptController> = None;

//==================================================================================================
// Implementations
//==================================================================================================

impl InterruptController {
    ///
    /// # Description
    ///
    /// Initializes the interrupt controller.
    ///
    /// # Parameters
    ///
    /// - `controller`: The interrupt controller.
    ///
    /// # Returns
    ///
    /// Returns the interrupt controller.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it mutates global variables.
    ///
    pub unsafe fn init(controller: arch::InterruptController) -> Result<Self, Error> {
        // Check if the interrupt controller was already initialized.
        if INTERRUPT_CONTROLLER.is_some() {
            let reason: &str = "interrupt controller already initialized";
            error!("init(): reason={:?}", reason);
            return Err(Error::new(
                ErrorCode::EntryExists,
                "interrupt controller already initialized",
            ));
        }

        let controller: InterruptController =
            InterruptController(Rc::new(RefCell::new(controller)));

        INTERRUPT_CONTROLLER = Some(controller.clone());

        Ok(controller)
    }

    pub fn ack(&self, intnum: arch::InterruptNumber) -> Result<(), Error> {
        self.0.borrow_mut().ack(intnum)
    }

    #[cfg(feature = "smp")]
    pub fn start_core(
        &self,
        coreid: u8,
        entry: VirtualAddress,
        kstack: *const u8,
    ) -> Result<(), Error> {
        self.0.borrow_mut().start_core(coreid, entry, kstack)
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

    pub fn try_get() -> Result<InterruptController, Error> {
        unsafe {
            match INTERRUPT_CONTROLLER.clone() {
                Some(controller) => Ok(controller),
                None => {
                    Err(Error::new(ErrorCode::NoSuchDevice, "interrupt controller not initialized"))
                },
            }
        }
    }
}
