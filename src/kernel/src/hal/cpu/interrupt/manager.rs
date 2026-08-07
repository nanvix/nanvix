// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    arch::{
        self,
        InterruptHandler,
    },
    cpu::interrupt::InterruptController,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

#[cfg(feature = "smp")]
pub use ::sys::mm::VirtualAddress;

//==================================================================================================
// Structures
//==================================================================================================

pub struct InterruptManager {
    controller: InterruptController,
}

impl InterruptManager {
    pub fn new(controller: arch::InterruptController) -> Result<Self, Error> {
        let mut intman: InterruptManager = InterruptManager {
            // TODO: add notes about unsafe.
            controller: unsafe { InterruptController::init(controller)? },
        };

        intman.init()?;

        Ok(intman)
    }

    ///
    /// # Description
    ///
    /// Registers an interrupt handler.
    ///
    /// # Parameters
    ///
    /// - `intnum`: Interrupt number.
    /// - `handler`: Interrupt handler.
    ///
    /// # Returns
    ///
    /// Upon success, the interrupt handler is registered. Upon failure, an error code is returned.
    ///
    pub fn register_handler(
        &mut self,
        intnum: arch::InterruptNumber,
        handler: arch::InterruptHandler,
    ) -> Result<(), Error> {
        trace!("intnum={:?}, handler={:?}", intnum, handler);

        // Check if another handler is already registered.
        if self.controller.get_handler(intnum)?.is_some() {
            let reason: &str = "interrupt handler already registered";
            error!("intnum={:?}, handler={:?}, reason={:?}", intnum, handler, reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        self.controller.set_handler(intnum, Some(handler))
    }

    pub fn unmask(&mut self, intnum: arch::InterruptNumber) -> Result<(), Error> {
        self.controller.unmask(intnum)
    }

    fn init(&mut self) -> Result<(), Error> {
        trace!("initializing interrupt manager");
        for intnum in arch::InterruptNumber::VALUES {
            trace!("registering default handler for interrupt {:?}", intnum);
            self.controller.set_handler(intnum, None)?;
        }
        Ok(())
    }

    #[cfg(feature = "smp")]
    pub fn start_core(
        &mut self,
        coreid: u8,
        entry: VirtualAddress,
        kstack: *const u8,
    ) -> Result<(), Error> {
        self.controller.start_core(coreid, entry, kstack)
    }

    ///
    /// # Description
    ///
    /// High-level interrupt dispatcher.
    ///
    /// # Parameters
    ///
    /// - `intnum`: Number of the interrupt.
    ///
    #[unsafe(no_mangle)]
    pub(crate) extern "C" fn do_interrupt(intnum: arch::InterruptNumber) {
        let handler: InterruptHandler = match InterruptController::try_get() {
            Ok(controller) => {
                if let Err(e) = controller.ack(intnum) {
                    {
                        error!("failed to acknowledge interrupt: {:?}", e);
                        return;
                    }
                }

                match controller.get_handler(intnum) {
                    Ok(Some(handler)) => handler,
                    Ok(None) => {
                        error!("no handler for interrupt {:?}", intnum as u32);
                        return;
                    },
                    Err(e) => {
                        error!("failed to get handler: {:?}", e);
                        return;
                    },
                }
            },
            Err(e) => {
                error!("failed to get pic: {:?}", e);
                return;
            },
        };

        // SAFETY: The interrupt handler is called without holding any resources.
        unsafe {
            handler(intnum);
        }
    }
}
