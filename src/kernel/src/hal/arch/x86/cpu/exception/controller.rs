// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::{
    ContextInformation,
    ExceptionInformation,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Types
//==================================================================================================

///
/// # Description
///
/// A type that represents an exception handler.
///
pub type ExceptionHandler = fn(&ExceptionInformation, &ContextInformation);

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents an exception controller.
///
pub struct ExceptionController;

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Exception handler.
///
static mut HANDLER: Option<ExceptionHandler> = None;

///
/// # Description
///
/// Exception controller.
///
static mut SINGLETON_CONTROLLER: Option<ExceptionController> = None;

//==================================================================================================
// Implementations
//==================================================================================================

impl ExceptionController {
    ///
    /// # Description
    ///
    /// Initializes the exception controller.
    ///
    /// # Returns
    ///
    /// Upon successful completion the exception controller is returned. Upon failure, an error is
    /// returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it mutates global variables.
    ///
    pub unsafe fn init() -> Result<Self, Error> {
        if SINGLETON_CONTROLLER.is_some() {
            let reason: &str = "exception controller already initialized";
            error!("init(): {}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        SINGLETON_CONTROLLER = Some(Self);

        Ok(Self)
    }

    ///
    /// # Description
    ///
    /// Sets an exception handler.
    ///
    /// # Parameters
    ///
    /// - `handler`: Exception handler.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it mutates global variables.
    ///
    pub unsafe fn register_handler(&mut self, handler: ExceptionHandler) -> Result<(), Error> {
        trace!("register_handler(): handler={:?}", handler);

        // Check if the handler is already set.
        if HANDLER.is_some() {
            let reason: &str = "exception handler already set";
            error!("register_handler(): {}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        HANDLER = Some(handler);

        Ok(())
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl Drop for ExceptionController {
    fn drop(&mut self) {
        unsafe {
            SINGLETON_CONTROLLER = None;
            HANDLER = None;
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// High-level exception dispatcher.
///
/// # Parameters
///
/// - `excp` Exception information.
/// - `ctx`  Context information.
///
/// # Safety
///
/// This function is unsafe for the following conditions:
/// - It dereferences raw pointers.
/// - It accesses global variables.
///
#[no_mangle]
pub unsafe extern "C" fn do_exception(
    excp: *const ExceptionInformation,
    ctx: *const ContextInformation,
) {
    let excp: &ExceptionInformation = &*excp;
    let ctx: &ContextInformation = &*ctx;

    match HANDLER {
        Some(handler) => handler(excp, ctx),
        None => {
            info!("{:?}", excp);
            info!("{:?}", ctx);
            panic!("unhandled exception");
        },
    }
}
