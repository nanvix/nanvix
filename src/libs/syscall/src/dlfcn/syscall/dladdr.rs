// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::dlfcn::{
    syscall::{
        dynlib::{
            DlHandle,
            DynamicLibrary,
        },
        DYNAMIC_LIBRARY_REGISTRY,
    },
    DlInfo,
};
use ::alloc::{
    collections::btree_map::BTreeMap,
    sync::Arc,
};
use ::spin::{
    Mutex,
    MutexGuard,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        VirtualAddress,
    },
};
use ::sysapi::ffi::c_void;

//==================================================================================================
// dladdr()
//==================================================================================================

/// Returns information about the symbol at the given address.
pub fn dladdr(addr: VirtualAddress, dlinfo: &mut DlInfo) -> Result<(), Error> {
    ::syslog::trace!("dladdr(): addr={:#x?}", addr);
    let registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
        DYNAMIC_LIBRARY_REGISTRY.lock();

    for (_dlname, library) in registry.iter() {
        let library = library.lock();
        if let Some((dname, fbase, sname, saddr)) = library.query(addr) {
            dlinfo.dli_fname = dname;
            dlinfo.dli_fbase = fbase.into_raw_value() as *const c_void;
            dlinfo.dli_sname = sname;
            dlinfo.dli_saddr = saddr.into_raw_value() as *const c_void;

            return Ok(());
        }
    }

    let reason: &str = "symbol not found";
    let error: Error = Error::new(ErrorCode::NoSuchEntry, reason);
    Err(error)
}
