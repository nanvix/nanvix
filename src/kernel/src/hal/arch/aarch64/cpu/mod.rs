// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

mod context;
mod exception;
#[path = "../../shared/cpu/exception_controller.rs"]
mod exception_controller;
mod fpu;
mod interrupt;
mod sigframe;

use ::sys::error::Error;
pub use context::{
    forge_user_stack,
    ContextInformation,
    SignalCpuContext,
};
pub use exception::ExceptionInformation;
pub use exception_controller::ExceptionController;
pub use fpu::{
    capture_fpu,
    install_fpu,
    FpuState,
};
pub use interrupt::{
    InterruptController,
    InterruptHandler,
    InterruptNumber,
};
pub use sigframe::{
    join_kcall_result,
    prepare_kcall_restart,
    read_trap_context,
    read_user_sp,
    redirect_to_handler,
    restore_trap_context,
    returning_to_user,
};

pub(super) unsafe fn enable_user_access() {
    unsafe {
        fpu::enable_user_access();
    }
}

pub(super) unsafe fn disable_user_access() {
    unsafe {
        fpu::disable_user_access();
    }
}

pub fn init() -> Result<(), Error> {
    unsafe {
        fpu::init();
        exception::init_vectors();
        interrupt::init();
    }
    Ok(())
}

#[cfg(feature = "smp")]
pub fn initialize_application_core() {
    unsafe {
        exception::init_vectors();
        interrupt::init_cpu_interface();
    }
}
