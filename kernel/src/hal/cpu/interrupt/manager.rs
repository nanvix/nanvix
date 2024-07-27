// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    hal::{
        arch,
        cpu::interrupt::InterruptController,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct InterruptManager {
    controller: InterruptController,
}

impl InterruptManager {
    pub fn new(controller: arch::InterruptController) -> Result<Self, Error> {
        let mut intman: InterruptManager = InterruptManager {
            controller: InterruptController::new(controller)?,
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
        trace!("register_handler(): intnum={:?}, handler={:?}", intnum, handler);

        // Check if another handler is already registered.
        if self.controller.get_handler(intnum)?.is_some() {
            let reason: &str = "interrupt handler already registered";
            error!(
                "register_handler(): intnum={:?}, handler={:?}, reason={:?}",
                intnum, handler, reason
            );
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

    ///
    /// # Description
    ///
    /// High-level interrupt dispatcher.
    ///
    /// # Parameters
    ///
    /// - `intnum`: Number of the interrupt.
    ///
    #[no_mangle]
    extern "C" fn do_interrupt(intnum: arch::InterruptNumber) {
        match InterruptController::try_get() {
            Ok(controller) => {
                if let Err(e) = controller.ack(intnum) {
                    error!("failed to acknowledge interrupt: {:?}", e);
                }

                match controller.get_handler(intnum) {
                    Ok(Some(handler)) => handler(intnum),
                    Ok(None) => error!("no handler for interrupt {:?}", intnum as u32),
                    Err(e) => error!("failed to get handler: {:?}", e),
                }
            },
            Err(e) => error!("failed to get pic: {:?}", e),
        }
    }
}
