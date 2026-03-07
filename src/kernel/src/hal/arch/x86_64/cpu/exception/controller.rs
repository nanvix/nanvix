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
pub type ExceptionHandler = fn(&ExceptionInformation, &mut ContextInformation);

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
            error!("{reason}");
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
        trace!("handler={:?}", handler);

        // Check if the handler is already set.
        if HANDLER.is_some() {
            let reason: &str = "exception handler already set";
            error!("{}", reason);
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
/// - `ctx`  Context information.
/// - `excp` Exception information.
///
/// # Note
///
/// On x86_64, the hooks.S assembly passes the context pointer in RDI (first argument) and the
/// exception information pointer in RSI (second argument). This is the opposite order from x86.
///
/// # Safety
///
/// This function is unsafe for the following conditions:
/// - It dereferences raw pointers.
/// - It accesses global variables.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_exception(
    ctx: *const ContextInformation,
    excp: *const ExceptionInformation,
) {
    let excp: &ExceptionInformation = &*excp;
    // SAFETY: the pointer comes from hooks.S and refers to the saved register block on the kernel
    // stack. We are the only consumer; casting to &mut is safe so that signal delivery can modify
    // the context before returning to user space.
    let ctx: &mut ContextInformation = &mut *(ctx as *mut ContextInformation);

    match HANDLER {
        Some(handler) => handler(excp, ctx),
        None => {
            error!("{:?}", excp);
            error!("{:?}", ctx);
            panic!("unhandled exception");
        },
    }
}
