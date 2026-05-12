// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::libc_stdlib::env_table;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the environment table from a raw `envp` pointer. This function should be called
/// once during process startup to populate the environment from the kernel-provided environment
/// page.
///
/// # Parameters
///
/// - `envp`: A pointer to a null-terminated array of null-terminated `KEY=VALUE` C strings. If
///   null, the environment table is left empty.
///
/// # Returns
///
/// Always returns `0`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if and only if:
/// - `envp` is either null or points to a valid null-terminated array of null-terminated C strings.
/// - Each string in the array remains valid for the duration of this call.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nanvix_env_init(envp: *const *const c_char) -> c_int {
    // Register the setenv callback for HOME synchronization.
    #[cfg(feature = "standalone")]
    env_table::register_setenv_callback(on_setenv);

    // NOTE: `init_from_raw` uses `insert_entry` directly (not `set`), so the callback
    // registered above is intentionally *not* invoked for each initial variable. HOME is
    // explicitly synchronized below instead.
    env_table::init_from_raw(envp);

    // If HOME is set, synchronize with VFS tilde expansion.
    #[cfg(feature = "standalone")]
    {
        let home: *const c_char = env_table::get("HOME");
        if !home.is_null() {
            if let Ok(home_str) = ::core::ffi::CStr::from_ptr(home).to_str() {
                if let Err(e) = ::nvx::vfs::set_home(home_str) {
                    ::syslog::warn!("__nanvix_env_init(): failed to set VFS home (error={e:?})");
                }
            }
        }
    }

    0
}

//==================================================================================================
// Callback Functions
//==================================================================================================

/// Callback invoked by `env_table::set()` after a variable is written. Synchronizes the VFS home
/// directory when HOME is updated.
#[cfg(feature = "standalone")]
fn on_setenv(key: &str, value: &str) {
    if key == "HOME" {
        if let Err(e) = ::nvx::vfs::set_home(value) {
            ::syslog::warn!("on_setenv(): failed to set VFS home (error={e:?})");
        }
    }
}
