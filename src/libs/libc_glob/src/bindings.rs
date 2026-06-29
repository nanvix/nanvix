// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    ffi::CString,
    vec::Vec,
};
use ::core::{
    ffi::CStr,
    mem::size_of,
    ptr,
};
use ::libc_fnmatch::fnmatch::fnmatch;
use ::sysapi::{
    dirent::dirent,
    errno::{
        __errno_location,
        ENOENT,
        ENOTDIR,
    },
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    sys_stat::{
        file_type::S_ISDIR,
        stat as stat_t,
    },
    sys_types::c_size_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Constants
//==================================================================================================

//
// Flag bits accepted in the `flags` argument, mirroring `<glob.h>`.
//
/// Stop the scan and return on read errors.
const GLOB_ERR: c_int = 0x0001;
/// Append a `'/'` to each matched pathname that is a directory.
const GLOB_MARK: c_int = 0x0002;
/// Do not sort the matched pathnames.
const GLOB_NOSORT: c_int = 0x0004;
/// Reserve `gl_offs` leading slots in `gl_pathv`.
const GLOB_DOOFFS: c_int = 0x0008;
/// Return the pattern itself when it matches no pathname.
const GLOB_NOCHECK: c_int = 0x0010;
/// Append the generated pathnames to those of a previous call.
const GLOB_APPEND: c_int = 0x0020;
/// Disable backslash escaping in the pattern.
const GLOB_NOESCAPE: c_int = 0x0040;

//
// Return values, mirroring `<glob.h>`.
//
/// An attempt to allocate memory failed.
const GLOB_NOSPACE: c_int = 1;
/// The scan was stopped because a directory could not be read.
const GLOB_ABORTED: c_int = 2;
/// The pattern did not match any existing pathname.
const GLOB_NOMATCH: c_int = 3;

//
// Flag bits passed to `fnmatch()`, mirroring `<fnmatch.h>`.
//
/// A backslash does not quote the following character.
const FNM_NOESCAPE: c_int = 1 << 1;
/// A leading `'.'` is matched only by an explicit `'.'` in the pattern.
const FNM_PERIOD: c_int = 1 << 2;

//==================================================================================================
// External Functions
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn opendir(dirname: *const c_char) -> *mut c_void;
    fn readdir(dirp: *mut c_void) -> *mut dirent;
    fn closedir(dirp: *mut c_void) -> c_int;
    fn stat(pathname: *const c_char, statbuf: *mut stat_t) -> c_int;
}

//==================================================================================================
// Structures
//==================================================================================================

/// Mirror of the C `glob_t` result structure declared in `<glob.h>`.
///
/// The fields match the C ABI layout `{ size_t gl_pathc; char **gl_pathv; size_t gl_offs; }` and are
/// read and written through the raw pointer supplied by the caller.
#[repr(C)]
struct GlobT {
    gl_pathc: usize,
    gl_pathv: *mut *mut c_char,
    gl_offs: usize,
}

/// Internal control-flow signal raised when a directory scan must stop early at the caller's
/// request (via `errfunc` or `GLOB_ERR`); it is reported to the caller as `GLOB_ABORTED`.
enum GlobError {
    /// The scan was aborted.
    Aborted,
}

/// Mutable state threaded through a single [`glob()`] expansion.
struct GlobCtx {
    /// The `flags` argument passed to [`glob()`].
    flags: c_int,
    /// Optional caller error callback, invoked when a directory cannot be opened or read.
    errfunc: Option<extern "C" fn(epath: *const c_char, eerrno: c_int) -> c_int>,
    /// Whether backslash escaping is disabled (`GLOB_NOESCAPE`).
    noescape: bool,
    /// Whether the pattern ended with a `'/'`, restricting matches to directories.
    dir_only: bool,
    /// Matched pathnames collected so far. Each entry pairs the pathname with a flag telling
    /// whether a trailing `'/'` must be appended (set for directories under `GLOB_MARK` or a
    /// directory-only pattern); the mark is applied after sorting.
    matches: Vec<(Vec<u8>, bool)>,
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `true` if `component` contains an unescaped `'*'`, `'?'`, or `'['` and therefore must be
/// expanded against the contents of a directory rather than looked up as a literal name.
fn has_magic(component: &[u8], noescape: bool) -> bool {
    let mut i: usize = 0;
    while i < component.len() {
        let byte: u8 = component[i];
        if !noescape && byte == b'\\' {
            // Skip the escaped byte; an escaped metacharacter is not magic.
            i += 2;
        } else if byte == b'*' || byte == b'?' || byte == b'[' {
            return true;
        } else {
            i += 1;
        }
    }
    false
}

/// Removes backslash escapes from a literal pattern component, yielding the byte string to look up
/// on disk. When `noescape` is set the component is returned verbatim.
fn unescape(component: &[u8], noescape: bool) -> Vec<u8> {
    if noescape {
        return component.to_vec();
    }
    let mut out: Vec<u8> = Vec::with_capacity(component.len());
    let mut i: usize = 0;
    while i < component.len() {
        if component[i] == b'\\' && i + 1 < component.len() {
            out.push(component[i + 1]);
            i += 2;
        } else {
            out.push(component[i]);
            i += 1;
        }
    }
    out
}

/// Joins directory `prefix` and entry `name` with a single `'/'` separator, avoiding a doubled
/// separator after a root prefix and omitting the separator entirely when `prefix` is empty (a
/// relative match in the current working directory).
fn join(prefix: &[u8], name: &[u8]) -> Vec<u8> {
    if prefix.is_empty() {
        return name.to_vec();
    }
    let mut out: Vec<u8> = Vec::with_capacity(prefix.len() + 1 + name.len());
    out.extend_from_slice(prefix);
    if prefix.last() != Some(&b'/') {
        out.push(b'/');
    }
    out.extend_from_slice(name);
    out
}

/// Builds a null-terminated copy of `bytes` in `malloc()`ed storage so it can be handed to the C
/// caller and later released by [`globfree()`]. Returns a null pointer if allocation fails.
///
/// # Safety
///
/// When non-null, the returned pointer owns a `malloc()` allocation of `bytes.len() + 1` bytes.
unsafe fn dup_to_malloc(bytes: &[u8]) -> *mut c_char {
    let copy: *mut u8 = malloc((bytes.len() + 1) as c_size_t).cast::<u8>();
    if copy.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), copy, bytes.len());
    *copy.add(bytes.len()) = 0;
    copy.cast::<c_char>()
}

/// Publishes the collected `paths` into the caller's `glob_t`, honoring `GLOB_APPEND` and
/// `GLOB_DOOFFS`.
///
/// On success the structure owns a freshly `malloc()`ed, null-terminated `gl_pathv` array (with any
/// previous entries preserved under `GLOB_APPEND`), and `gl_offs` records the offset actually used,
/// so that [`globfree()`] can later release exactly the pathnames allocated here.
///
/// # Safety
///
/// `pglob` must point to a valid `glob_t`; under `GLOB_APPEND` its existing `gl_pathv` must have
/// been produced by a previous [`glob()`] call.
unsafe fn finalize(pglob: *mut GlobT, flags: c_int, paths: &[Vec<u8>]) -> c_int {
    let append: bool = (flags & GLOB_APPEND) != 0 && !(*pglob).gl_pathv.is_null();
    let old_count: usize = if append { (*pglob).gl_pathc } else { 0 };
    // When appending, the previous call already laid out `gl_pathv` using its own `gl_offs`, so the
    // same offset must be reused to carry those entries over correctly even if the caller did not
    // repeat `GLOB_DOOFFS` on this call. Otherwise honor `GLOB_DOOFFS` for a fresh expansion.
    let offs: usize = if append || (flags & GLOB_DOOFFS) != 0 {
        (*pglob).gl_offs
    } else {
        0
    };
    let new_count: usize = paths.len();

    // Pointer slots needed: reserved offset + carried-over entries + new entries + null terminator.
    let slots: usize = match offs
        .checked_add(old_count)
        .and_then(|value| value.checked_add(new_count))
        .and_then(|value| value.checked_add(1))
    {
        Some(slots) => slots,
        None => return GLOB_NOSPACE,
    };
    let array: *mut *mut c_char = match slots.checked_mul(size_of::<*mut c_char>()) {
        Some(bytes) => malloc(bytes as c_size_t).cast::<*mut c_char>(),
        None => return GLOB_NOSPACE,
    };
    if array.is_null() {
        return GLOB_NOSPACE;
    }

    // The reserved leading slots are null pointers owned by the caller.
    ptr::write_bytes(array, 0, offs);

    // Carry over the pathnames from a previous call; their storage is reused, not copied.
    if append {
        let old: *mut *mut c_char = (*pglob).gl_pathv;
        for i in 0..old_count {
            *array.add(offs + i) = *old.add(offs + i);
        }
    }

    // Duplicate the new pathnames into `malloc()`ed storage.
    for (i, path) in paths.iter().enumerate() {
        let copy: *mut c_char = dup_to_malloc(path);
        if copy.is_null() {
            // Roll back the duplicates made so far before reporting the failure.
            for j in 0..i {
                free((*array.add(offs + old_count + j)).cast::<c_void>());
            }
            free(array.cast::<c_void>());
            return GLOB_NOSPACE;
        }
        *array.add(offs + old_count + i) = copy;
    }
    *array.add(offs + old_count + new_count) = ptr::null_mut();

    // Release the previous pointer array; its strings were carried over above.
    if append {
        free((*pglob).gl_pathv.cast::<c_void>());
    }

    (*pglob).gl_pathv = array;
    (*pglob).gl_pathc = old_count + new_count;
    (*pglob).gl_offs = offs;
    0
}

impl GlobCtx {
    /// `fnmatch()` flags used to match a single pattern component against a directory entry.
    fn fnmatch_flags(&self) -> c_int {
        let mut flags: c_int = FNM_PERIOD;
        if self.noescape {
            flags |= FNM_NOESCAPE;
        }
        flags
    }

    /// Reacts to a failure to open or read directory `path` reported with error number `err`.
    ///
    /// Invokes the caller's `errfunc` (if any) and honors `GLOB_ERR`, returning
    /// [`GlobError::Aborted`] when the scan must stop.
    ///
    /// # Safety
    ///
    /// Invokes the caller-supplied `errfunc` function pointer.
    unsafe fn handle_error(&self, path: &[u8], err: c_int) -> Result<(), GlobError> {
        if let Some(func) = self.errfunc {
            if let Ok(cpath) = CString::new(path) {
                if func(cpath.as_ptr(), err) != 0 {
                    return Err(GlobError::Aborted);
                }
            }
        }
        if (self.flags & GLOB_ERR) != 0 {
            return Err(GlobError::Aborted);
        }
        Ok(())
    }

    /// Records `path` as a match, applying the `GLOB_MARK` and directory-only rules.
    ///
    /// `known_exists` is `true` when `path` came from a directory entry (so it certainly exists) and
    /// `false` for a literal final component, whose existence must be confirmed with `stat()`.
    ///
    /// # Safety
    ///
    /// `stat()`s `path` when its file type must be determined.
    unsafe fn emit(&mut self, path: &[u8], known_exists: bool) {
        let want_mark: bool = self.dir_only || (self.flags & GLOB_MARK) != 0;
        let mut is_dir: bool = false;
        if want_mark || !known_exists {
            let Ok(cpath) = CString::new(path) else {
                return;
            };
            let mut st: stat_t = stat_t::default();
            if stat(cpath.as_ptr(), &mut st) == 0 {
                is_dir = S_ISDIR(st.st_mode);
            } else if !known_exists {
                // A literal component that does not exist contributes no match.
                return;
            }
        }
        // A directory-only pattern (trailing '/') discards non-directory matches.
        if self.dir_only && !is_dir {
            return;
        }
        self.matches.push((path.to_vec(), want_mark && is_dir));
    }

    /// Recursively expands pattern component `idx` against directory `prefix`.
    ///
    /// # Safety
    ///
    /// Performs directory and file-system operations through libc; `prefix` and the components in
    /// `comps` must be valid byte strings free of interior null bytes.
    unsafe fn walk(&mut self, comps: &[&[u8]], prefix: &[u8], idx: usize) -> Result<(), GlobError> {
        let comp: &[u8] = comps[idx];
        let is_last: bool = idx + 1 == comps.len();

        // A component without wildcards is taken literally: descend without a directory scan, and
        // confirm existence only at the final component.
        if !has_magic(comp, self.noescape) {
            let literal: Vec<u8> = unescape(comp, self.noescape);
            let next: Vec<u8> = join(prefix, &literal);
            if is_last {
                self.emit(&next, false);
                return Ok(());
            }
            return self.walk(comps, &next, idx + 1);
        }

        // A wildcard component is matched against the entries of `prefix` (or the current working
        // directory when `prefix` is empty).
        let dir: &[u8] = if prefix.is_empty() { &b"."[..] } else { prefix };
        let Ok(cdir) = CString::new(dir) else {
            return Ok(());
        };
        let dirp: *mut c_void = opendir(cdir.as_ptr());
        if dirp.is_null() {
            let err: c_int = *__errno_location();
            // A missing path or a non-directory simply yields no matches and is never fatal, which
            // mirrors the empty-list result POSIX describes for a pattern such as "non-existing/*".
            if err != ENOENT && err != ENOTDIR {
                self.handle_error(dir, err)?;
            }
            return Ok(());
        }

        let Ok(cpattern) = CString::new(comp) else {
            closedir(dirp);
            return Ok(());
        };
        let fnm_flags: c_int = self.fnmatch_flags();
        let mut outcome: Result<(), GlobError> = Ok(());

        loop {
            // `readdir()` leaves `errno` unchanged at end-of-directory but sets it on error, so it
            // is cleared first to tell the two apart.
            *__errno_location() = 0;
            let entry: *mut dirent = readdir(dirp);
            if entry.is_null() {
                let err: c_int = *__errno_location();
                if err != 0 {
                    outcome = self.handle_error(dir, err);
                }
                break;
            }

            let name_ptr: *const c_char = (*entry).d_name.as_ptr().cast::<c_char>();
            let name: &[u8] = CStr::from_ptr(name_ptr).to_bytes();

            // The "." and ".." entries are never produced by wildcard expansion.
            if name == b"." || name == b".." {
                continue;
            }

            if fnmatch(cpattern.as_ptr(), name_ptr, fnm_flags) == 0 {
                let next: Vec<u8> = join(prefix, name);
                let step: Result<(), GlobError> = if is_last {
                    self.emit(&next, true);
                    Ok(())
                } else {
                    self.walk(comps, &next, idx + 1)
                };
                if let Err(err) = step {
                    outcome = Err(err);
                    break;
                }
            }
        }

        closedir(dirp);
        outcome
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Expands the shell wildcard pattern `pattern` into the list of existing pathnames that it matches,
/// following the POSIX pathname pattern-matching rules. The matched pathnames are returned through
/// `pglob` and, unless `GLOB_NOSORT` is set, are sorted in byte (C locale) order.
///
/// The pattern is matched component by component (split on `'/'`): a component containing `'*'`,
/// `'?'`, or a `[...]` bracket expression is expanded against the entries of the corresponding
/// directory using [`fnmatch()`], while a literal component is looked up directly. A leading `'.'`
/// in a filename is matched only by an explicit `'.'` in the pattern, and the `"."`/`".."` entries
/// are never produced by a wildcard.
///
/// # Parameters
///
/// - `pattern`: Null-terminated pathname pattern to expand.
/// - `flags`: Bitwise-or of `GLOB_ERR`, `GLOB_MARK`, `GLOB_NOSORT`, `GLOB_DOOFFS`, `GLOB_NOCHECK`,
///   `GLOB_APPEND`, and `GLOB_NOESCAPE`.
/// - `errfunc`: Optional callback invoked as `errfunc(path, errno)` when a directory cannot be
///   opened or read; a non-zero return aborts the scan.
/// - `pglob`: Pointer to a caller-provided `glob_t` that receives the results.
///
/// # Returns
///
/// `0` on success. Otherwise one of `GLOB_NOSPACE` (allocation failed), `GLOB_ABORTED` (the scan was
/// stopped by an error), or `GLOB_NOMATCH` (no pathname matched and `GLOB_NOCHECK` was not set). On
/// every return `pglob->gl_pathc` and `pglob->gl_pathv` reflect the pathnames scanned so far.
///
/// # Safety
///
/// `pattern` must be null or a valid null-terminated C string, and `pglob` must be null or a valid,
/// writable pointer to a `glob_t`. Under `GLOB_APPEND` the `glob_t` must come from a previous
/// [`glob()`] call; under `GLOB_DOOFFS` its `gl_offs` must be initialized. `errfunc`, when not null,
/// must be a valid function pointer.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/glob.html>
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn glob(
    pattern: *const c_char,
    flags: c_int,
    errfunc: Option<extern "C" fn(epath: *const c_char, eerrno: c_int) -> c_int>,
    pglob: *mut c_void,
) -> c_int {
    // A null result structure cannot be populated.
    if pglob.is_null() {
        return GLOB_NOMATCH;
    }
    let pglob: *mut GlobT = pglob.cast::<GlobT>();

    // A null pattern matches nothing; publish an empty but valid result.
    if pattern.is_null() {
        let code: c_int = finalize(pglob, flags, &[]);
        return if code != 0 { code } else { GLOB_NOMATCH };
    }

    let pattern_bytes: &[u8] = CStr::from_ptr(pattern).to_bytes();
    let absolute: bool = pattern_bytes.first() == Some(&b'/');
    let components: Vec<&[u8]> = pattern_bytes
        .split(|&byte| byte == b'/')
        .filter(|segment| !segment.is_empty())
        .collect();
    // A trailing '/' restricts matches to directories, except for a pattern of only slashes, which
    // denotes the root directory and is handled as a special case below.
    let dir_only: bool = pattern_bytes.last() == Some(&b'/') && !components.is_empty();

    let mut ctx: GlobCtx = GlobCtx {
        flags,
        errfunc,
        noescape: (flags & GLOB_NOESCAPE) != 0,
        dir_only,
        matches: Vec::new(),
    };

    let outcome: Result<(), GlobError> = if components.is_empty() {
        // The pattern is empty or consists solely of slashes.
        if absolute {
            // Any number of leading slashes denotes the root directory.
            ctx.emit(&b"/"[..], false);
        }
        Ok(())
    } else {
        let start: &[u8] = if absolute { &b"/"[..] } else { &b""[..] };
        ctx.walk(&components, start, 0)
    };

    // Sort the new matches by pathname (before any trailing mark) unless asked not to.
    if (flags & GLOB_NOSORT) == 0 {
        ctx.matches.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let mut code: c_int = 0;
    match outcome {
        Ok(()) => {
            if ctx.matches.is_empty() {
                if (flags & GLOB_NOCHECK) != 0 {
                    // Return the pattern itself, verbatim and unmarked.
                    ctx.matches.push((pattern_bytes.to_vec(), false));
                } else {
                    code = GLOB_NOMATCH;
                }
            }
        },
        Err(GlobError::Aborted) => code = GLOB_ABORTED,
    }

    // Apply the trailing-'/' mark now that the ordering is fixed, producing the final byte strings.
    let paths: Vec<Vec<u8>> = ctx
        .matches
        .into_iter()
        .map(|(mut path, mark)| {
            if mark && path.last() != Some(&b'/') {
                path.push(b'/');
            }
            path
        })
        .collect();

    let published: c_int = finalize(pglob, flags, &paths);
    if published != 0 {
        return published;
    }
    code
}

///
/// # Description
///
/// Frees the storage allocated by a previous successful [`glob()`] call and resets `pglob` to an
/// empty state. The reserved `gl_offs` leading slots are owned by the caller and are not freed.
///
/// # Parameters
///
/// - `pglob`: Pointer to the `glob_t` structure to release.
///
/// # Safety
///
/// `pglob` must be null or point to a `glob_t` previously populated by [`glob()`] and not already
/// freed. This function does not modify `errno`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn globfree(pglob: *mut c_void) {
    if pglob.is_null() {
        return;
    }
    let pglob: *mut GlobT = pglob.cast::<GlobT>();
    let pathv: *mut *mut c_char = (*pglob).gl_pathv;

    // Release the allocations only when there is a pathname array to free; an already-empty result
    // has nothing to release.
    if !pathv.is_null() {
        // The pathnames occupy `gl_pathc` slots that follow the `gl_offs` caller-owned reserved
        // slots; `glob()` always records the matching offset in `gl_offs`.
        let base: usize = (*pglob).gl_offs;
        for i in 0..(*pglob).gl_pathc {
            free((*pathv.add(base + i)).cast::<c_void>());
        }
        free(pathv.cast::<c_void>());
    }

    // Reset the result to an empty state unconditionally so the documented post-condition holds
    // regardless of whether any memory was allocated. The caller-owned `gl_offs` is preserved.
    (*pglob).gl_pathv = ptr::null_mut();
    (*pglob).gl_pathc = 0;
}
