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
    dlclose,
    dlinit,
    dlopen,
    dlsym,
};
use core::ffi::{
    CStr,
    c_char,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path to the shared library used by the PIE dlfcn tests.
const LIB_PATH: &str = "lib/libmul-pie.so";

//==================================================================================================
// Exported Globals
//==================================================================================================

/// A global variable exported by the main executable (via --export-dynamic).
/// The PIE tests verify that this symbol can be resolved through the global
/// symbol table using `DlHandle::GLOBAL` (equivalent to RTLD_DEFAULT in C).
#[used]
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static exe_global_value: i32 = 42;

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

/// Tests if symbols exported by the main executable can be resolved through the
/// global symbol table.
///
/// NOTE: The Rust dlfcn API does not support `dlopen(NULL)`, which in C returns
/// a handle to the global symbol scope. Instead, we use `DlHandle::GLOBAL` with
/// `dlsym()`, which is equivalent to using `RTLD_DEFAULT` in C. The C test also
/// verifies that `dlclose()` on the NULL-opened handle is a no-op; this cannot
/// be tested here because `DlHandle::GLOBAL` is a sentinel value not backed by
/// a registry entry.
fn test_dlopen_null() -> Result<(), Error> {
    // Ensure the global symbol table is populated from the executable's
    // .dynsym/.dynstr sections (requires --export-dynamic linker flag).
    dlinit();

    // Prevent the compiler from optimizing away the global.
    core::hint::black_box(&exe_global_value);

    // Resolve the symbol through the global scope (RTLD_DEFAULT equivalent).
    let addr = dlsym(&DlHandle::GLOBAL, "exe_global_value")?;
    let value_ptr: *const i32 = addr.as_ptr().cast();
    let value = unsafe { *value_ptr };
    assert!(value == 42, "exe_global_value should be 42");

    // A second lookup should return the same address.
    let addr2 = dlsym(&DlHandle::GLOBAL, "exe_global_value")?;
    assert!(
        addr.as_ptr() == addr2.as_ptr(),
        "repeated global lookups should return the same address"
    );

    Ok(())
}

/// Tests if dlsym() works on a shared library loaded from a PIE executable.
fn test_dlsym() -> Result<(), Error> {
    let handle = open_library(LIB_PATH)?;

    // Test add(1, 2) == 3.
    {
        let addr = resolve_symbol(&handle, "add")?;
        let add: unsafe extern "C" fn(i32, i32) -> i32 =
            unsafe { core::mem::transmute(addr.into_raw_value()) };
        assert!(unsafe { add(1, 2) } == 3, "add(1, 2) should return 3");
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

    close_library(&handle)?;
    Ok(())
}

//==================================================================================================
// Public Interface
//==================================================================================================

/// Runs every PIE dynamic linking test.
///
/// NOTE: The C `dlfcn-pie-c` test is built as a Position-Independent Executable
/// with `-pie -rdynamic -Wl,--no-dynamic-linker`. In this Rust binary, PIE is
/// the default compilation mode and `--export-dynamic` is set in `build.rs` to
/// export symbols. If a fully separate PIE binary with different linker flags is
/// required, a dedicated Cargo binary target would be needed.
pub fn run() -> Result<(), Error> {
    test_dlopen_null()?;
    test_dlsym()?;
    Ok(())
}
