// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod dispatcher;
mod handler;
mod kcall_args;
mod kcall_error;
mod kcall_result;
mod kcall_success;

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::{
        mutex::{
            Mutex,
            MutexGuard,
        },
        semaphore::Semaphore,
    },
    SleepError,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Exports
//==================================================================================================

pub use handler::kcall_handler as handler;
pub use kcall_args::KcallArgs;
pub use kcall_error::KcallError;
pub use kcall_result::KcallResult;
pub use kcall_success::KcallSuccess;

//==================================================================================================
// Scoreboard
//==================================================================================================

static mut SCOREBOARD: Option<ScoreBoard> = None;

struct ScoreBoard {
    lock: Mutex,
    dispatched: Semaphore,
    handled: Semaphore,
    args: KcallArgs,
    ret: KcallResult,
}

impl ScoreBoard {
    fn init() {
        unsafe {
            SCOREBOARD = Some(ScoreBoard {
                lock: Mutex::new(),
                dispatched: Semaphore::new(0),
                handled: Semaphore::new(0),
                args: KcallArgs {
                    pid: ProcessIdentifier::from(i32::MAX),
                    tid: ThreadIdentifier::from(i32::MAX),
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                    arg3: 0,
                    number: 0,
                },
                ret: KcallResult::ok(),
            });
        }
    }

    pub fn get_mut() -> Result<&'static mut ScoreBoard, Error> {
        unsafe {
            if let Some(scoreboard) = SCOREBOARD.as_mut() {
                Ok(scoreboard)
            } else {
                let reason: &str = "uninitialized scoreboard";
                error!("{reason}");
                Err(Error::new(ErrorCode::TryAgain, reason))
            }
        }
    }

    ///
    /// # Description
    ///
    /// Dispatches a kernel to be executed by the kernel thread.
    ///
    /// # Parameters
    ///
    /// - `number`: Number of the kernel call.
    /// - `pid`: Identifier of the process that is invoking the kernel call.
    /// - `tid`: Identifier of the thread that is invoking the kernel call.
    /// - `arg0`: First kernel call argument.
    /// - `arg1`: Second kernel call argument.
    /// - `arg2`: Third kernel call argument.
    /// - `arg3`: Fourth kernel call argument.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the return value of the kernel call is returned. Otherwise, an
    /// error is returned instead.
    ///
    /// # Safety
    ///
    /// This function panics if the kernel process tries to sleep.
    ///
    /// This function is unsafe because it blocks the calling thread until it is woken up by another
    /// thread.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    /// - This function is invoked without holding any resources.
    ///
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn dispatch(
        &mut self,
        number: u32,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
    ) -> Result<KcallResult, SleepError> {
        let _guard: MutexGuard = self.lock.lock(None)?;
        self.args = KcallArgs {
            pid,
            tid,
            arg0,
            arg1,
            arg2,
            arg3,
            number,
        };
        self.dispatched.up().map_err(SleepError::Generic)?;
        self.handled.down()?;

        Ok(self.ret)
    }

    pub fn handle(&self) -> Result<&KcallArgs, Error> {
        self.dispatched.try_down()?;

        Ok(&self.args)
    }

    pub unsafe fn handled(&mut self, ret: KcallResult) -> Result<(), Error> {
        self.ret = ret;
        self.handled.up()
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init() {
    info!("initializing kernel call handler...");
    ScoreBoard::init();
}
