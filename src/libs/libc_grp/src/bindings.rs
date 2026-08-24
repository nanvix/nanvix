// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    cell::UnsafeCell,
    ffi::CStr,
    ptr,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sys::pm::GroupIdentifier;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    grp::group,
    sys_types::gid_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Static Storage
//==================================================================================================

/// Name of the synthetic root group.
static GR_NAME: [u8; 5] = *b"root\0";

/// Placeholder password field (POSIX convention is a single `x`).
static GR_PASSWD: [u8; 2] = *b"x\0";

/// Empty, null-terminated member list shared by every returned entry.
static mut GR_MEM: [*const c_char; 1] = [ptr::null()];

/// Shared storage for the synthetic root entry.
struct GroupEntry(UnsafeCell<group>);

// SAFETY: callers must serialize access to the POSIX static entry.
unsafe impl Sync for GroupEntry {}

/// Static root entry returned by the group database functions.
static GR_ENTRY: GroupEntry = GroupEntry(UnsafeCell::new(group {
    gr_name: GR_NAME.as_ptr() as *const c_char,
    gr_passwd: GR_PASSWD.as_ptr() as *const c_char,
    gr_gid: GroupIdentifier::ROOT.as_usize() as gid_t,
    gr_mem: ptr::addr_of!(GR_MEM) as *const *const c_char,
}));

/// Tracks whether the sequential enumeration started by [`setgrent`]/[`getgrent`] has already
/// yielded the single synthetic `root` group. Reset by [`setgrent`] and [`endgrent`].
static GR_ENUM_DONE: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Private Functions
//==================================================================================================

/// Checks whether `name` identifies root.
unsafe fn is_root_name(name: *const c_char) -> bool {
    if name.is_null() {
        return false;
    }

    // SAFETY: the caller provides a valid NUL-terminated name.
    unsafe { CStr::from_ptr(name) }.to_bytes() == b"root"
}

/// Returns the root group identifier at the POSIX boundary.
fn root_gid() -> gid_t {
    GroupIdentifier::ROOT.as_usize() as gid_t
}

//==================================================================================================
// getgrgid()
//==================================================================================================

///
/// # Description
///
/// Returns the synthetic `root` group entry. Other group IDs return a null pointer.
///
/// # Parameters
///
/// - `gid`: The group ID to look up.
///
/// # Returns
///
/// A pointer to a statically-allocated [`group`] entry. Per POSIX, the storage may be overwritten by
/// subsequent calls.
///
/// # Safety
///
/// This function returns a pointer to static storage that must not be freed by the caller and may be
/// overwritten by subsequent calls. It is not thread-safe.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getgrgid(gid: gid_t) -> *mut group {
    if gid != root_gid() {
        return ptr::null_mut();
    }

    GR_ENTRY.0.get()
}

//==================================================================================================
// getgrnam()
//==================================================================================================

///
/// # Description
///
/// Returns the synthetic `root` group entry. Other names return a null pointer.
///
/// # Parameters
///
/// - `name`: The group name to look up.
///
/// # Returns
///
/// A pointer to a statically-allocated [`group`] entry describing the `root` group. Per POSIX, the
/// storage may be overwritten by subsequent calls.
///
/// # Safety
///
/// `name` must point to a readable NUL-terminated string. This function returns a pointer to static
/// storage that must not be freed by the caller and may be overwritten by subsequent calls. It is
/// not thread-safe.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getgrnam(name: *const c_char) -> *mut group {
    // SAFETY: the caller provides a valid NUL-terminated group name.
    if !unsafe { is_root_name(name) } {
        return ptr::null_mut();
    }

    GR_ENTRY.0.get()
}

//==================================================================================================
// getgrouplist()
//==================================================================================================

///
/// # Description
///
/// Computes the group list for root. Nanvix has no supplementary groups, so the result contains
/// only the root group.
///
/// # Parameters
///
/// - `user`: The user whose groups are queried.
/// - `group`: The primary group ID to include in the result.
/// - `groups`: Buffer that receives the group IDs.
/// - `ngroups`: On input, the number of elements `groups` can hold; on output, the number of groups
///   found.
///
/// # Returns
///
/// On success, the number of groups (`1`) is returned. If the buffer was too small, `-1` is returned
/// and `*ngroups` is set to the number of groups required.
///
/// # Safety
///
/// `user` must point to a readable NUL-terminated string. The caller must ensure that `ngroups`
/// points to a valid `int`, and that `groups` points to at least `*ngroups` elements when the input
/// capacity is positive.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getgrouplist(
    user: *const c_char,
    group: gid_t,
    groups: *mut gid_t,
    ngroups: *mut c_int,
) -> c_int {
    // SAFETY: the caller provides a valid NUL-terminated user name.
    if !unsafe { is_root_name(user) } || group != root_gid() {
        return -1;
    }

    if ngroups.is_null() {
        return -1;
    }

    // SAFETY: the caller guarantees `ngroups` points to a valid `int`.
    let capacity: c_int = unsafe { *ngroups };

    // Exactly one group (the supplied primary group) is reported.
    if capacity >= 1 && !groups.is_null() {
        // SAFETY: `capacity >= 1` and `groups` is non-null, so the first element is writable.
        unsafe {
            groups.write(group);
            *ngroups = 1;
        }
        return 1;
    }

    // SAFETY: `ngroups` is non-null (checked above).
    unsafe {
        *ngroups = 1;
    }
    -1
}

//==================================================================================================
// setgrent()
//==================================================================================================

///
/// # Description
///
/// Rewinds the group database so that the next call to [`getgrent`] returns the first entry. On
/// Nanvix the database is synthetic and contains only the `root` group.
///
/// # Safety
///
/// This function mutates shared enumeration state and is not thread-safe.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn setgrent() {
    GR_ENUM_DONE.store(false, Ordering::Relaxed);
}

//==================================================================================================
// endgrent()
//==================================================================================================

///
/// # Description
///
/// Closes the group database, resetting the sequential enumeration state. On Nanvix there is no
/// backing file to close, so this only rewinds the synthetic enumeration.
///
/// # Safety
///
/// This function mutates shared enumeration state and is not thread-safe.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn endgrent() {
    GR_ENUM_DONE.store(false, Ordering::Relaxed);
}

//==================================================================================================
// getgrent()
//==================================================================================================

///
/// # Description
///
/// Returns the next entry from the group database. The synthetic database currently holds only the
/// `root` group, so the first call after a [`setgrent`] returns that entry and subsequent calls
/// return a null pointer until the enumeration is rewound.
///
/// # Returns
///
/// A pointer to a statically-allocated [`group`] entry, or a null pointer once every entry has been
/// enumerated.
///
/// # Safety
///
/// This function returns a pointer to static storage that must not be freed by the caller and may be
/// overwritten by subsequent calls. It is not thread-safe.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getgrent() -> *mut group {
    // Yield the single synthetic `root` group exactly once per enumeration.
    if GR_ENUM_DONE.swap(true, Ordering::Relaxed) {
        return ptr::null_mut();
    }

    GR_ENTRY.0.get()
}

//==================================================================================================
// initgroups()
//==================================================================================================

///
/// # Description
///
/// Initializes the supplementary group access list for root. Nanvix has no supplementary groups,
/// so valid root arguments succeed without modifying any state.
///
/// # Parameters
///
/// - `user`: The user whose group memberships would be consulted.
/// - `group`: An additional group ID to include.
///
/// # Returns
///
/// Returns `0` on success.
///
/// # Safety
///
/// `user` must point to a readable NUL-terminated string.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn initgroups(user: *const c_char, group: gid_t) -> c_int {
    // SAFETY: the caller provides a valid NUL-terminated user name.
    if unsafe { is_root_name(user) } && group == root_gid() {
        0
    } else {
        -1
    }
}
