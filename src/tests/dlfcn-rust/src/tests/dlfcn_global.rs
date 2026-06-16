// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::Error,
    mm::Address,
};
use ::syscall::dlfcn::{
    DlHandle,
    dlclose,
    dlopen,
    dlsym,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path to the shared library used by the RTLD_GLOBAL tests.
const LIB_PATH: &str = "lib/libmul.so";

//==================================================================================================
// Test Functions
//==================================================================================================

/// Tests that dlopen with global=true succeeds and the library's symbols
/// become visible through the global scope (DlHandle::GLOBAL).
fn test_global_registers_symbols() -> Result<(), Error> {
    // Load the library with RTLD_GLOBAL.
    let handle: DlHandle = dlopen(LIB_PATH, true)?;

    // The "add" symbol should now be in the global symbol table.
    let addr = dlsym(&DlHandle::GLOBAL, "add")?;
    let add: unsafe extern "C" fn(i32, i32) -> i32 =
        unsafe { core::mem::transmute(addr.into_raw_value()) };
    assert!(unsafe { add(10, 20) } == 30, "add(10, 20) should return 30");

    dlclose(&handle)?;
    Ok(())
}

/// Tests that re-opening a library with global=true promotes it to global
/// scope. Verifies that the cache-hit path in dlopen correctly calls
/// register_library_in_global_scope when global=true is requested on a
/// previously loaded library.
///
/// NOTE: Because test_global_registers_symbols runs first and registers all
/// of libmul.so's symbols globally, this test cannot verify the negative case
/// (that global=false does NOT register symbols). Full isolation would require
/// a separate library. Instead, this test verifies the promotion plumbing
/// (same handle returned, global lookup matches handle lookup).
fn test_global_promotion_on_reopen() -> Result<(), Error> {
    // Load with global=false first.
    let handle: DlHandle = dlopen(LIB_PATH, false)?;

    // "multiply" should be accessible via the specific handle.
    let addr = dlsym(&handle, "multiply")?;
    let multiply: unsafe extern "C" fn(i32, i32) -> i32 =
        unsafe { core::mem::transmute(addr.into_raw_value()) };
    assert!(unsafe { multiply(7, 6) } == 42, "multiply(7, 6) should return 42");

    // Re-open the same library with global=true to promote it.
    let handle2: DlHandle = dlopen(LIB_PATH, true)?;

    // Should be the same handle (cache hit).
    assert!(
        handle.as_mut_ptr() == handle2.as_mut_ptr(),
        "re-opening same library should return same handle"
    );

    // "multiply" should now be visible through the global scope.
    let global_addr = dlsym(&DlHandle::GLOBAL, "multiply")?;
    assert!(
        addr.into_raw_value() == global_addr.into_raw_value(),
        "global lookup should return the same address as handle lookup"
    );

    dlclose(&handle)?;
    Ok(())
}

/// Tests that pinning prevents dlclose from unloading a global library.
/// After dlclose, symbols registered via RTLD_GLOBAL should remain valid.
fn test_global_pin_survives_dlclose() -> Result<(), Error> {
    // Load with RTLD_GLOBAL.
    let handle: DlHandle = dlopen(LIB_PATH, true)?;

    // Get the address of "add" from the global scope.
    let addr_before = dlsym(&DlHandle::GLOBAL, "add")?;

    // Close the library handle. The pin should keep it alive.
    dlclose(&handle)?;

    // The symbol should still be resolvable from the global scope.
    let addr_after = dlsym(&DlHandle::GLOBAL, "add")?;
    assert!(
        addr_before.into_raw_value() == addr_after.into_raw_value(),
        "global symbol should survive dlclose due to pinning"
    );

    Ok(())
}

//==================================================================================================
// Public Interface
//==================================================================================================

/// Runs all RTLD_GLOBAL dynamic linking tests.
///
/// IMPORTANT: Test ordering matters. These tests share process-global state
/// (the `GLOBAL_SYMBOL_TABLE` and `GLOBAL_PINNED_LIBRARIES`), and global
/// registrations persist for the lifetime of the process (matching Linux
/// semantics). Specifically:
///
/// - `test_global_registers_symbols` registers "add" globally. This
///   registration persists through subsequent tests.
/// - `test_global_promotion_on_reopen` uses a different symbol ("multiply")
///   to verify that promotion works independently of the prior registration.
/// - `test_global_pin_survives_dlclose` opens the library with RTLD_GLOBAL
///   and verifies that its pinned symbols remain available after dlclose.
///
/// This module must run AFTER all non-global dlfcn tests (see `mod.rs`)
/// to avoid contaminating their global scope expectations.
pub fn run() -> Result<(), Error> {
    test_global_registers_symbols()?;
    test_global_promotion_on_reopen()?;
    test_global_pin_survives_dlclose()?;
    Ok(())
}
