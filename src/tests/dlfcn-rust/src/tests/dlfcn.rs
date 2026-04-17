// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::Error,
    mm::{
        Address,
        VirtualAddress,
    },
};
use ::syscall::dlfcn::{
    DlHandle,
    DlInfo,
    dladdr,
    dlclose,
    dlopen,
    dlsym,
};
use core::{
    ffi::{
        CStr,
        c_char,
        c_void,
    },
    ptr,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path to the shared library used by the non-PIE dlfcn tests.
const LIB_PATH: &str = "lib/libmul.so";

//==================================================================================================
// Compile-Time Assertions
//==================================================================================================

/// Asserts that `DlInfo` has the same size as the C `Dl_info_t` structure:
/// four pointer-sized fields (dli_fname, dli_fbase, dli_sname, dli_saddr).
const _: () = assert!(
    core::mem::size_of::<DlInfo>()
        == core::mem::size_of::<*const c_char>()
            + core::mem::size_of::<*const c_void>()
            + core::mem::size_of::<*const c_char>()
            + core::mem::size_of::<*const c_void>()
);

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Opens a dynamic load library.
fn open_library(path: &str) -> Result<DlHandle, Error> {
    dlopen(path, false)
}

/// Closes a dynamic load library.
fn close_library(handle: &DlHandle) -> Result<(), Error> {
    dlclose(handle)
}

/// Resolves a symbol in a dynamic load library.
fn resolve_symbol(handle: &DlHandle, symbol: &str) -> Result<VirtualAddress, Error> {
    dlsym(handle, symbol)
}

//==================================================================================================
// Test Functions
//==================================================================================================

/// Tests if dlopen() and dlclose() work.
fn test_dlopen_dlclose() -> Result<(), Error> {
    let handle = open_library(LIB_PATH)?;
    close_library(&handle)?;
    Ok(())
}

/// Tests if dlsym() works for functions and data symbols.
fn test_dlsym() -> Result<(), Error> {
    let handle = open_library(LIB_PATH)?;

    // Test add(1, 2) == 3.
    {
        let addr = resolve_symbol(&handle, "add")?;
        let add: unsafe extern "C" fn(i32, i32) -> i32 =
            unsafe { core::mem::transmute(addr.into_raw_value()) };
        assert!(unsafe { add(1, 2) } == 3, "add(1, 2) should return 3");
    }

    // Test fast_mul(7, 2) == 14.
    {
        let addr = resolve_symbol(&handle, "fast_mul")?;
        let fast_mul: unsafe extern "C" fn(i32, i32) -> i32 =
            unsafe { core::mem::transmute(addr.into_raw_value()) };
        assert!(unsafe { fast_mul(7, 2) } == 14, "fast_mul(7, 2) should return 14");
    }

    // Test slow_mul(3, 4) == 12.
    {
        let addr = resolve_symbol(&handle, "slow_mul")?;
        let slow_mul: unsafe extern "C" fn(i32, i32) -> i32 =
            unsafe { core::mem::transmute(addr.into_raw_value()) };
        assert!(unsafe { slow_mul(3, 4) } == 12, "slow_mul(3, 4) should return 12");
    }

    // Test multiply(7, 6) == 42.
    {
        let addr = resolve_symbol(&handle, "multiply")?;
        let multiply: unsafe extern "C" fn(i32, i32) -> i32 =
            unsafe { core::mem::transmute(addr.into_raw_value()) };
        assert!(unsafe { multiply(7, 6) } == 42, "multiply(7, 6) should return 42");
    }

    // Test VERSION == "0.0.1".
    {
        let addr = resolve_symbol(&handle, "VERSION")?;
        let version_ptr_ptr: *const *const c_char = addr.as_ptr().cast();
        let version_ptr = unsafe { *version_ptr_ptr };
        let version_str = unsafe { CStr::from_ptr(version_ptr) };
        assert!(version_str == c"0.0.1", "VERSION should be \"0.0.1\"");
    }

    // Test get_version() == "0.0.1".
    {
        let addr = resolve_symbol(&handle, "get_version")?;
        let get_version: unsafe extern "C" fn() -> *const c_char =
            unsafe { core::mem::transmute(addr.into_raw_value()) };
        let result_ptr = unsafe { get_version() };
        let result_str = unsafe { CStr::from_ptr(result_ptr) };
        assert!(result_str == c"0.0.1", "get_version() should return \"0.0.1\"");
    }

    close_library(&handle)?;
    Ok(())
}

/// Tests if dladdr() works.
fn test_dladdr() -> Result<(), Error> {
    let handle = open_library(LIB_PATH)?;

    let addr = resolve_symbol(&handle, "add")?;
    let add_raw_ptr = addr.as_ptr();

    let mut info = DlInfo {
        dli_fname: ptr::null(),
        dli_fbase: ptr::null(),
        dli_sname: ptr::null(),
        dli_saddr: ptr::null(),
    };
    dladdr(addr, &mut info)?;

    // Verify file name matches the library path (canonical form without leading /).
    let fname = unsafe { CStr::from_ptr(info.dli_fname) };
    assert!(fname == c"lib/libmul.so", "dli_fname should match the library path");

    // Verify base address is not null.
    assert!(!info.dli_fbase.is_null(), "dli_fbase should not be null");

    // Verify symbol name is "add".
    let sname = unsafe { CStr::from_ptr(info.dli_sname) };
    assert!(sname == c"add", "dli_sname should be \"add\"");

    // Verify symbol address matches what dlsym returned.
    assert!(
        info.dli_saddr == add_raw_ptr.cast::<c_void>(),
        "dli_saddr should match the address returned by dlsym"
    );

    close_library(&handle)?;
    Ok(())
}

//==================================================================================================
// Public Interface
//==================================================================================================

/// Runs every non-PIE dynamic linking test.
pub fn run() -> Result<(), Error> {
    test_dlopen_dlclose()?;
    test_dlsym()?;
    test_dladdr()?;
    Ok(())
}
