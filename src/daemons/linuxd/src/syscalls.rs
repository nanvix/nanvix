// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Function Type Aliases
//==================================================================================================

/// Type alias for `times()` system call function.
pub type TimesFn = unsafe fn(*mut libc::tms) -> libc::clock_t;

/// Type alias for `chdir()` system call function.
pub type ChdirFn = unsafe fn(*const libc::c_char) -> libc::c_int;

/// Type alias for `close()` system call function.
pub type CloseFn = unsafe fn(libc::c_int) -> libc::c_int;

/// Type alias for `faccessat()` system call function.
pub type FaccessatFn =
    unsafe fn(libc::c_int, *const libc::c_char, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `fdatasync()` system call function.
pub type FdatasyncFn = unsafe fn(libc::c_int) -> libc::c_int;

/// Type alias for `getuid()` system call function.
pub type GetuidFn = unsafe fn() -> libc::uid_t;

/// Type alias for `geteuid()` system call function.
pub type GeteuidFn = unsafe fn() -> libc::uid_t;

/// Type alias for `getgid()` system call function.
pub type GetgidFn = unsafe fn() -> libc::gid_t;

/// Type alias for `getegid()` system call function.
pub type GetegidFn = unsafe fn() -> libc::gid_t;

/// Type alias for `getcwd()` system call function.
pub type GetcwdFn = unsafe fn(*mut libc::c_char, libc::size_t) -> *mut libc::c_char;

/// Type alias for `fsync()` system call function.
pub type FsyncFn = unsafe fn(libc::c_int) -> libc::c_int;

/// Type alias for `lseek()` system call function.
pub type LseekFn = unsafe fn(libc::c_int, libc::off_t, libc::c_int) -> libc::off_t;

/// Type alias for `ftruncate()` system call function.
pub type FtruncateFn = unsafe fn(libc::c_int, libc::off_t) -> libc::c_int;

/// Type alias for `write()` system call function.
pub type WriteFn = unsafe fn(libc::c_int, *const libc::c_void, libc::size_t) -> libc::ssize_t;

/// Type alias for `read()` system call function.
pub type ReadFn = unsafe fn(libc::c_int, *mut libc::c_void, libc::size_t) -> libc::ssize_t;

/// Type alias for `pwrite()` system call function.
pub type PwriteFn =
    unsafe fn(libc::c_int, *const libc::c_void, libc::size_t, libc::off_t) -> libc::ssize_t;

/// Type alias for `pread()` system call function.
pub type PreadFn =
    unsafe fn(libc::c_int, *mut libc::c_void, libc::size_t, libc::off_t) -> libc::ssize_t;

/// Type alias for `linkat()` system call function.
pub type LinkatFn = unsafe fn(
    libc::c_int,
    *const libc::c_char,
    libc::c_int,
    *const libc::c_char,
    libc::c_int,
) -> libc::c_int;

/// Type alias for `fchdir()` system call function.
pub type FchdirFn = unsafe fn(libc::c_int) -> libc::c_int;

/// Type alias for `fchown()` system call function.
pub type FchownFn = unsafe fn(libc::c_int, libc::uid_t, libc::gid_t) -> libc::c_int;

/// Type alias for `pipe()` system call function.
pub type PipeFn = unsafe fn(*mut libc::c_int) -> libc::c_int;

/// Type alias for `getdents()` system call function.
pub type GetdentsFn = unsafe fn(libc::c_int, *mut u8, libc::size_t) -> libc::c_long;

/// Type alias for `socket()` system call function.
pub type SocketFn = unsafe fn(libc::c_int, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `socketpair()` system call function.
pub type SocketpairFn =
    unsafe fn(libc::c_int, libc::c_int, libc::c_int, *mut libc::c_int) -> libc::c_int;

/// Type alias for `bind()` system call function.
pub type BindFn = unsafe fn(libc::c_int, *const libc::sockaddr, libc::socklen_t) -> libc::c_int;

/// Type alias for `connect()` system call function.
pub type ConnectFn = unsafe fn(libc::c_int, *const libc::sockaddr, libc::socklen_t) -> libc::c_int;

/// Type alias for `listen()` system call function.
pub type ListenFn = unsafe fn(libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `getpeername()` system call function.
pub type GetpeernameFn =
    unsafe fn(libc::c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int;

/// Type alias for `getsockname()` system call function.
pub type GetsocknameFn =
    unsafe fn(libc::c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int;

/// Type alias for `accept()` system call function.
pub type AcceptFn =
    unsafe fn(libc::c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int;

/// Type alias for `recv()` system call function.
pub type RecvFn =
    unsafe fn(libc::c_int, *mut libc::c_void, libc::size_t, libc::c_int) -> libc::ssize_t;

/// Type alias for `send()` system call function.
pub type SendFn =
    unsafe fn(libc::c_int, *const libc::c_void, libc::size_t, libc::c_int) -> libc::ssize_t;

/// Type alias for `shutdown()` system call function.
pub type ShutdownFn = unsafe fn(libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `select()` system call function.
pub type SelectFn = unsafe fn(
    libc::c_int,
    *mut libc::fd_set,
    *mut libc::fd_set,
    *mut libc::fd_set,
    *mut libc::timeval,
) -> libc::c_int;

/// Type alias for `poll()` system call function.
pub type PollFn = unsafe fn(*mut libc::pollfd, libc::nfds_t, libc::c_int) -> libc::c_int;

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
/// - `path`: Path to change to.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_chdir(path: *const libc::c_char) -> libc::c_int {
    libc::chdir(path)
}

///
/// # Description
///
/// Default implementation for `close()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor to close.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_close(fd: libc::c_int) -> libc::c_int {
    libc::close(fd)
}

///
/// # Description
///
/// Default implementation for `faccessat()` system call.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to check.
/// - `mode`: Access mode.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_faccessat(
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
/// - `fd`: File descriptor to synchronize.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fdatasync(fd: libc::c_int) -> libc::c_int {
    libc::fdatasync(fd)
}

///
/// # Description
///
/// Default implementation for `getuid()` system call.
///
/// # Returns
///
/// The real user ID of the calling process.
///
pub unsafe fn default_getuid() -> libc::uid_t {
    libc::getuid()
}

///
/// # Description
///
/// Default implementation for `geteuid()` system call.
///
/// # Returns
///
/// The effective user ID of the calling process.
///
pub unsafe fn default_geteuid() -> libc::uid_t {
    libc::geteuid()
}

///
/// # Description
///
/// Default implementation for `getgid()` system call.
///
/// # Returns
///
/// The real group ID of the calling process.
///
pub unsafe fn default_getgid() -> libc::gid_t {
    libc::getgid()
}

///
/// # Description
///
/// Default implementation for `getegid()` system call.
///
/// # Returns
///
/// The effective group ID of the calling process.
///
pub unsafe fn default_getegid() -> libc::gid_t {
    libc::getegid()
}

///
/// # Description
///
/// Default implementation for `getcwd()` system call.
///
/// # Parameters
///
/// - `buf`: Buffer to store current working directory.
/// - `size`: Size of buffer.
///
/// # Returns
///
/// Upon successful completion, a pointer to the buffer is returned. Otherwise, NULL is returned and `errno` is set.
///
pub unsafe fn default_getcwd(buf: *mut libc::c_char, size: libc::size_t) -> *mut libc::c_char {
    libc::getcwd(buf, size)
}

///
/// # Description
///
/// Default implementation for `fsync()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor to synchronize.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fsync(fd: libc::c_int) -> libc::c_int {
    libc::fsync(fd)
}

///
/// # Description
///
/// Default implementation for `lseek()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset.
/// - `whence`: Whence.
///
/// # Returns
///
/// Upon successful completion, the resulting offset is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_lseek(
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
/// - `fd`: File descriptor.
/// - `length`: Length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_ftruncate(fd: libc::c_int, length: libc::off_t) -> libc::c_int {
    libc::ftruncate(fd, length)
}

///
/// # Description
///
/// Default implementation for `write()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buf`: Buffer to write from.
/// - `count`: Number of bytes to write.
///
/// # Returns
///
/// Upon successful completion, the number of bytes written is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_write(
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
/// - `fd`: File descriptor.
/// - `buf`: Buffer to read into.
/// - `count`: Number of bytes to read.
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_read(
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
/// - `fd`: File descriptor.
/// - `buf`: Buffer to write from.
/// - `count`: Number of bytes to write.
/// - `offset`: Offset.
///
/// # Returns
///
/// Upon successful completion, the number of bytes written is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_pwrite(
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
/// - `fd`: File descriptor.
/// - `buf`: Buffer to read into.
/// - `count`: Number of bytes to read.
/// - `offset`: Offset.
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_pread(
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
pub unsafe fn default_linkat(
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
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchdir(fd: libc::c_int) -> libc::c_int {
    libc::fchdir(fd)
}

///
/// # Description
///
/// Default implementation for `fchown()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `owner`: Owner.
/// - `group`: Group.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchown(
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
/// - `pipefd`: Pipe file descriptors.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_pipe(pipefd: *mut libc::c_int) -> libc::c_int {
    libc::pipe(pipefd)
}

/// Type alias for `openat()` system call function.
pub type OpenatFn =
    unsafe fn(libc::c_int, *const libc::c_char, libc::c_int, libc::mode_t) -> libc::c_int;

/// Type alias for `unlinkat()` system call function.
pub type UnlinkatFn = unsafe fn(libc::c_int, *const libc::c_char, libc::c_int) -> libc::c_int;

/// Type alias for `renameat()` system call function.
pub type RenameatFn =
    unsafe fn(libc::c_int, *const libc::c_char, libc::c_int, *const libc::c_char) -> libc::c_int;

/// Type alias for `fstatat()` system call function.
pub type FstatatFn =
    unsafe fn(libc::c_int, *const libc::c_char, *mut libc::stat, libc::c_int) -> libc::c_int;

/// Type alias for `posix_fallocate()` system call function.
pub type PosixFallocateFn = unsafe fn(libc::c_int, libc::off_t, libc::off_t) -> libc::c_int;

/// Type alias for `posix_fadvise()` system call function.
pub type PosixFadviseFn =
    unsafe fn(libc::c_int, libc::off_t, libc::off_t, libc::c_int) -> libc::c_int;

/// Type alias for `fstat()` system call function.
pub type FstatFn = unsafe fn(libc::c_int, *mut libc::stat) -> libc::c_int;

/// Type alias for `symlinkat()` system call function.
pub type SymlinkatFn =
    unsafe fn(*const libc::c_char, libc::c_int, *const libc::c_char) -> libc::c_int;

/// Type alias for `readlinkat()` system call function.
pub type ReadlinkatFn =
    unsafe fn(libc::c_int, *const libc::c_char, *mut libc::c_char, libc::size_t) -> libc::ssize_t;

/// Type alias for `mkdirat()` system call function.
pub type MkdiratFn = unsafe fn(libc::c_int, *const libc::c_char, libc::mode_t) -> libc::c_int;

/// Type alias for `utimensat()` system call function.
pub type UtimensatFn =
    unsafe fn(libc::c_int, *const libc::c_char, *const libc::timespec, libc::c_int) -> libc::c_int;

/// Type alias for `futimens()` system call function.
pub type FutimensFn = unsafe fn(libc::c_int, *const libc::timespec) -> libc::c_int;

/// Type alias for `fcntl()` system call function.
pub type FcntlFn = unsafe fn(libc::c_int, libc::c_int, libc::c_int) -> libc::c_int;

/// Type alias for `fchownat()` system call function.
pub type FchownatFn = unsafe fn(
    libc::c_int,
    *const libc::c_char,
    libc::uid_t,
    libc::gid_t,
    libc::c_int,
) -> libc::c_int;

/// Type alias for `fchmod()` system call function.
pub type FchmodFn = unsafe fn(libc::c_int, libc::mode_t) -> libc::c_int;

/// Type alias for `fchmodat()` system call function.
pub type FchmodatFn =
    unsafe fn(libc::c_int, *const libc::c_char, libc::mode_t, libc::c_int) -> libc::c_int;

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
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to open.
/// - `flags`: Flags.
/// - `mode`: Mode.
///
/// # Returns
///
/// Upon successful completion, a file descriptor is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_openat(
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
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to unlink.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_unlinkat(
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
/// - `olddirfd`: Old directory file descriptor.
/// - `oldpath`: Old path.
/// - `newdirfd`: New directory file descriptor.
/// - `newpath`: New path.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_renameat(
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
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to stat.
/// - `buf`: Buffer to store stat.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fstatat(
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
/// - `fd`: File descriptor.
/// - `offset`: Offset.
/// - `len`: Length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, an error number is returned.
///
pub unsafe fn default_posix_fallocate(
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
/// - `fd`: File descriptor.
/// - `offset`: Offset.
/// - `len`: Length.
/// - `advice`: Advice.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, an error number is returned.
///
pub unsafe fn default_posix_fadvise(
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
/// - `fd`: File descriptor.
/// - `buf`: Buffer to store stat.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fstat(fd: libc::c_int, buf: *mut libc::stat) -> libc::c_int {
    libc::fstat(fd, buf)
}

///
/// # Description
///
/// Default implementation for `symlinkat()` system call.
///
/// # Parameters
///
/// - `target`: Target.
/// - `newdirfd`: New directory file descriptor.
/// - `linkpath`: Link path.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_symlinkat(
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
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to read.
/// - `buf`: Buffer to store link.
/// - `bufsiz`: Buffer size.
///
/// # Returns
///
/// Upon successful completion, the number of bytes placed in the buffer is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_readlinkat(
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
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to create.
/// - `mode`: Mode.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_mkdirat(
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
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to update.
/// - `times`: Times.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_utimensat(
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
/// - `fd`: File descriptor.
/// - `times`: Times.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_futimens(fd: libc::c_int, times: *const libc::timespec) -> libc::c_int {
    libc::futimens(fd, times)
}

///
/// # Description
///
/// Default implementation for `fcntl()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `cmd`: Command.
/// - `arg`: Argument.
///
/// # Returns
///
/// Upon successful completion, a value depends on the command. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fcntl(fd: libc::c_int, cmd: libc::c_int, arg: libc::c_int) -> libc::c_int {
    libc::fcntl(fd, cmd, arg)
}

///
/// # Description
///
/// Default implementation for `fchownat()` system call.
///
/// # Parameters
///
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
pub unsafe fn default_fchownat(
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
/// - `fd`: File descriptor.
/// - `mode`: Mode.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchmod(fd: libc::c_int, mode: libc::mode_t) -> libc::c_int {
    libc::fchmod(fd, mode)
}

///
/// # Description
///
/// Default implementation for `fchmodat()` system call.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Path to change mode.
/// - `mode`: Mode.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_fchmodat(
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
/// - `fd`: File descriptor.
/// - `dirp`: Directory entries buffer.
/// - `count`: Buffer size.
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_getdents(
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
/// - `domain`: Domain.
/// - `type_`: Type.
/// - `protocol`: Protocol.
///
/// # Returns
///
/// Upon successful completion, a file descriptor is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_socket(
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
/// - `domain`: Domain.
/// - `type_`: Type.
/// - `protocol`: Protocol.
/// - `sv`: Socket pair.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_socketpair(
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
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_bind(
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
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_connect(
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
/// - `sockfd`: Socket file descriptor.
/// - `backlog`: Backlog.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_listen(sockfd: libc::c_int, backlog: libc::c_int) -> libc::c_int {
    libc::listen(sockfd, backlog)
}

///
/// # Description
///
/// Default implementation for `getpeername()` system call.
///
/// # Parameters
///
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_getpeername(
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
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_getsockname(
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
/// - `sockfd`: Socket file descriptor.
/// - `addr`: Address.
/// - `addrlen`: Address length.
///
/// # Returns
///
/// Upon successful completion, a file descriptor is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_accept(
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
/// - `sockfd`: Socket file descriptor.
/// - `buf`: Buffer.
/// - `len`: Length.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, the number of bytes received is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_recv(
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
/// - `sockfd`: Socket file descriptor.
/// - `buf`: Buffer.
/// - `len`: Length.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, the number of bytes sent is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_send(
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
/// - `sockfd`: Socket file descriptor.
/// - `how`: How.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_shutdown(sockfd: libc::c_int, how: libc::c_int) -> libc::c_int {
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
/// - `fds`: File descriptors.
/// - `nfds`: Number of file descriptors.
/// - `timeout`: Timeout.
///
/// # Returns
///
/// Upon successful completion, the number of file descriptors with events is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_poll(
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
pub unsafe fn default_select(
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
/// - `buf`: Buffer to store times.
///
/// # Returns
///
/// Upon successful completion, the elapsed real time in clock ticks is returned. Otherwise, -1 is returned and `errno` is set.
///
pub unsafe fn default_times(buf: *mut libc::tms) -> libc::clock_t {
    libc::times(buf)
}

pub enum SystemCallAction<F> {
    Block,
    Forward(F),
}

pub struct SystemCallRouteTable {
    // unistd.rs system calls.
    pub syscall_chdir: SystemCallAction<ChdirFn>,
    pub syscall_close: SystemCallAction<CloseFn>,
    pub syscall_faccessat: SystemCallAction<FaccessatFn>,
    pub syscall_fdatasync: SystemCallAction<FdatasyncFn>,
    pub syscall_fchdir: SystemCallAction<FchdirFn>,
    pub syscall_fchown: SystemCallAction<FchownFn>,
    pub syscall_fsync: SystemCallAction<FsyncFn>,
    pub syscall_ftruncate: SystemCallAction<FtruncateFn>,
    pub syscall_getcwd: SystemCallAction<GetcwdFn>,
    pub syscall_getegid: SystemCallAction<GetegidFn>,
    pub syscall_geteuid: SystemCallAction<GeteuidFn>,
    pub syscall_getgid: SystemCallAction<GetgidFn>,
    pub syscall_getuid: SystemCallAction<GetuidFn>,
    pub syscall_linkat: SystemCallAction<LinkatFn>,
    pub syscall_lseek: SystemCallAction<LseekFn>,
    pub syscall_pipe: SystemCallAction<PipeFn>,
    pub syscall_pread: SystemCallAction<PreadFn>,
    pub syscall_pwrite: SystemCallAction<PwriteFn>,
    pub syscall_read: SystemCallAction<ReadFn>,
    pub syscall_write: SystemCallAction<WriteFn>,

    // fcntl.rs system calls.
    pub syscall_fchmod: SystemCallAction<FchmodFn>,
    pub syscall_fchmodat: SystemCallAction<FchmodatFn>,
    pub syscall_fchownat: SystemCallAction<FchownatFn>,
    pub syscall_fcntl: SystemCallAction<FcntlFn>,
    pub syscall_fstat: SystemCallAction<FstatFn>,
    pub syscall_fstatat: SystemCallAction<FstatatFn>,
    pub syscall_futimens: SystemCallAction<FutimensFn>,
    pub syscall_mkdirat: SystemCallAction<MkdiratFn>,
    pub syscall_openat: SystemCallAction<OpenatFn>,
    pub syscall_posix_fadvise: SystemCallAction<PosixFadviseFn>,
    pub syscall_posix_fallocate: SystemCallAction<PosixFallocateFn>,
    pub syscall_readlinkat: SystemCallAction<ReadlinkatFn>,
    pub syscall_renameat: SystemCallAction<RenameatFn>,
    pub syscall_symlinkat: SystemCallAction<SymlinkatFn>,
    pub syscall_unlinkat: SystemCallAction<UnlinkatFn>,
    pub syscall_utimensat: SystemCallAction<UtimensatFn>,

    // dirent.rs system calls.
    pub syscall_getdents: SystemCallAction<GetdentsFn>,

    // socket.rs system calls.
    pub syscall_accept: SystemCallAction<AcceptFn>,
    pub syscall_bind: SystemCallAction<BindFn>,
    pub syscall_connect: SystemCallAction<ConnectFn>,
    pub syscall_getpeername: SystemCallAction<GetpeernameFn>,
    pub syscall_getsockname: SystemCallAction<GetsocknameFn>,
    pub syscall_listen: SystemCallAction<ListenFn>,
    pub syscall_recv: SystemCallAction<RecvFn>,
    pub syscall_send: SystemCallAction<SendFn>,
    pub syscall_shutdown: SystemCallAction<ShutdownFn>,
    pub syscall_socket: SystemCallAction<SocketFn>,
    pub syscall_socketpair: SystemCallAction<SocketpairFn>,

    // poll.rs system calls.
    pub syscall_poll: SystemCallAction<PollFn>,

    // sys_select.rs system calls.
    pub syscall_select: SystemCallAction<SelectFn>,

    // times.rs system calls.
    pub syscall_times: SystemCallAction<TimesFn>,
}

impl Default for SystemCallRouteTable {
    fn default() -> Self {
        Self {
            // unistd.rs system calls.
            syscall_chdir: SystemCallAction::Forward(default_chdir),
            syscall_close: SystemCallAction::Forward(default_close),
            syscall_faccessat: SystemCallAction::Forward(default_faccessat),
            syscall_fdatasync: SystemCallAction::Forward(default_fdatasync),
            syscall_fchdir: SystemCallAction::Forward(default_fchdir),
            syscall_fchown: SystemCallAction::Forward(default_fchown),
            syscall_fsync: SystemCallAction::Forward(default_fsync),
            syscall_ftruncate: SystemCallAction::Forward(default_ftruncate),
            syscall_getcwd: SystemCallAction::Forward(default_getcwd),
            syscall_getegid: SystemCallAction::Forward(default_getegid),
            syscall_geteuid: SystemCallAction::Forward(default_geteuid),
            syscall_getgid: SystemCallAction::Forward(default_getgid),
            syscall_getuid: SystemCallAction::Forward(default_getuid),
            syscall_linkat: SystemCallAction::Forward(default_linkat),
            syscall_lseek: SystemCallAction::Forward(default_lseek),
            syscall_pipe: SystemCallAction::Forward(default_pipe),
            syscall_pread: SystemCallAction::Forward(default_pread),
            syscall_pwrite: SystemCallAction::Forward(default_pwrite),
            syscall_read: SystemCallAction::Forward(default_read),
            syscall_write: SystemCallAction::Forward(default_write),

            // fcntl.rs system calls.
            syscall_fchmod: SystemCallAction::Forward(default_fchmod),
            syscall_fchmodat: SystemCallAction::Forward(default_fchmodat),
            syscall_fchownat: SystemCallAction::Forward(default_fchownat),
            syscall_fcntl: SystemCallAction::Forward(default_fcntl),
            syscall_fstat: SystemCallAction::Forward(default_fstat),
            syscall_fstatat: SystemCallAction::Forward(default_fstatat),
            syscall_futimens: SystemCallAction::Forward(default_futimens),
            syscall_mkdirat: SystemCallAction::Forward(default_mkdirat),
            syscall_openat: SystemCallAction::Forward(default_openat),
            syscall_posix_fadvise: SystemCallAction::Forward(default_posix_fadvise),
            syscall_posix_fallocate: SystemCallAction::Forward(default_posix_fallocate),
            syscall_readlinkat: SystemCallAction::Forward(default_readlinkat),
            syscall_renameat: SystemCallAction::Forward(default_renameat),
            syscall_symlinkat: SystemCallAction::Forward(default_symlinkat),
            syscall_unlinkat: SystemCallAction::Forward(default_unlinkat),
            syscall_utimensat: SystemCallAction::Forward(default_utimensat),

            // dirent.rs system calls.
            syscall_getdents: SystemCallAction::Forward(default_getdents),

            // socket.rs system calls.
            syscall_accept: SystemCallAction::Forward(default_accept),
            syscall_bind: SystemCallAction::Forward(default_bind),
            syscall_connect: SystemCallAction::Forward(default_connect),
            syscall_getpeername: SystemCallAction::Forward(default_getpeername),
            syscall_getsockname: SystemCallAction::Forward(default_getsockname),
            syscall_listen: SystemCallAction::Forward(default_listen),
            syscall_recv: SystemCallAction::Forward(default_recv),
            syscall_send: SystemCallAction::Forward(default_send),
            syscall_shutdown: SystemCallAction::Forward(default_shutdown),
            syscall_socket: SystemCallAction::Forward(default_socket),
            syscall_socketpair: SystemCallAction::Forward(default_socketpair),

            // poll.rs system calls.
            syscall_poll: SystemCallAction::Forward(default_poll),

            // sys_select.rs system calls.
            syscall_select: SystemCallAction::Forward(default_select),

            // times.rs system calls.
            syscall_times: SystemCallAction::Forward(default_times),
        }
    }
}
