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
/// space-separated `execv` wire format without being silently altered.
///
/// Tokens are flattened by joining them with single spaces, and the new image's runtime re-splits
/// the result on spaces. A token is therefore rejected when it:
///
/// - is empty, because the kernel trims surrounding whitespace and adjacent delimiters collapse, so
///   an empty token would be dropped;
/// - contains a space, because it would be split into two or more tokens; or
/// - contains a NUL byte, because it would terminate the C string early and is rejected by the
///   kernel.
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

    if token.bytes().any(|byte| byte == b' ' || byte == 0) {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "execv token must not contain spaces or NUL bytes",
        ));
    }

    Ok(())
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
/// vectors are flattened into space-separated strings, which the new image's runtime re-splits. To
/// keep that round-trip lossless, every token is validated up front (see `validate_exec_token`): a
/// token must be non-empty and contain neither a space nor a NUL byte, otherwise
/// [`ErrorCode::InvalidArgument`] is returned before the executable is mapped.
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
    // cannot survive the space-separated wire format fails fast: this avoids leaking a mapping on
    // the error path and prevents the new image from silently observing altered vectors.
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

    // Flatten the argument and environment vectors into the kernel's space-separated, on-the-wire
    // form. The new image's runtime re-splits each string on spaces, with every space acting as a
    // token delimiter, so a token must not contain embedded spaces (an embedded space would split
    // it into two); because the kernel trims surrounding whitespace, leading and trailing empty
    // tokens are not preserved.
    let args: String = argv.join(" ");
    let env: String = envp.join(" ");

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

    // Issue the kernel call. On success it never returns; only a failure surfaces here.
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
/// C-ABI adapter for the environment-inheriting members of the `execv` family (`execv`, `execvp`):
/// parses the `path` and `argv` C strings, snapshots the calling process's current environment, and
/// replaces the calling process's image via [`do_execv`].
///
/// Unlike [`execv_from_c`], which takes an explicit environment, this adapter inherits the caller's
/// environment to honor POSIX `execv()`/`execvp()` semantics. The environment is read from the
/// process-local environment table that also backs `getenv`/`setenv`, and is flattened into the
/// kernel's space-separated `KEY=VALUE` form.
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
