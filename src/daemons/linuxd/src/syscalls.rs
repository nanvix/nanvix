// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Function Type Aliases
//==================================================================================================

/// Type alias for `times()` system call function.
pub type TimesFn<T> = unsafe fn(&T, *mut libc::tms) -> libc::clock_t;

/// Type alias for `chdir()` system call function.
pub type ChdirFn<T> = unsafe fn(&T, *const libc::c_char) -> libc::c_int;

/// Type alias for `close()` system call function.
pub type CloseFn<T> = unsafe fn(&T, libc::c_int) -> libc::c_int;

/// Type alias for `faccessat()` system call function.
pub type FaccessatFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_char, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `fdatasync()` system call function.
pub type FdatasyncFn<T> = unsafe fn(&T, libc::c_int) -> libc::c_int;

/// Type alias for `getuid()` system call function.
pub type GetuidFn<T> = unsafe fn(&T) -> libc::uid_t;

/// Type alias for `geteuid()` system call function.
pub type GeteuidFn<T> = unsafe fn(&T) -> libc::uid_t;

/// Type alias for `getgid()` system call function.
pub type GetgidFn<T> = unsafe fn(&T) -> libc::gid_t;

/// Type alias for `getegid()` system call function.
pub type GetegidFn<T> = unsafe fn(&T) -> libc::gid_t;

/// Type alias for `getcwd()` system call function.
pub type GetcwdFn<T> = unsafe fn(&T, *mut libc::c_char, libc::size_t) -> *mut libc::c_char;

/// Type alias for `fsync()` system call function.
pub type FsyncFn<T> = unsafe fn(&T, libc::c_int) -> libc::c_int;

/// Type alias for `lseek()` system call function.
pub type LseekFn<T> = unsafe fn(&T, libc::c_int, libc::off_t, libc::c_int) -> libc::off_t;

/// Type alias for `ftruncate()` system call function.
pub type FtruncateFn<T> = unsafe fn(&T, libc::c_int, libc::off_t) -> libc::c_int;

/// Type alias for `write()` system call function.
pub type WriteFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_void, libc::size_t) -> libc::ssize_t;

/// Type alias for `read()` system call function.
pub type ReadFn<T> = unsafe fn(&T, libc::c_int, *mut libc::c_void, libc::size_t) -> libc::ssize_t;

/// Type alias for `pwrite()` system call function.
pub type PwriteFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_void, libc::size_t, libc::off_t) -> libc::ssize_t;

/// Type alias for `pread()` system call function.
pub type PreadFn<T> =
    unsafe fn(&T, libc::c_int, *mut libc::c_void, libc::size_t, libc::off_t) -> libc::ssize_t;

/// Type alias for `linkat()` system call function.
pub type LinkatFn<T> = unsafe fn(
    &T,
    libc::c_int,
    *const libc::c_char,
    libc::c_int,
    *const libc::c_char,
    libc::c_int,
) -> libc::c_int;

/// Type alias for `fchdir()` system call function.
pub type FchdirFn<T> = unsafe fn(&T, libc::c_int) -> libc::c_int;

/// Type alias for `fchown()` system call function.
pub type FchownFn<T> = unsafe fn(&T, libc::c_int, libc::uid_t, libc::gid_t) -> libc::c_int;

/// Type alias for `pipe()` system call function.
pub type PipeFn<T> = unsafe fn(&T, *mut libc::c_int) -> libc::c_int;

/// Type alias for `getdents()` system call function.
pub type GetdentsFn<T> = unsafe fn(&T, libc::c_int, *mut u8, libc::size_t) -> libc::c_long;

/// Type alias for `socket()` system call function.
pub type SocketFn<T> = unsafe fn(&T, libc::c_int, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `socketpair()` system call function.
pub type SocketpairFn<T> =
    unsafe fn(&T, libc::c_int, libc::c_int, libc::c_int, *mut libc::c_int) -> libc::c_int;

/// Type alias for `bind()` system call function.
pub type BindFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::sockaddr, libc::socklen_t) -> libc::c_int;

/// Type alias for `connect()` system call function.
pub type ConnectFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::sockaddr, libc::socklen_t) -> libc::c_int;

/// Type alias for `listen()` system call function.
pub type ListenFn<T> = unsafe fn(&T, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `getpeername()` system call function.
pub type GetpeernameFn<T> =
    unsafe fn(&T, libc::c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int;

/// Type alias for `getsockname()` system call function.
pub type GetsocknameFn<T> =
    unsafe fn(&T, libc::c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int;

/// Type alias for `accept()` system call function.
pub type AcceptFn<T> =
    unsafe fn(&T, libc::c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int;

/// Type alias for `recv()` system call function.
pub type RecvFn<T> =
    unsafe fn(&T, libc::c_int, *mut libc::c_void, libc::size_t, libc::c_int) -> libc::ssize_t;

/// Type alias for `send()` system call function.
pub type SendFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_void, libc::size_t, libc::c_int) -> libc::ssize_t;

/// Type alias for `shutdown()` system call function.
pub type ShutdownFn<T> = unsafe fn(&T, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `select()` system call function.
pub type SelectFn<T> = unsafe fn(
    &T,
    libc::c_int,
    *mut libc::fd_set,
    *mut libc::fd_set,
    *mut libc::fd_set,
    *mut libc::timeval,
) -> libc::c_int;

/// Type alias for `poll()` system call function.
pub type PollFn<T> = unsafe fn(&T, *mut libc::pollfd, libc::nfds_t, libc::c_int) -> libc::c_int;

//==================================================================================================
// Default Implementations - unistd.rs
//==================================================================================================

///
/// # Description
///
/// Default implementation for `chdir()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `path`: Path to change to.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_chdir<T>(_state: &T, path: *const libc::c_char) -> libc::c_int {
    libc::chdir(path)
}

///
/// # Description
///
/// Default implementation for `close()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor to close.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_close<T>(_state: &T, fd: libc::c_int) -> libc::c_int {
    libc::close(fd)
}

///
/// # Description
///
/// Default implementation for `faccessat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to check.
/// - `mode`: Access mode.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_faccessat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    mode: libc::c_int,
    flags: libc::c_int,
) -> libc::c_int {
    libc::faccessat(dirfd, pathname, mode, flags)
}

///
/// # Description
///
/// Default implementation for `fdatasync()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor to synchronize.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fdatasync<T>(_state: &T, fd: libc::c_int) -> libc::c_int {
    libc::fdatasync(fd)
}

///
/// # Description
///
/// Default implementation for `getuid()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
///
/// # Returns
///
/// The real user ID of the calling process.
///
pub unsafe fn default_getuid<T>(_state: &T) -> libc::uid_t {
    libc::getuid()
}

///
/// # Description
///
/// Default implementation for `geteuid()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
///
/// # Returns
///
/// The effective user ID of the calling process.
///
pub unsafe fn default_geteuid<T>(_state: &T) -> libc::uid_t {
    libc::geteuid()
}

///
/// # Description
///
/// Default implementation for `getgid()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
///
/// # Returns
///
/// The real group ID of the calling process.
///
pub unsafe fn default_getgid<T>(_state: &T) -> libc::gid_t {
    libc::getgid()
}

///
/// # Description
///
/// Default implementation for `getegid()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
///
/// # Returns
///
/// The effective group ID of the calling process.
///
pub unsafe fn default_getegid<T>(_state: &T) -> libc::gid_t {
    libc::getegid()
}

///
/// # Description
///
/// Default implementation for `getcwd()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `buf`: Buffer to store current working directory.
/// - `size`: Size of buffer.
///
/// # Returns
///
/// Upon successful completion, a pointer to the buffer is returned. Otherwise, NULL is returned and `errno` is set.
///
pub unsafe fn default_getcwd<T>(
    _state: &T,
    buf: *mut libc::c_char,
    size: libc::size_t,
) -> *mut libc::c_char {
    libc::getcwd(buf, size)
}

///
/// # Description
///
/// Default implementation for `fsync()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor to synchronize.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fsync<T>(_state: &T, fd: libc::c_int) -> libc::c_int {
    libc::fsync(fd)
}

///
/// # Description
///
/// Default implementation for `lseek()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `offset`: Offset.
/// - `whence`: Whence.
///
/// # Returns
///
/// Upon successful completion, the resulting offset is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_lseek<T>(
    _state: &T,
    fd: libc::c_int,
    offset: libc::off_t,
    whence: libc::c_int,
) -> libc::off_t {
    libc::lseek(fd, offset, whence)
}

///
/// # Description
///
/// Default implementation for `ftruncate()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `length`: Length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_ftruncate<T>(
    _state: &T,
    fd: libc::c_int,
    length: libc::off_t,
) -> libc::c_int {
    libc::ftruncate(fd, length)
}

///
/// # Description
///
/// Default implementation for `write()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `buf`: Buffer to write from.
/// - `count`: Number of bytes to write.
///
/// # Returns
///
/// Upon successful completion, the number of bytes written is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_write<T>(
    _state: &T,
    fd: libc::c_int,
    buf: *const libc::c_void,
    count: libc::size_t,
) -> libc::ssize_t {
    libc::write(fd, buf, count)
}

///
/// # Description
///
/// Default implementation for `read()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `buf`: Buffer to read into.
/// - `count`: Number of bytes to read.
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_read<T>(
    _state: &T,
    fd: libc::c_int,
    buf: *mut libc::c_void,
    count: libc::size_t,
) -> libc::ssize_t {
    libc::read(fd, buf, count)
}

///
/// # Description
///
/// Default implementation for `pwrite()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `buf`: Buffer to write from.
/// - `count`: Number of bytes to write.
/// - `offset`: Offset.
///
/// # Returns
///
/// Upon successful completion, the number of bytes written is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_pwrite<T>(
    _state: &T,
    fd: libc::c_int,
    buf: *const libc::c_void,
    count: libc::size_t,
    offset: libc::off_t,
) -> libc::ssize_t {
    libc::pwrite(fd, buf, count, offset)
}

///
/// # Description
///
/// Default implementation for `pread()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `buf`: Buffer to read into.
/// - `count`: Number of bytes to read.
/// - `offset`: Offset.
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_pread<T>(
    _state: &T,
    fd: libc::c_int,
    buf: *mut libc::c_void,
    count: libc::size_t,
    offset: libc::off_t,
) -> libc::ssize_t {
    libc::pread(fd, buf, count, offset)
}

///
/// # Description
///
/// Default implementation for `linkat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `olddirfd`: Old directory file descriptor.
/// - `oldpath`: Old path.
/// - `newdirfd`: New directory file descriptor.
/// - `newpath`: New path.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_linkat<T>(
    _state: &T,
    olddirfd: libc::c_int,
    oldpath: *const libc::c_char,
    newdirfd: libc::c_int,
    newpath: *const libc::c_char,
    flags: libc::c_int,
) -> libc::c_int {
    libc::linkat(olddirfd, oldpath, newdirfd, newpath, flags)
}

///
/// # Description
///
/// Default implementation for `fchdir()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchdir<T>(_state: &T, fd: libc::c_int) -> libc::c_int {
    libc::fchdir(fd)
}

///
/// # Description
///
/// Default implementation for `fchown()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `owner`: Owner.
/// - `group`: Group.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchown<T>(
    _state: &T,
    fd: libc::c_int,
    owner: libc::uid_t,
    group: libc::gid_t,
) -> libc::c_int {
    libc::fchown(fd, owner, group)
}

///
/// # Description
///
/// Default implementation for `pipe()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `pipefd`: Pipe file descriptors.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_pipe<T>(_state: &T, pipefd: *mut libc::c_int) -> libc::c_int {
    libc::pipe(pipefd)
}

/// Type alias for `openat()` system call function.
pub type OpenatFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_char, libc::c_int, libc::mode_t) -> libc::c_int;

/// Type alias for `unlinkat()` system call function.
pub type UnlinkatFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_char, libc::c_int) -> libc::c_int;

/// Type alias for `renameat()` system call function.
pub type RenameatFn<T> = unsafe fn(
    &T,
    libc::c_int,
    *const libc::c_char,
    libc::c_int,
    *const libc::c_char,
) -> libc::c_int;

/// Type alias for `fstatat()` system call function.
pub type FstatatFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_char, *mut libc::stat, libc::c_int) -> libc::c_int;

/// Type alias for `posix_fallocate()` system call function.
pub type PosixFallocateFn<T> = unsafe fn(&T, libc::c_int, libc::off_t, libc::off_t) -> libc::c_int;

/// Type alias for `posix_fadvise()` system call function.
pub type PosixFadviseFn<T> =
    unsafe fn(&T, libc::c_int, libc::off_t, libc::off_t, libc::c_int) -> libc::c_int;

/// Type alias for `fstat()` system call function.
pub type FstatFn<T> = unsafe fn(&T, libc::c_int, *mut libc::stat) -> libc::c_int;

/// Type alias for `symlinkat()` system call function.
pub type SymlinkatFn<T> =
    unsafe fn(&T, *const libc::c_char, libc::c_int, *const libc::c_char) -> libc::c_int;

/// Type alias for `readlinkat()` system call function.
pub type ReadlinkatFn<T> = unsafe fn(
    &T,
    libc::c_int,
    *const libc::c_char,
    *mut libc::c_char,
    libc::size_t,
) -> libc::ssize_t;

/// Type alias for `mkdirat()` system call function.
pub type MkdiratFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_char, libc::mode_t) -> libc::c_int;

/// Type alias for `utimensat()` system call function.
pub type UtimensatFn<T> = unsafe fn(
    &T,
    libc::c_int,
    *const libc::c_char,
    *const libc::timespec,
    libc::c_int,
) -> libc::c_int;

/// Type alias for `futimens()` system call function.
pub type FutimensFn<T> = unsafe fn(&T, libc::c_int, *const libc::timespec) -> libc::c_int;

/// Type alias for `fcntl()` system call function.
pub type FcntlFn<T> = unsafe fn(&T, libc::c_int, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `fchownat()` system call function.
pub type FchownatFn<T> = unsafe fn(
    &T,
    libc::c_int,
    *const libc::c_char,
    libc::uid_t,
    libc::gid_t,
    libc::c_int,
) -> libc::c_int;

/// Type alias for `fchmod()` system call function.
pub type FchmodFn<T> = unsafe fn(&T, libc::c_int, libc::mode_t) -> libc::c_int;

/// Type alias for `fchmodat()` system call function.
pub type FchmodatFn<T> =
    unsafe fn(&T, libc::c_int, *const libc::c_char, libc::mode_t, libc::c_int) -> libc::c_int;

//==================================================================================================
// Default Implementations - fcntl.rs
//==================================================================================================

///
/// # Description
///
/// Default implementation for `openat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to open.
/// - `flags`: Flags.
/// - `mode`: Mode.
///
/// # Returns
///
/// Upon successful completion, a file descriptor is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_openat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> libc::c_int {
    libc::openat(dirfd, pathname, flags, mode)
}

///
/// # Description
///
/// Default implementation for `unlinkat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to unlink.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_unlinkat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    flags: libc::c_int,
) -> libc::c_int {
    libc::unlinkat(dirfd, pathname, flags)
}

///
/// # Description
///
/// Default implementation for `renameat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `olddirfd`: Old directory file descriptor.
/// - `oldpath`: Old path.
/// - `newdirfd`: New directory file descriptor.
/// - `newpath`: New path.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_renameat<T>(
    _state: &T,
    olddirfd: libc::c_int,
    oldpath: *const libc::c_char,
    newdirfd: libc::c_int,
    newpath: *const libc::c_char,
) -> libc::c_int {
    libc::renameat(olddirfd, oldpath, newdirfd, newpath)
}

///
/// # Description
///
/// Default implementation for `fstatat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to stat.
/// - `buf`: Buffer to store stat.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fstatat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    buf: *mut libc::stat,
    flags: libc::c_int,
) -> libc::c_int {
    libc::fstatat(dirfd, pathname, buf, flags)
}

///
/// # Description
///
/// Default implementation for `posix_fallocate()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `offset`: Offset.
/// - `len`: Length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, an error number is returned.
///
pub unsafe fn default_posix_fallocate<T>(
    _state: &T,
    fd: libc::c_int,
    offset: libc::off_t,
    len: libc::off_t,
) -> libc::c_int {
    libc::posix_fallocate(fd, offset, len)
}

///
/// # Description
///
/// Default implementation for `posix_fadvise()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `offset`: Offset.
/// - `len`: Length.
/// - `advice`: Advice.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, an error number is returned.
///
pub unsafe fn default_posix_fadvise<T>(
    _state: &T,
    fd: libc::c_int,
    offset: libc::off_t,
    len: libc::off_t,
    advice: libc::c_int,
) -> libc::c_int {
    libc::posix_fadvise(fd, offset, len, advice)
}

///
/// # Description
///
/// Default implementation for `fstat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `buf`: Buffer to store stat.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fstat<T>(_state: &T, fd: libc::c_int, buf: *mut libc::stat) -> libc::c_int {
    libc::fstat(fd, buf)
}

///
/// # Description
///
/// Default implementation for `symlinkat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `target`: Target.
/// - `newdirfd`: New directory file descriptor.
/// - `linkpath`: Link path.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_symlinkat<T>(
    _state: &T,
    target: *const libc::c_char,
    newdirfd: libc::c_int,
    linkpath: *const libc::c_char,
) -> libc::c_int {
    libc::symlinkat(target, newdirfd, linkpath)
}

///
/// # Description
///
/// Default implementation for `readlinkat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to read.
/// - `buf`: Buffer to store link.
/// - `bufsiz`: Buffer size.
///
/// # Returns
///
/// Upon successful completion, the number of bytes placed in the buffer is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_readlinkat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    buf: *mut libc::c_char,
    bufsiz: libc::size_t,
) -> libc::ssize_t {
    libc::readlinkat(dirfd, pathname, buf, bufsiz)
}

///
/// # Description
///
/// Default implementation for `mkdirat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to create.
/// - `mode`: Mode.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_mkdirat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    mode: libc::mode_t,
) -> libc::c_int {
    libc::mkdirat(dirfd, pathname, mode)
}

///
/// # Description
///
/// Default implementation for `utimensat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to update.
/// - `times`: Times.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_utimensat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    times: *const libc::timespec,
    flags: libc::c_int,
) -> libc::c_int {
    libc::utimensat(dirfd, pathname, times, flags)
}

///
/// # Description
///
/// Default implementation for `futimens()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `times`: Times.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_futimens<T>(
    _state: &T,
    fd: libc::c_int,
    times: *const libc::timespec,
) -> libc::c_int {
    libc::futimens(fd, times)
}

///
/// # Description
///
/// Default implementation for `fcntl()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `cmd`: Command.
/// - `arg`: Argument.
///
/// # Returns
///
/// Upon successful completion, a value depends on the command. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fcntl<T>(
    _state: &T,
    fd: libc::c_int,
    cmd: libc::c_int,
    arg: libc::c_int,
) -> libc::c_int {
    libc::fcntl(fd, cmd, arg)
}

///
/// # Description
///
/// Default implementation for `fchownat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to change ownership.
/// - `owner`: Owner.
/// - `group`: Group.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchownat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    owner: libc::uid_t,
    group: libc::gid_t,
    flags: libc::c_int,
) -> libc::c_int {
    libc::fchownat(dirfd, pathname, owner, group, flags)
}

///
/// # Description
///
/// Default implementation for `fchmod()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `mode`: Mode.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchmod<T>(_state: &T, fd: libc::c_int, mode: libc::mode_t) -> libc::c_int {
    libc::fchmod(fd, mode)
}

///
/// # Description
///
/// Default implementation for `fchmodat()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to change mode.
/// - `mode`: Mode.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchmodat<T>(
    _state: &T,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    mode: libc::mode_t,
    flags: libc::c_int,
) -> libc::c_int {
    libc::fchmodat(dirfd, pathname, mode, flags)
}

//==================================================================================================
// Default Implementations - dirent.rs
//==================================================================================================

///
/// # Description
///
/// Default implementation for `getdents()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fd`: File descriptor.
/// - `dirp`: Directory entries buffer.
/// - `count`: Buffer size.
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_getdents<T>(
    _state: &T,
    fd: libc::c_int,
    dirp: *mut u8,
    count: libc::size_t,
) -> libc::c_long {
    libc::syscall(libc::SYS_getdents, fd, dirp, count)
}

//==================================================================================================
// Default Implementations - socket.rs
//==================================================================================================

///
/// # Description
///
/// Default implementation for `socket()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `domain`: Domain.
/// - `type_`: Type.
/// - `protocol`: Protocol.
///
/// # Returns
///
/// Upon successful completion, a file descriptor is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_socket<T>(
    _state: &T,
    domain: libc::c_int,
    type_: libc::c_int,
    protocol: libc::c_int,
) -> libc::c_int {
    libc::socket(domain, type_, protocol)
}

///
/// # Description
///
/// Default implementation for `socketpair()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `domain`: Domain.
/// - `type_`: Type.
/// - `protocol`: Protocol.
/// - `sv`: Socket pair.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_socketpair<T>(
    _state: &T,
    domain: libc::c_int,
    type_: libc::c_int,
    protocol: libc::c_int,
    sv: *mut libc::c_int,
) -> libc::c_int {
    libc::socketpair(domain, type_, protocol, sv)
}

///
/// # Description
///
/// Default implementation for `bind()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_bind<T>(
    _state: &T,
    sockfd: libc::c_int,
    addr: *const libc::sockaddr,
    addrlen: libc::socklen_t,
) -> libc::c_int {
    libc::bind(sockfd, addr, addrlen)
}

///
/// # Description
///
/// Default implementation for `connect()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_connect<T>(
    _state: &T,
    sockfd: libc::c_int,
    addr: *const libc::sockaddr,
    addrlen: libc::socklen_t,
) -> libc::c_int {
    libc::connect(sockfd, addr, addrlen)
}

///
/// # Description
///
/// Default implementation for `listen()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `backlog`: Backlog.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_listen<T>(
    _state: &T,
    sockfd: libc::c_int,
    backlog: libc::c_int,
) -> libc::c_int {
    libc::listen(sockfd, backlog)
}

///
/// # Description
///
/// Default implementation for `getpeername()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_getpeername<T>(
    _state: &T,
    sockfd: libc::c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut libc::socklen_t,
) -> libc::c_int {
    libc::getpeername(sockfd, addr, addrlen)
}

///
/// # Description
///
/// Default implementation for `getsockname()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_getsockname<T>(
    _state: &T,
    sockfd: libc::c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut libc::socklen_t,
) -> libc::c_int {
    libc::getsockname(sockfd, addr, addrlen)
}

///
/// # Description
///
/// Default implementation for `accept()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, a file descriptor is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_accept<T>(
    _state: &T,
    sockfd: libc::c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut libc::socklen_t,
) -> libc::c_int {
    libc::accept(sockfd, addr, addrlen)
}

///
/// # Description
///
/// Default implementation for `recv()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `buf`: Buffer.
/// - `len`: Length.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, the number of bytes received is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_recv<T>(
    _state: &T,
    sockfd: libc::c_int,
    buf: *mut libc::c_void,
    len: libc::size_t,
    flags: libc::c_int,
) -> libc::ssize_t {
    libc::recv(sockfd, buf, len, flags)
}

///
/// # Description
///
/// Default implementation for `send()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `buf`: Buffer.
/// - `len`: Length.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, the number of bytes sent is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_send<T>(
    _state: &T,
    sockfd: libc::c_int,
    buf: *const libc::c_void,
    len: libc::size_t,
    flags: libc::c_int,
) -> libc::ssize_t {
    libc::send(sockfd, buf, len, flags)
}

///
/// # Description
///
/// Default implementation for `shutdown()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `sockfd`: Socket file descriptor.
/// - `how`: How.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_shutdown<T>(
    _state: &T,
    sockfd: libc::c_int,
    how: libc::c_int,
) -> libc::c_int {
    libc::shutdown(sockfd, how)
}

//==================================================================================================
// Default Implementations - poll.rs
//==================================================================================================

///
/// # Description
///
/// Default implementation for `poll()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `fds`: File descriptors.
/// - `nfds`: Number of file descriptors.
/// - `timeout`: Timeout.
///
/// # Returns
///
/// Upon successful completion, the number of file descriptors with events is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_poll<T>(
    _state: &T,
    fds: *mut libc::pollfd,
    nfds: libc::nfds_t,
    timeout: libc::c_int,
) -> libc::c_int {
    libc::poll(fds, nfds, timeout)
}

//==================================================================================================
// Default Implementations - sys_select.rs
//==================================================================================================

///
/// # Description
///
/// Default implementation for `select()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `nfds`: Number of file descriptors.
/// - `readfds`: Read file descriptors.
/// - `writefds`: Write file descriptors.
/// - `exceptfds`: Exception file descriptors.
/// - `timeout`: Timeout.
///
/// # Returns
///
/// Upon successful completion, the number of file descriptors with events is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_select<T>(
    _state: &T,
    nfds: libc::c_int,
    readfds: *mut libc::fd_set,
    writefds: *mut libc::fd_set,
    exceptfds: *mut libc::fd_set,
    timeout: *mut libc::timeval,
) -> libc::c_int {
    libc::select(nfds, readfds, writefds, exceptfds, timeout)
}

//==================================================================================================
// Default Implementations - times.rs
//==================================================================================================

///
/// # Description
///
/// Default implementation for `times()` system call.
///
/// # Parameters
///
/// - `_state`: Immutable reference to state (unused in default implementation).
/// - `buf`: Buffer to store times.
///
/// # Returns
///
/// Upon successful completion, the elapsed real time in clock ticks is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_times<T>(_state: &T, buf: *mut libc::tms) -> libc::clock_t {
    libc::times(buf)
}

///
/// # Description
///
/// Action to take on a system call.
pub enum SyscallAction<F> {
    /// Block system call.
    Block,
    /// Forward system call to underlying provider.
    Forward(F),
}

///
/// # Description
///
/// System call routing table.
///
/// This structure holds the configuration for how system calls should be handled. Each system
/// call can be either blocked or forwarded to a handler function. The generic type parameter `T`
/// represents custom state that is passed as a reference to each system call handler, allowing
/// implementations to maintain context-specific information (e.g., file descriptor tables,
/// process state, or other runtime data).
///
/// # Type Parameters
///
/// - `T`: Custom state type that is passed to all system call handler functions. Use `()` if no
///   state is required.
///
pub struct SyscallTable<T> {
    /// State that is passed to each system call action function.
    pub state: T,

    // unistd.rs system calls.
    pub chdir: SyscallAction<ChdirFn<T>>,
    pub close: SyscallAction<CloseFn<T>>,
    pub faccessat: SyscallAction<FaccessatFn<T>>,
    pub fdatasync: SyscallAction<FdatasyncFn<T>>,
    pub fchdir: SyscallAction<FchdirFn<T>>,
    pub fchown: SyscallAction<FchownFn<T>>,
    pub fsync: SyscallAction<FsyncFn<T>>,
    pub ftruncate: SyscallAction<FtruncateFn<T>>,
    pub getcwd: SyscallAction<GetcwdFn<T>>,
    pub getegid: SyscallAction<GetegidFn<T>>,
    pub geteuid: SyscallAction<GeteuidFn<T>>,
    pub getgid: SyscallAction<GetgidFn<T>>,
    pub getuid: SyscallAction<GetuidFn<T>>,
    pub linkat: SyscallAction<LinkatFn<T>>,
    pub lseek: SyscallAction<LseekFn<T>>,
    pub pipe: SyscallAction<PipeFn<T>>,
    pub pread: SyscallAction<PreadFn<T>>,
    pub pwrite: SyscallAction<PwriteFn<T>>,
    pub read: SyscallAction<ReadFn<T>>,
    pub write: SyscallAction<WriteFn<T>>,

    // fcntl.rs system calls.
    pub fchmod: SyscallAction<FchmodFn<T>>,
    pub fchmodat: SyscallAction<FchmodatFn<T>>,
    pub fchownat: SyscallAction<FchownatFn<T>>,
    pub fcntl: SyscallAction<FcntlFn<T>>,
    pub fstat: SyscallAction<FstatFn<T>>,
    pub fstatat: SyscallAction<FstatatFn<T>>,
    pub futimens: SyscallAction<FutimensFn<T>>,
    pub mkdirat: SyscallAction<MkdiratFn<T>>,
    pub openat: SyscallAction<OpenatFn<T>>,
    pub posix_fadvise: SyscallAction<PosixFadviseFn<T>>,
    pub posix_fallocate: SyscallAction<PosixFallocateFn<T>>,
    pub readlinkat: SyscallAction<ReadlinkatFn<T>>,
    pub renameat: SyscallAction<RenameatFn<T>>,
    pub symlinkat: SyscallAction<SymlinkatFn<T>>,
    pub unlinkat: SyscallAction<UnlinkatFn<T>>,
    pub utimensat: SyscallAction<UtimensatFn<T>>,

    // dirent.rs system calls.
    pub getdents: SyscallAction<GetdentsFn<T>>,

    // socket.rs system calls.
    pub accept: SyscallAction<AcceptFn<T>>,
    pub bind: SyscallAction<BindFn<T>>,
    pub connect: SyscallAction<ConnectFn<T>>,
    pub getpeername: SyscallAction<GetpeernameFn<T>>,
    pub getsockname: SyscallAction<GetsocknameFn<T>>,
    pub listen: SyscallAction<ListenFn<T>>,
    pub recv: SyscallAction<RecvFn<T>>,
    pub send: SyscallAction<SendFn<T>>,
    pub shutdown: SyscallAction<ShutdownFn<T>>,
    pub socket: SyscallAction<SocketFn<T>>,
    pub socketpair: SyscallAction<SocketpairFn<T>>,

    // poll.rs system calls.
    pub poll: SyscallAction<PollFn<T>>,

    // sys_select.rs system calls.
    pub select: SyscallAction<SelectFn<T>>,

    // times.rs system calls.
    pub times: SyscallAction<TimesFn<T>>,
}

impl<T> SyscallTable<T> {
    ///
    /// # Description
    ///
    /// Creates a new syscall table with the given state.
    ///
    /// # Parameters
    ///
    /// - `state`: State that is passed to each system call action function.
    ///
    /// # Returns
    ///
    /// A new syscall table.
    ///
    pub fn new(state: T) -> Self {
        Self {
            state,

            // unistd.rs system calls.
            chdir: SyscallAction::Forward(default_chdir),
            close: SyscallAction::Forward(default_close),
            faccessat: SyscallAction::Forward(default_faccessat),
            fdatasync: SyscallAction::Forward(default_fdatasync),
            fchdir: SyscallAction::Forward(default_fchdir),
            fchown: SyscallAction::Forward(default_fchown),
            fsync: SyscallAction::Forward(default_fsync),
            ftruncate: SyscallAction::Forward(default_ftruncate),
            getcwd: SyscallAction::Forward(default_getcwd),
            getegid: SyscallAction::Forward(default_getegid),
            geteuid: SyscallAction::Forward(default_geteuid),
            getgid: SyscallAction::Forward(default_getgid),
            getuid: SyscallAction::Forward(default_getuid),
            linkat: SyscallAction::Forward(default_linkat),
            lseek: SyscallAction::Forward(default_lseek),
            pipe: SyscallAction::Forward(default_pipe),
            pread: SyscallAction::Forward(default_pread),
            pwrite: SyscallAction::Forward(default_pwrite),
            read: SyscallAction::Forward(default_read),
            write: SyscallAction::Forward(default_write),

            // fcntl.rs system calls.
            fchmod: SyscallAction::Forward(default_fchmod),
            fchmodat: SyscallAction::Forward(default_fchmodat),
            fchownat: SyscallAction::Forward(default_fchownat),
            fcntl: SyscallAction::Forward(default_fcntl),
            fstat: SyscallAction::Forward(default_fstat),
            fstatat: SyscallAction::Forward(default_fstatat),
            futimens: SyscallAction::Forward(default_futimens),
            mkdirat: SyscallAction::Forward(default_mkdirat),
            openat: SyscallAction::Forward(default_openat),
            posix_fadvise: SyscallAction::Forward(default_posix_fadvise),
            posix_fallocate: SyscallAction::Forward(default_posix_fallocate),
            readlinkat: SyscallAction::Forward(default_readlinkat),
            renameat: SyscallAction::Forward(default_renameat),
            symlinkat: SyscallAction::Forward(default_symlinkat),
            unlinkat: SyscallAction::Forward(default_unlinkat),
            utimensat: SyscallAction::Forward(default_utimensat),

            // dirent.rs system calls.
            getdents: SyscallAction::Forward(default_getdents),

            // socket.rs system calls.
            accept: SyscallAction::Forward(default_accept),
            bind: SyscallAction::Forward(default_bind),
            connect: SyscallAction::Forward(default_connect),
            getpeername: SyscallAction::Forward(default_getpeername),
            getsockname: SyscallAction::Forward(default_getsockname),
            listen: SyscallAction::Forward(default_listen),
            recv: SyscallAction::Forward(default_recv),
            send: SyscallAction::Forward(default_send),
            shutdown: SyscallAction::Forward(default_shutdown),
            socket: SyscallAction::Forward(default_socket),
            socketpair: SyscallAction::Forward(default_socketpair),

            // poll.rs system calls.
            poll: SyscallAction::Forward(default_poll),

            // sys_select.rs system calls.
            select: SyscallAction::Forward(default_select),

            // times.rs system calls.
            times: SyscallAction::Forward(default_times),
        }
    }
}

impl Default for SyscallTable<()> {
    fn default() -> Self {
        Self::new(())
    }
}
