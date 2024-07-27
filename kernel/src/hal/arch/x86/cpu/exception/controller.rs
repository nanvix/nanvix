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
/// Inner state of the exception controller.
///
struct ExceptionControllerInner {
    handler: Option<ExceptionHandler>,
}

///
/// # Description
///
/// A type that represents an exception controller.
///
pub struct ExceptionController(Option<&'static mut ExceptionControllerInner>);

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Exception controller. This should be a global variable to enable access from the low-level
/// exception dispatcher.
///
static mut CONTROLLER: ExceptionControllerInner = ExceptionControllerInner { handler: None };

///
/// # Description
///
/// Reference to the exception controller. This enables a single reference to the exception
/// controller from synchronous path.
///
static mut CONTROLLER_REF: Option<&'static mut ExceptionControllerInner> =
    unsafe { Some(&mut CONTROLLER) };

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
    /// Upon successful completion the exception controller is returned. Upon
    /// failure, an error is returned instead.
    ///
    pub fn init() -> Result<Self, Error> {
        unsafe {
            match CONTROLLER_REF.take() {
                Some(controller) => Ok(ExceptionController(Some(controller))),
                None => {
                    let reason: &str = "exception controller already initialized";
                    error!("init(): {}", reason);
                    Err(Error::new(ErrorCode::ResourceBusy, reason))
                },
            }
        }
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
    pub fn register_handler(&mut self, handler: ExceptionHandler) {
        trace!("register_handler(): handler={:?}", handler);
        self.0.as_mut().unwrap().handler = Some(handler);
    }
}

impl Drop for ExceptionController {
    fn drop(&mut self) {
        unsafe {
            // Safety: the following call to unwrap is safe because the controller is not `None`.
            CONTROLLER_REF = Some(self.0.take().unwrap());
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
#[no_mangle]
pub extern "C" fn do_exception(excp: *const ExceptionInformation, ctx: *const ContextInformation) {
    let excp: &ExceptionInformation = unsafe { &*excp };
    let ctx: &ContextInformation = unsafe { &*ctx };

    match unsafe { CONTROLLER.handler } {
        Some(handler) => handler(excp, ctx),
        None => {
            info!("{:?}", excp);
            info!("{:?}", ctx);
            panic!("unhandled exception");
        },
    }
}
