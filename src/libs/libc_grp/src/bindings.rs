// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ptr,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
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

/// Group name reported for the single-user Nanvix system.
static GR_NAME: [u8; 5] = *b"root\0";

/// Placeholder password field (POSIX convention is a single `x`).
static GR_PASSWD: [u8; 2] = *b"x\0";

/// Empty, null-terminated member list shared by every returned entry.
static mut GR_MEM: [*const c_char; 1] = [ptr::null()];

/// Static `group` entry returned by [`getgrgid`] and [`getgrnam`]. POSIX allows the result to point
/// at static storage that is overwritten by subsequent calls.
static mut GR_ENTRY: group = group {
    gr_name: ptr::null(),
    gr_passwd: ptr::null(),
    gr_gid: 0,
    gr_mem: ptr::null(),
};

/// Tracks whether the sequential enumeration started by [`setgrent`]/[`getgrent`] has already
/// yielded the single synthetic `root` group. Reset by [`setgrent`] and [`endgrent`].
static GR_ENUM_DONE: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Populates the shared static [`group`] entry describing the `root` group with the supplied group
/// ID and returns a pointer to it.
///
/// # Safety
///
/// The returned pointer aliases mutable static storage and must not be used concurrently from
/// multiple threads.
///
unsafe fn fill_group(gid: gid_t) -> *mut group {
    // Populate the static entry through a raw pointer. `addr_of_mut!` avoids constructing a
    // reference to the mutable static (see `static_mut_refs`).
    let entry: *mut group = ptr::addr_of_mut!(GR_ENTRY);
    // SAFETY: `entry` points to the live, aligned `GR_ENTRY` static and the string/array pointers
    // refer to NUL-terminated static buffers that outlive any use of the returned entry.
    unsafe {
        (*entry).gr_name = GR_NAME.as_ptr() as *const c_char;
        (*entry).gr_passwd = GR_PASSWD.as_ptr() as *const c_char;
        (*entry).gr_gid = gid;
        (*entry).gr_mem = ptr::addr_of!(GR_MEM) as *const *const c_char;
    }
    entry
}

//==================================================================================================
// getgrgid()
//==================================================================================================

///
/// # Description
///
/// Returns the group database entry for the group identified by `gid`. Nanvix is effectively a
/// single-user system, so rather than consulting an `/etc/group` database this function returns a
/// statically-allocated [`group`] entry describing the `root` group, reporting the supplied `gid`
/// verbatim in `gr_gid`.
///
/// # Parameters
///
/// - `gid`: The group ID to report in the returned entry.
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
    // SAFETY: `fill_group()` only writes to the shared static entry and returns a pointer to it.
    unsafe { fill_group(gid) }
}

//==================================================================================================
// getgrnam()
//==================================================================================================

///
/// # Description
///
/// Returns the group database entry for the group with the name `name`. Nanvix only has the `root`
/// group, so the lookup ignores `name` and returns the statically-allocated [`group`] entry for the
/// root group (GID `0`).
///
/// # Parameters
///
/// - `name`: The group name to look up. Ignored on Nanvix.
///
/// # Returns
///
/// A pointer to a statically-allocated [`group`] entry describing the `root` group. Per POSIX, the
/// storage may be overwritten by subsequent calls.
///
/// # Safety
///
/// This function returns a pointer to static storage that must not be freed by the caller and may be
/// overwritten by subsequent calls. It is not thread-safe.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getgrnam(name: *const c_char) -> *mut group {
    // Nanvix only has the `root` group; the looked-up name is ignored.
    let _ = name;
    // SAFETY: `fill_group()` only writes to the shared static entry and returns a pointer to it.
    unsafe { fill_group(0) }
}

//==================================================================================================
// getgrouplist()
//==================================================================================================

///
/// # Description
///
/// Computes the list of group IDs that the user `user` belongs to. Nanvix is a single-user system
/// with no supplementary group memberships, so the resulting list contains only the caller-supplied
/// primary group `group`.
///
/// # Parameters
///
/// - `user`: The user whose groups are queried. Ignored on Nanvix.
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
/// The caller must ensure that `ngroups` points to a valid `int`, and that `groups` points to at
/// least `*ngroups` elements.
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
    // Nanvix has no supplementary group memberships; the user is ignored.
    let _ = user;

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
/// Returns the next entry from the group database. Nanvix is a single-user system whose synthetic
/// database holds only the `root` group, so the first call after a [`setgrent`] returns that entry
/// and subsequent calls return a null pointer until the enumeration is rewound.
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

    // SAFETY: `fill_group()` only writes to the shared static entry and returns a pointer to it.
    unsafe { fill_group(0) }
}

//==================================================================================================
// initgroups()
//==================================================================================================

///
/// # Description
///
/// Initializes the supplementary group access list of the calling process from the group database
/// for `user`, together with the additional group `group`. Nanvix is a single-user system with no
/// supplementary group memberships, so the access list is already correct and this operation
/// succeeds without modifying any state.
///
/// # Parameters
///
/// - `user`: The user whose group memberships would be consulted. Ignored on Nanvix.
/// - `group`: An additional group ID to include. Ignored on Nanvix.
///
/// # Returns
///
/// Returns `0` on success.
///
/// # Safety
///
/// This function does not dereference its arguments and is safe to call with any values.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn initgroups(user: *const c_char, group: gid_t) -> c_int {
    // Nanvix has no supplementary group memberships; the arguments are ignored.
    let _ = (user, group);
    0
}
