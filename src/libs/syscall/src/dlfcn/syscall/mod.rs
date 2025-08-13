// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//====================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

mod dladdr;
mod dlclose;
mod dlopen;
mod dlsym;
mod dynlib;

//==================================================================================================
// Imports
//===================================================================================================

pub use self::dynlib::DlHandle;

use self::dynlib::DynamicLibrary;
use ::alloc::{
    collections::btree_map::BTreeMap,
    sync::Arc,
};
use ::spin::{
    Lazy,
    Mutex,
};

//==================================================================================================

static DYNAMIC_LIBRARY_REGISTRY: Lazy<Mutex<BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Exports
//==================================================================================================

pub use dladdr::dladdr;
pub use dlclose::dlclose;
pub use dlopen::dlopen;
pub use dlsym::dlsym;
