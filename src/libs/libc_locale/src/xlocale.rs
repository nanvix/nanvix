// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicPtr,
    Ordering,
};
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_void,
};

//==================================================================================================
// Types
//==================================================================================================

/// Opaque locale handle.
#[allow(non_camel_case_types)]
pub type locale_t = *mut c_void;

//==================================================================================================
// Constants
//==================================================================================================

/// Handle selecting the process-global locale.
pub const LC_GLOBAL_LOCALE: locale_t = !0usize as locale_t;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Currently installed locale handle.
///
/// A null pointer denotes the global locale managed by `setlocale()` (POSIX `LC_GLOBAL_LOCALE`),
/// which is also the initial state. Nanvix supports only the C/POSIX locale, so the stored value is
/// purely bookkeeping for `uselocale()`'s query/swap contract.
///
/// # Caveat
///
/// POSIX specifies `uselocale()` state as **per-thread**. This uses a single process-global atomic
/// instead (consistent with `setlocale()`'s existing global state). That is sound while only the
/// C/POSIX locale exists — every `*_l` function ignores the handle — but switching to true
/// thread-local storage would be required before adding genuine multi-locale support.
static CURRENT_LOCALE: AtomicPtr<c_void> = AtomicPtr::new(::core::ptr::null_mut());

/// A single stable, non-null object whose address is the one and only `locale_t`.
///
/// Only the C/POSIX locale exists, so every locale handle aliases this object. It is never
/// dereferenced; only its address is used as an opaque, non-null sentinel.
static C_LOCALE_OBJECT: u8 = 0;

//==================================================================================================
// Private Functions
//==================================================================================================

/// Returns the canonical, non-null handle for the C/POSIX locale.
fn c_locale_handle() -> locale_t {
    (&raw const C_LOCALE_OBJECT).cast::<c_void>().cast_mut()
}

/// Returns POSIX `LC_GLOBAL_LOCALE`, i.e. `(locale_t) -1`.
fn lc_global_locale() -> locale_t {
    LC_GLOBAL_LOCALE
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// # Description
///
/// Creates a locale object. Nanvix supports only the C/POSIX locale, so the canonical handle is
/// returned regardless of the requested categories, locale name, or base object.
///
/// # Parameters
///
/// - `category_mask`: Bitwise OR of `LC_*_MASK` categories to set (ignored).
/// - `locale`: Name of the locale to load (ignored; not dereferenced).
/// - `base`: An existing locale object to modify (ignored; not dereferenced).
///
/// # Returns
///
/// The canonical, non-null C/POSIX locale handle.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn newlocale(
    _category_mask: c_int,
    _locale: *const c_char,
    _base: locale_t,
) -> locale_t {
    c_locale_handle()
}

/// # Description
///
/// Releases a locale object. The single C/POSIX handle is static, so this is a no-op.
///
/// # Parameters
///
/// - `locobj`: The locale object to release (ignored; never freed).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn freelocale(_locobj: locale_t) {}

/// # Description
///
/// Installs `newloc` as the current locale and returns the previously installed one. A null
/// argument queries the current locale without changing it (POSIX `uselocale((locale_t) 0)`).
///
/// # Parameters
///
/// - `newloc`: The locale to install, `LC_GLOBAL_LOCALE` to restore the global locale, or a null
///   pointer to query the current locale without changing it.
///
/// # Returns
///
/// The locale that was installed before the call, or `LC_GLOBAL_LOCALE` if the global locale was
/// active.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn uselocale(newloc: locale_t) -> locale_t {
    // A null argument is a pure query of the current locale.
    if newloc.is_null() {
        let current: *mut c_void = CURRENT_LOCALE.load(Ordering::Relaxed);
        return if current.is_null() {
            lc_global_locale()
        } else {
            current
        };
    }

    // `LC_GLOBAL_LOCALE` is stored internally as null so that a later query reports it correctly.
    let to_store: *mut c_void = if newloc == lc_global_locale() {
        ::core::ptr::null_mut()
    } else {
        newloc
    };
    let previous: *mut c_void = CURRENT_LOCALE.swap(to_store, Ordering::Relaxed);
    if previous.is_null() {
        lc_global_locale()
    } else {
        previous
    }
}

/// # Description
///
/// Duplicates a locale object. Every handle aliases the single C/POSIX locale, so the canonical
/// handle is returned.
///
/// # Parameters
///
/// - `locobj`: The locale object to duplicate (ignored; not dereferenced).
///
/// # Returns
///
/// The canonical, non-null C/POSIX locale handle.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn duplocale(_locobj: locale_t) -> locale_t {
    c_locale_handle()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
