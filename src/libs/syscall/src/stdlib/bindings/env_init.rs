// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::libc_stdlib::env_table;
use ::sysapi::ffi::c_char;

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
/// # Note
///
/// Tilde expansion (`~` → `$HOME`) is performed client-side by the syscall layer (see
/// [`crate::path::expand_path`]) before paths are sent to vfsd via IPC. The `HOME` value is read
/// from the process-local environment table populated by this function.
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
pub unsafe extern "C" fn __nanvix_env_init(envp: *const *const c_char) {
    env_table::init_from_raw(envp);
}
