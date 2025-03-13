// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_int,
    c_void,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
//pthread_key_create()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_key_create(
    _key: *mut c_int,
    _destructor: Option<extern "C" fn(*mut c_void)>,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/505
    ::nvx::error!("pthread_key_create(): not implemented");
    ErrorCode::OperationNotSupported.into_errno()
}

//==================================================================================================
// pthread_key_delete()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_key_delete(_key: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/506
    ::nvx::error!("pthread_key_delete(): not implemented");
    ErrorCode::OperationNotSupported.into_errno()
}

//==================================================================================================
// pthread_getspecific()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_getspecific(_key: c_int) -> *mut c_void {
    // TODO: https://github.com/nanvix/nanvix/issues/504
    ::nvx::error!("pthread_getspecific(): not implemented");
    ::core::ptr::null_mut()
}

//==================================================================================================
// pthread_setspecific()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pthread_setspecific(_key: c_int, _value: *const c_void) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/521
    ::nvx::error!("pthread_setspecific(): not implemented");
    ErrorCode::OperationNotSupported.into_errno()
}
