// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    event,
    ipc,
    kcall::ScoreBoard,
    pm::ProcessManager,
};
use ::sys::number::KcallNumber;

//==================================================================================================
//  Standalone Functions
//==================================================================================================

///
/// # Description
///
/// High-level kernel call dispatcher.
///
/// # Parameters
///
/// - `arg0`: First kernel call argument.
/// - `arg1`: Second kernel call argument.
/// - `arg2`: Third kernel call argument.
/// - `arg3`: Fourth kernel call argument.
/// - `arg4`: Fifth kernel call argument.
/// - `number`: Number of the kernel call.
///
#[no_mangle]
pub extern "C" fn do_kcall(number: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i32 {
    match KcallNumber::from(number) {
        // Handle `getpid()` locally.
        KcallNumber::GetPid => match ProcessManager::get_pid() {
            Ok(pid) => pid.into(),
            Err(e) => e.code.into_errno(),
        },
        // Handle `gettid()` locally.
        KcallNumber::GetTid => match ProcessManager::get_tid() {
            Ok(tid) => tid.into(),
            Err(e) => e.code.into_errno(),
        },
        KcallNumber::Exit => match ProcessManager::exit(arg0 as i32) {
            Ok(_) => unsafe { core::hint::unreachable_unchecked() },
            Err(e) => e.code.into_errno(),
        },
        KcallNumber::Recv => ipc::recv(arg0 as usize),
        KcallNumber::Resume => event::resume(arg0 as usize),
        // Dispatch kernel call for remote execution.
        _ => match ScoreBoard::get_mut() {
            Ok(scoreboard) => match scoreboard.dispatch(number, arg0, arg1, arg2, arg3) {
                Ok(result) => result,
                Err(e) => e.code.into_errno(),
            },

            Err(e) => e.code.into_errno(),
        },
    }
}
