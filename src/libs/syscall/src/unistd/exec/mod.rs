// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::{
        FileSystem,
        FileSystemPath,
        RegularFileOpenFlags,
    },
    sys::mman::{
        mmap,
        munmap,
        MemoryMapProtectionFlags,
    },
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm::__kcall_execv,
    mm::{
        Address,
        VirtualAddress,
    },
    pm::ExecvArgs,
};
use ::sysapi::{
    ffi::c_char,
    sys_mman::prot_flags,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Separator between directory entries in the `PATH` environment variable. POSIX uses a colon.
const PATH_SEPARATOR: char = ':';

/// Search path used by `execvp()` when `PATH` is absent from the environment. POSIX leaves this
/// value implementation-defined; a minimal, sensible default is used here.
const DEFAULT_PATH: &str = "/bin:/usr/bin";

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Maps the program image at `path` into the calling process's address space and reads its full
/// contents into the mapping.
///
/// The image is placed in an anonymous `mmap` region rather than a heap buffer so that the kernel
/// can stage it directly from mapped pages, and so that releasing it is a single `munmap`. The
/// returned region must be released with [`munmap`] if the subsequent `execv` kernel call fails.
///
/// # Parameters
///
/// - `path`: Path of the program image to read.
///
/// # Returns
///
/// Upon success, a tuple of the mapping's base address, the image length in bytes (the file
/// size), and the page-aligned length of the mapping (the value that must be passed to
/// [`munmap`] to release it) is returned. Otherwise, an error is returned and no mapping is left
/// behind.
///
fn map_executable(path: &str) -> Result<(VirtualAddress, usize, usize), Error> {
    let pathname: FileSystemPath = FileSystemPath::new(path)?;
    let file = FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;

    // Determine the file size up front so the mapping can be sized once.
    let size: usize = file.attributes()?.size().try_into().map_err(|_| {
        Error::new(ErrorCode::InvalidExecutableFormat, "executable size is invalid")
    })?;

    // An empty file cannot be a valid executable.
    if size == 0 {
        return Err(Error::new(ErrorCode::InvalidExecutableFormat, "executable is empty"));
    }

    // `mmap` rounds the request up to a whole number of pages; compute that same page-aligned
    // length here so it can be handed back for the eventual `munmap`, which only accepts a
    // page-aligned length that matches the mapped region's capacity.
    let map_len: usize =
        ::sys::mm::align_up(size, ::arch::mem::PAGE_ALIGNMENT).ok_or_else(|| {
            Error::new(ErrorCode::InvalidExecutableFormat, "executable size is invalid")
        })?;

    // Map a read-write, anonymous region to hold the image. There is no artificial size cap: the
    // image is bounded only by the mmap region and the available physical memory, and `mmap` below
    // fails if it cannot be satisfied.
    let prot: MemoryMapProtectionFlags =
        MemoryMapProtectionFlags::try_from(prot_flags::PROT_READ | prot_flags::PROT_WRITE)?;
    let base: VirtualAddress = mmap(size, prot)?;

    // Read the whole file into the mapping, accounting for short reads.
    // SAFETY: `mmap` returned a region of at least `size` bytes that this process exclusively owns.
    let buf: &mut [u8] =
        unsafe { core::slice::from_raw_parts_mut(base.into_raw_value() as *mut u8, size) };
    let mut total: usize = 0;
    while total < size {
        match file.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(error) => {
                let _ = munmap(base, map_len);
                return Err(error);
            },
        }
    }

    // A regular file must yield exactly its reported size; a short read indicates a truncated or
    // racing file and cannot be a valid executable.
    if total != size {
        let _ = munmap(base, map_len);
        return Err(Error::new(ErrorCode::InvalidExecutableFormat, "short read of executable"));
    }

    Ok((base, size, map_len))
}

///
/// # Description
///
/// Validates that a single argument or environment token can round-trip through the kernel's
/// NUL-separated `execv` wire format without being silently altered.
///
/// Tokens are flattened by joining them with a single NUL byte, and the new image's runtime
/// re-splits the result on NUL bytes. Every byte other than NUL is carried verbatim, so an argument
/// containing a space (or any other non-NUL byte) is delivered unchanged as one argument. A token is
/// therefore rejected only when it:
///
/// - is empty, because an empty token is reserved as the end-of-list sentinel in the wire format and
///   would otherwise be indistinguishable from it; or
/// - contains a NUL byte, because NUL is the token delimiter and an interior NUL would split the
///   token into two (and would also terminate the C string early in the new image).
///
/// # Parameters
///
/// - `token`: The argument or environment token to validate.
///
/// # Returns
///
/// `Ok(())` if the token is representable, otherwise an [`ErrorCode::InvalidArgument`] error.
///
fn validate_exec_token(token: &str) -> Result<(), Error> {
    if token.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "execv token must not be empty"));
    }

    if token.bytes().any(|byte| byte == 0) {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "execv token must not contain NUL bytes",
        ));
    }

    Ok(())
}

///
/// # Description
///
/// Reads the value of the `PATH` environment variable from the calling process's environment table
/// (the same table that backs `getenv`/`setenv`).
///
/// # Returns
///
/// The value of `PATH` as an owned string if it is set and is valid UTF-8, otherwise `None`.
///
fn read_env_path() -> Option<String> {
    let value: *const c_char = ::libc_stdlib::env_table::get("PATH");
    if value.is_null() {
        return None;
    }
    // SAFETY: `env_table::get` returned a non-null pointer (checked above) to a NUL-terminated C
    // string that stays valid until the next `set`/`unset` of `PATH`. No such mutation happens
    // before the value is copied into an owned `String` here, so the borrow is sound and the result
    // does not depend on the table's storage afterward.
    unsafe { ::core::ffi::CStr::from_ptr(value) }
        .to_str()
        .ok()
        .map(String::from)
}

///
/// # Description
///
/// Joins a single `PATH` directory prefix and a bare program name into one candidate path.
///
/// # Parameters
///
/// - `dir`: A single directory entry taken from `PATH`. Per POSIX, an empty entry denotes the
///   current working directory; in that case `file` is returned unprefixed so it resolves relative
///   to the current working directory.
/// - `file`: The bare program name (the caller guarantees it contains no slash).
///
/// # Returns
///
/// The constructed candidate path.
///
fn join_search_path(dir: &str, file: &str) -> String {
    if dir.is_empty() {
        return String::from(file);
    }

    let mut candidate: String = String::with_capacity(dir.len() + 1 + file.len());
    candidate.push_str(dir);
    if !dir.ends_with('/') {
        candidate.push('/');
    }
    candidate.push_str(file);
    candidate
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Replaces the image of the calling process with the program found at `path`, following POSIX
/// `execv()` semantics.
///
/// Because the kernel performs no filesystem I/O, this wrapper maps the target program's ELF image
/// into the calling process's address space (via `mmap`) and hands it, together with the argument
/// and environment vectors, to the [`__kcall_execv`] kernel call. The argument and environment
/// vectors are flattened into NUL-separated strings, which the new image's runtime re-splits on NUL
/// bytes. Because NUL is the only delimiter, every other byte — including spaces — is carried
/// verbatim, so an argument that contains a space arrives as a single argument. To keep that
/// round-trip lossless, every token is validated up front (see `validate_exec_token`): a token must
/// be non-empty and must not contain a NUL byte, otherwise [`ErrorCode::InvalidArgument`] is
/// returned before the executable is mapped.
///
/// # Parameters
///
/// - `path`: Path of the program image to execute.
/// - `argv`: Argument vector for the new image (including the program name as `argv[0]`).
/// - `envp`: Environment for the new image.
///
/// # Returns
///
/// On success this function does not return: the calling process's image is replaced and control
/// transfers to the new program. It returns only on failure, yielding the error that prevented the
/// replacement; the calling process is left intact in that case.
///
pub fn do_execv(path: &str, argv: &[&str], envp: &[&str]) -> Error {
    // POSIX requires `argv` to carry at least the program name in `argv[0]`; an empty vector
    // would otherwise flatten into an empty args string and leave the new image without a name.
    if argv.is_empty() {
        return Error::new(ErrorCode::InvalidArgument, "argv must contain at least argv[0]");
    }

    // Validate every argument and environment token before mapping the executable, so a token that
    // cannot survive the NUL-separated wire format fails fast: this avoids leaking a mapping on the
    // error path and prevents the new image from silently observing altered vectors.
    for &token in argv.iter().chain(envp.iter()) {
        if let Err(error) = validate_exec_token(token) {
            return error;
        }
    }

    // Map the target executable into this address space.
    let (elf_base, elf_len, map_len): (VirtualAddress, usize, usize) = match map_executable(path) {
        Ok(image) => image,
        Err(error) => return error,
    };

    // Flatten the argument and environment vectors into the kernel's NUL-separated, on-the-wire
    // form. The new image's runtime re-splits each string on NUL bytes, with every NUL acting as a
    // token delimiter, so a token must not contain an interior NUL (validated above); every other
    // byte — including spaces — is carried verbatim, so an argument containing a space is delivered
    // as a single argument. The new image stops at the first empty token (the zero-filled tail of
    // the kernel-installed page following the final token), so no trailing delimiter is required
    // here.
    let args: String = argv.join("\0");
    let env: String = envp.join("\0");

    // Describe the image and its arguments for the kernel. The buffers remain valid for the
    // duration of the kernel call, which reads from them before the image is replaced.
    let exec_args: ExecvArgs = ExecvArgs {
        elf_ptr: elf_base,
        elf_len,
        args_ptr: VirtualAddress::from_raw_value(args.as_ptr() as usize),
        args_len: args.len(),
        env_ptr: VirtualAddress::from_raw_value(env.as_ptr() as usize),
        env_len: env.len(),
    };

    // Issue the kernel call. On success it never returns and the new image gets fresh BSS; on
    // failure, the old image keeps running and must retain its descriptor-resolution cache.
    let error: Error = __kcall_execv(&exec_args);

    // The image was not replaced: release the mapping holding it before returning the error.
    let _ = munmap(elf_base, map_len);
    error
}

///
/// # Description
///
/// Collects a NUL-terminated array of C strings into a vector of string slices that borrow the
/// underlying C string storage.
///
/// # Parameters
///
/// - `array`: Pointer to a NUL-pointer-terminated array of C strings, or null for an empty vector.
///
/// # Returns
///
/// Upon success, the collected string slices are returned. If any element is not valid UTF-8, an
/// error is returned instead.
///
/// # Safety
///
/// The caller must ensure that `array`, when non-null, points to a NUL-pointer-terminated array of
/// valid, NUL-terminated C strings that remain valid for the lifetime `'a`.
///
unsafe fn collect_c_str_array<'a>(array: *const *const c_char) -> Result<Vec<&'a str>, Error> {
    let mut out: Vec<&'a str> = Vec::new();
    if array.is_null() {
        return Ok(out);
    }

    let mut index: isize = 0;
    loop {
        // SAFETY: the caller guarantees a NUL-pointer-terminated array.
        let entry: *const c_char = unsafe { *array.offset(index) };
        if entry.is_null() {
            break;
        }
        // SAFETY: the caller guarantees each entry is a valid, NUL-terminated C string.
        match unsafe { ::core::ffi::CStr::from_ptr(entry) }.to_str() {
            Ok(arg) => out.push(arg),
            Err(_) => {
                return Err(Error::new(ErrorCode::InvalidArgument, "argument is not valid UTF-8"))
            },
        }
        index += 1;
    }

    Ok(out)
}

///
/// # Description
///
/// Parses and validates the `path` and `argv` C-ABI inputs shared by every member of the `execv`
/// family, returning the path and the collected argument vector.
///
/// # Parameters
///
/// - `path`: NUL-terminated path of the program image to execute.
/// - `argv`: NUL-pointer-terminated array of argument C strings.
///
/// # Returns
///
/// Upon success, the parsed `path` and argument vector are returned. Otherwise, an error is
/// returned: [`ErrorCode::BadAddress`] if `path` or `argv` is null, or
/// [`ErrorCode::InvalidArgument`] if `path` or an argument is not valid UTF-8.
///
/// # Safety
///
/// The caller must ensure that `path` points to a valid, NUL-terminated C string and that `argv` is
/// non-null and points to a NUL-pointer-terminated array of valid, NUL-terminated C strings.
///
unsafe fn parse_path_and_argv<'a>(
    path: *const c_char,
    argv: *const *const c_char,
) -> Result<(&'a str, Vec<&'a str>), Error> {
    if path.is_null() {
        return Err(Error::new(ErrorCode::BadAddress, "execv path pointer is null"));
    }

    // The execv family requires a non-null `argv` carrying at least `argv[0]`; a null pointer is
    // a caller error rather than an empty argument vector.
    if argv.is_null() {
        return Err(Error::new(ErrorCode::BadAddress, "execv argv pointer is null"));
    }

    // SAFETY: the caller guarantees `path` is a valid, NUL-terminated C string.
    let path: &str = match unsafe { ::core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(path) => path,
        Err(_) => {
            return Err(Error::new(ErrorCode::InvalidArgument, "execv path is not valid UTF-8"))
        },
    };

    // SAFETY: the caller upholds the array invariants documented above.
    let argv_vec: Vec<&str> = unsafe { collect_c_str_array(argv) }?;

    Ok((path, argv_vec))
}

///
/// # Description
///
/// C-ABI adapter for the `execv` family: parses the `path`, `argv`, and optional `envp` C strings
/// and replaces the calling process's image via [`do_execv`].
///
/// # Parameters
///
/// - `path`: NUL-terminated path of the program image to execute.
/// - `argv`: NUL-pointer-terminated array of argument C strings.
/// - `envp`: NUL-pointer-terminated array of environment C strings, or null for an empty
///   environment.
///
/// # Returns
///
/// This function returns only on failure, yielding the error that prevented the replacement; on
/// success the process image is replaced and control does not return.
///
/// # Safety
///
/// The caller must ensure that `path` points to a valid, NUL-terminated C string and that `argv`
/// and `envp`, when non-null, point to NUL-pointer-terminated arrays of valid, NUL-terminated C
/// strings.
///
pub unsafe fn execv_from_c(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> Error {
    // SAFETY: the caller upholds the documented C-string and array invariants.
    let (path, argv_vec): (&str, Vec<&str>) = match unsafe { parse_path_and_argv(path, argv) } {
        Ok(parsed) => parsed,
        Err(error) => return error,
    };

    // SAFETY: the caller upholds the array invariants documented above.
    let envp_vec: Vec<&str> = match unsafe { collect_c_str_array(envp) } {
        Ok(envp_vec) => envp_vec,
        Err(error) => return error,
    };

    do_execv(path, &argv_vec, &envp_vec)
}

///
/// # Description
///
/// C-ABI adapter for the environment-inheriting `execv()`: parses the `path` and `argv` C strings,
/// snapshots the calling process's current environment, and replaces the calling process's image via
/// [`do_execv`].
///
/// Unlike [`execv_from_c`], which takes an explicit environment, this adapter inherits the caller's
/// environment to honor POSIX `execv()` semantics. The environment is read from the process-local
/// environment table that also backs `getenv`/`setenv`, and is flattened into the kernel's
/// NUL-separated `KEY=VALUE` form. (`execvp()` shares these semantics but performs a `PATH` search
/// first; see [`execvp_from_c`].)
///
/// # Parameters
///
/// - `path`: NUL-terminated path of the program image to execute.
/// - `argv`: NUL-pointer-terminated array of argument C strings.
///
/// # Returns
///
/// This function returns only on failure, yielding the error that prevented the replacement; on
/// success the process image is replaced and control does not return.
///
/// # Safety
///
/// The caller must ensure that `path` points to a valid, NUL-terminated C string and that `argv` is
/// non-null and points to a NUL-pointer-terminated array of valid, NUL-terminated C strings.
///
pub unsafe fn execv_inherit_env_from_c(path: *const c_char, argv: *const *const c_char) -> Error {
    // SAFETY: the caller upholds the documented C-string and array invariants.
    let (path, argv_vec): (&str, Vec<&str>) = match unsafe { parse_path_and_argv(path, argv) } {
        Ok(parsed) => parsed,
        Err(error) => return error,
    };

    // Snapshot the caller's environment so the new image inherits it (POSIX `execv`/`execvp`
    // semantics). The owned strings outlive the borrowed token slice handed to `do_execv`.
    let env_owned: Vec<String> = ::libc_stdlib::env_table::snapshot();
    let env_tokens: Vec<&str> = env_owned.iter().map(String::as_str).collect();

    do_execv(path, &argv_vec, &env_tokens)
}

///
/// # Description
///
/// C-ABI adapter for `execvp()`: parses the `file` and `argv` C strings, locates the executable
/// following POSIX `execvp()` rules, and replaces the calling process's image via [`do_execv`],
/// inheriting the caller's environment.
///
/// If `file` contains a slash it is used directly as a path, with no search. Otherwise the
/// directories listed in the `PATH` environment variable are searched in order for an executable
/// named `file`, and the first candidate that can be executed replaces the image. When `PATH` is
/// unset, the default path [`DEFAULT_PATH`] is searched instead. An empty `PATH` entry denotes the
/// current working directory.
///
/// During the search a candidate that does not exist ([`ErrorCode::NoSuchEntry`]) or whose prefix is
/// not a directory ([`ErrorCode::InvalidDirectory`]) is skipped. A candidate that exists but cannot
/// be accessed ([`ErrorCode::PermissionDenied`]) is remembered and reported only if no later
/// candidate succeeds, mirroring the POSIX requirement to surface `EACCES`. Any other failure is
/// reported immediately. If the search exhausts every entry, [`ErrorCode::NoSuchEntry`] is returned.
///
/// # Parameters
///
/// - `file`: NUL-terminated name or path of the program image to execute.
/// - `argv`: NUL-pointer-terminated array of argument C strings.
///
/// # Returns
///
/// This function returns only on failure, yielding the error that prevented the replacement; on
/// success the process image is replaced and control does not return.
///
/// # Safety
///
/// The caller must ensure that `file` points to a valid, NUL-terminated C string and that `argv` is
/// non-null and points to a NUL-pointer-terminated array of valid, NUL-terminated C strings.
///
pub unsafe fn execvp_from_c(file: *const c_char, argv: *const *const c_char) -> Error {
    // SAFETY: the caller upholds the documented C-string and array invariants.
    let (file, argv_vec): (&str, Vec<&str>) = match unsafe { parse_path_and_argv(file, argv) } {
        Ok(parsed) => parsed,
        Err(error) => return error,
    };

    // An empty file name can never name an executable; fail fast before any search.
    if file.is_empty() {
        return Error::new(ErrorCode::NoSuchEntry, "execvp file name is empty");
    }

    // Snapshot the caller's environment so the new image inherits it (POSIX `execvp()` semantics).
    // The owned strings outlive the borrowed token slice handed to `do_execv`.
    let env_owned: Vec<String> = ::libc_stdlib::env_table::snapshot();
    let env_tokens: Vec<&str> = env_owned.iter().map(String::as_str).collect();

    // A file name that contains a slash is used as a path directly, with no `PATH` search (POSIX).
    if file.contains('/') {
        return do_execv(file, &argv_vec, &env_tokens);
    }

    // Otherwise, search each directory listed in `PATH` (or the default path when `PATH` is unset).
    let path_value: Option<String> = read_env_path();
    let search_path: &str = path_value.as_deref().unwrap_or(DEFAULT_PATH);

    // Remember a `PermissionDenied` failure so it can be reported if nothing else executes, matching
    // the POSIX requirement to surface `EACCES` when a candidate was found but could not be run.
    let mut deferred: Option<Error> = None;
    for dir in search_path.split(PATH_SEPARATOR) {
        let candidate: String = join_search_path(dir, file);

        // `do_execv` returns only on failure; on success the image is replaced and this never
        // returns.
        let error: Error = do_execv(&candidate, &argv_vec, &env_tokens);
        match error.code {
            // Not present in this directory: keep searching.
            ErrorCode::NoSuchEntry | ErrorCode::InvalidDirectory => continue,
            // Found but not accessible: remember it and keep searching.
            ErrorCode::PermissionDenied => deferred = Some(error),
            // Any other failure (e.g. a malformed executable) is reported immediately.
            _ => return error,
        }
    }

    // No candidate could be executed: report the remembered access error, if any, otherwise that
    // the file was not found anywhere on the search path.
    deferred.unwrap_or_else(|| Error::new(ErrorCode::NoSuchEntry, "execvp: file not found in PATH"))
}

//==================================================================================================
// Exec Startup Barrier
//==================================================================================================

///
/// # Description
///
/// Synchronizes a freshly started image with the process daemons before it runs `main`, applying
/// the close-on-exec half of POSIX `execv()` semantics.
///
/// A successful `execv()` never returns: the kernel replaces the image in place and transfers
/// control to the new image's `crt0`, which calls this. Two facts make a barrier necessary here.
/// First, the per-process descriptor table lives in `vfsd` and survives the image replacement, but
/// `vfsd` is not told the `exec` happened, so its `FD_CLOEXEC` descriptors are still present.
/// Second, this image's resolution cache was wiped along with BSS, so it must be rebuilt against the
/// post-close-on-exec table — never the pre-`exec` one.
///
/// This requests that the process manager daemon hold the calling process until `vfsd` has dropped
/// its `FD_CLOEXEC` descriptors, then proceeds. Because the daemon releases the process only after
/// that close-on-exec has been applied, the empty cache is afterwards rebuilt lazily on first use
/// (each miss resolves authoritatively against `vfsd`) and a descriptor flagged `FD_CLOEXEC` is
/// provably gone before this image can observe it.
///
/// The barrier is best-effort: the image has already been replaced and an `exec` cannot be undone,
/// so any failure to reach the daemons is logged and tolerated rather than blocking `main` forever.
///
/// This runs at the start of every standalone image — both a genuine `exec` and a fresh process
/// start. For a fresh start there are no `FD_CLOEXEC` descriptors to drop, so the round-trip is a
/// harmless no-op against an empty or console-only table. The process manager daemon itself (and
/// any minimal deployment whose root image runs as pid `PROCD`) has no distinct peer to query, so
/// it skips the barrier entirely rather than address the request to itself.
///
#[cfg(feature = "standalone")]
pub fn exec_startup_barrier() {
    use ::proc::{
        exec_request,
        ExecAckMessage,
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
    };
    use ::sys::{
        ipc::{
            Message,
            MessageType,
            SystemMessage,
            SystemMessageHeader,
        },
        pm::ProcessIdentifier,
    };

    // Identify ourselves so the barrier names the right subject and the process manager daemon can
    // release us by pid.
    let pid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(error) => {
            ::syslog::warn!("exec_startup_barrier(): failed to get pid: {error:?}");
            return;
        },
    };

    // If this image is itself the process manager daemon, there is no distinct peer to synchronize
    // with: the request below is addressed to `PROCD`, which is our own pid, so it would loop
    // straight back to us instead of eliciting an independent acknowledgement — and a process
    // parked in this barrier cannot service its own mailbox to produce one, so the wait could never
    // complete. This also covers minimal deployments where the root image runs as pid `PROCD` with
    // no separate daemon. In either case there is no foreign descriptor table to reconcile, so the
    // barrier is a no-op.
    if pid == ProcessIdentifier::PROCD {
        return;
    }

    // Announce the image start to the process manager daemon and ask to be held until the
    // filesystem daemon has applied close-on-exec to our inherited descriptor table.
    let request: Message = match exec_request(pid, ProcessIdentifier::PROCD, pid) {
        Ok(request) => request,
        Err(error) => {
            ::syslog::warn!("exec_startup_barrier(): failed to build exec request: {error:?}");
            return;
        },
    };
    if let Err(error) = ::sys::kcall::ipc::__kcall_send(&request) {
        ::syslog::warn!("exec_startup_barrier(): failed to send exec request: {error:?}");
        return;
    }

    // Wait for the process manager daemon to release us. `execv` succeeds only when the kernel can
    // replace the image, and procd answers an exec request exactly once — with success once vfsd
    // has applied close-on-exec, or with a failure status if the relay could not be dispatched — so
    // exactly one acknowledgement is expected, and it is the only message in flight before `main`.
    //
    // The wait is therefore a single receive: validate the reply and proceed. The image has already
    // been replaced and the exec cannot be undone, so a malformed, misdelivered, or failing reply
    // is logged and tolerated rather than retried. A retry loop would be unsafe here, not just
    // unhelpful: crt0 has no non-blocking or timed receive primitive, so a second receive after an
    // unexpected first message would block with nothing left to deliver. The self-addressed case
    // (this image is `PROCD`) is already excluded above, so a separate procd is guaranteed to be
    // the sender; a genuinely silent procd is a daemon-crash scenario beyond what the barrier can
    // recover, exactly as for the fork barrier this mirrors.
    let message: Message = match ::sys::kcall::ipc::__kcall_recv() {
        Ok(message) => message,
        Err(error) => {
            ::syslog::warn!("exec_startup_barrier(): failed to receive exec ack: {error:?}");
            return;
        },
    };

    // The kernel stamps the authoritative originating process into `message.source.pid`.
    let source: ProcessIdentifier = { message.source }.pid;
    if source != ProcessIdentifier::PROCD {
        ::syslog::warn!(
            "exec_startup_barrier(): unexpected message source while awaiting exec ack \
             (source={source:?})"
        );
        return;
    }
    if !matches!(message.message_type, MessageType::Ipc) {
        ::syslog::warn!(
            "exec_startup_barrier(): unexpected message type while awaiting exec ack ({:?})",
            message.message_type
        );
        return;
    }
    let system_message: SystemMessage = match SystemMessage::from_bytes(message.payload) {
        Ok(system_message) => system_message,
        Err(error) => {
            ::syslog::warn!("exec_startup_barrier(): malformed system message: {error:?}");
            return;
        },
    };
    if !matches!(system_message.header, SystemMessageHeader::ProcessManagement) {
        ::syslog::warn!(
            "exec_startup_barrier(): unexpected system message while awaiting exec ack ({:?})",
            system_message.header
        );
        return;
    }
    let pm_message: ProcessManagementMessage =
        match ProcessManagementMessage::from_bytes(system_message.payload) {
            Ok(pm_message) => pm_message,
            Err(error) => {
                ::syslog::warn!("exec_startup_barrier(): malformed process message: {error:?}");
                return;
            },
        };
    match pm_message.header {
        ProcessManagementMessageHeader::ExecAck => {
            let ack: ExecAckMessage = ExecAckMessage::from_bytes(pm_message.payload);
            let ack_pid: ProcessIdentifier = ack.pid;
            if ack_pid != pid {
                ::syslog::warn!(
                    "exec_startup_barrier(): exec ack named another process (expected={pid:?}, \
                     got={:?})",
                    ack_pid
                );
                return;
            }
            let status: i32 = ack.status;
            if status != ExecAckMessage::STATUS_SUCCESS {
                ::syslog::warn!(
                    "exec_startup_barrier(): barrier reported failure (status={status})"
                );
            }
        },
        header => {
            ::syslog::warn!(
                "exec_startup_barrier(): unexpected process message while awaiting exec ack \
                 ({header:?})"
            );
        },
    }
}

///
/// # Description
///
/// In run modes without a guest `vfsd`/`procd`, descriptors are interpreted directly by the host
/// and there is no flat descriptor table to synchronize with, so the exec startup barrier is a
/// no-op.
///
#[cfg(not(feature = "standalone"))]
pub fn exec_startup_barrier() {}
