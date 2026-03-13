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
use ::spin::MutexGuard;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::VirtualAddress,
};

//===================================================================================================
// dlsym()
//==================================================================================================

pub fn dlsym(handle: &DlHandle, symbol: &str) -> Result<VirtualAddress, Error> {
    ::syslog::trace!("dlsym(): handle={:?}, symbol={}", handle, symbol);

    // Handle the global scope sentinel (returned by dlopen(NULL)).
    if *handle == DlHandle::GLOBAL {
        return match super::global_symbol_lookup(symbol) {
            Some(addr) => Ok(VirtualAddress::from_raw_value(addr)),
            None => {
                let reason: &str = "symbol not found in global scope";
                ::syslog::error!("dlsym(): {}", reason);
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        };
    }

    // Get dynamic file.
    match DYNAMIC_LIBRARY_REGISTRY.lock().get_mut(handle) {
        Some(dlfile) => {
            let dlfile: MutexGuard<'_, DynamicLibrary> = dlfile.lock();

            match dlfile.lookup(symbol)? {
                Some((base, offset)) => Ok(VirtualAddress::from_raw_value(base + offset)),
                None => {
                    let reason: &str = "symbol not found";
                    ::syslog::error!("dlsym(): {}", reason);
                    Err(Error::new(ErrorCode::NoSuchEntry, reason))
                },
            }
        },
        None => {
            let reason: &str = "dynamic library file not open";
            ::syslog::error!("dlinfo(): {}", reason);
            Err(Error::new(ErrorCode::BadFile, reason))
        },
    }
}
