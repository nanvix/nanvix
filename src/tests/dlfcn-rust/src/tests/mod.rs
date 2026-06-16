// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod dlfcn;
mod dlfcn_global;
mod dlfcn_pie;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every dynamic linking test.
pub fn run_all() -> Result<(), Error> {
    dlfcn::run()?;
    dlfcn_pie::run()?;
    // RTLD_GLOBAL tests run last because they permanently register symbols
    // in the global scope (matching Linux semantics where global scope
    // entries persist after dlclose).
    dlfcn_global::run()?;
    Ok(())
}
