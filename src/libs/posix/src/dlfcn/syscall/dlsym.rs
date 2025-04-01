// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use super::dynlib::DlHandle;
use crate::dlfcn::syscall::{
    dynlib::DynamicLibrary,
    DYNAMIC_LIBRARY_REGISTRY,
};
use ::nvx::{
    mm::{
        Address,
        VirtualAddress,
    },
    sys::error::{
        Error,
        ErrorCode,
    },
};
use ::spin::MutexGuard;

//===================================================================================================
// dlsym()
//==================================================================================================

pub fn dlsym(handle: &DlHandle, symbol: &str) -> Result<VirtualAddress, Error> {
    ::nvx::trace!("dlsym(): handle={:?}, symbol={}", handle, symbol);

    // Get dynamic file.
    match DYNAMIC_LIBRARY_REGISTRY.lock().get_mut(handle) {
        Some(dlfile) => {
            let dlfile: MutexGuard<'_, DynamicLibrary> = dlfile.lock();

            match dlfile.lookup(symbol, false) {
                Some(addr) => Ok(VirtualAddress::from_raw_value(addr.into_raw_value())),
                None => {
                    let reason: &str = "symbol not found";
                    ::nvx::error!("dlsym(): {}", reason);
                    Err(Error::new(ErrorCode::NoSuchEntry, reason))
                },
            }
        },
        None => {
            let reason: &str = "dynamic library file not open";
            ::nvx::error!("dlinfo(): {}", reason);
            Err(Error::new(ErrorCode::BadFile, reason))
        },
    }
}
