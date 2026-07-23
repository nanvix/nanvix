// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Extern Crates
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_long,
        c_longlong,
        c_short,
        c_uchar,
        c_uint,
        c_ulong,
        c_ulonglong,
        c_ushort,
    },
    sched::sched_param,
    sys_types::{
        blkcnt_t,
        blksize_t,
        c_size_t,
        c_ssize_t,
        clock_t,
        clockid_t,
        dev_t,
        gid_t,
        ino_t,
        mode_t,
        nlink_t,
        off_t,
        pid_t,
        reclen_t,
        time_t,
        uid_t,
    },
    time::timespec,
    unistd::STDOUT_FILENO,
};
use ::syscall::unistd;
use core::mem::{
    align_of,
    size_of,
};

//==================================================================================================
// Static Assertions: Signed Primitive Types
//==================================================================================================

const _: () = assert!(size_of::<c_char>() == 1);
const _: () = assert!(size_of::<c_short>() == 2);
const _: () = assert!(size_of::<c_int>() == 4);
const _: () = assert!(size_of::<c_long>() == 4);
const _: () = assert!(size_of::<c_longlong>() == 8);
const _: () = assert!(size_of::<f32>() == 4);
const _: () = assert!(size_of::<f64>() == 8);

//==================================================================================================
// Static Assertions: Unsigned Primitive Types
//==================================================================================================

const _: () = assert!(size_of::<c_uchar>() == 1);
const _: () = assert!(size_of::<c_ushort>() == 2);
const _: () = assert!(size_of::<c_uint>() == 4);
const _: () = assert!(size_of::<c_ulong>() == 4);
const _: () = assert!(size_of::<c_ulonglong>() == 8);

//==================================================================================================
// Static Assertions: Fixed-Width Integer Types (<stdint.h>)
//==================================================================================================

const _: () = assert!(size_of::<i8>() == 1);
const _: () = assert!(size_of::<i16>() == 2);
const _: () = assert!(size_of::<i32>() == 4);
const _: () = assert!(size_of::<i64>() == 8);
const _: () = assert!(size_of::<u8>() == 1);
const _: () = assert!(size_of::<u16>() == 2);
const _: () = assert!(size_of::<u32>() == 4);
const _: () = assert!(size_of::<u64>() == 8);

//==================================================================================================
// Static Assertions: System Types (<sys/types.h>)
//==================================================================================================

const _: () = assert!(size_of::<blkcnt_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<blksize_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<clock_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<clockid_t>() == size_of::<c_int>());
const _: () = assert!(size_of::<dev_t>() == size_of::<c_ulonglong>());
const _: () = assert!(size_of::<gid_t>() == size_of::<c_uint>());
const _: () = assert!(size_of::<ino_t>() == size_of::<c_ulonglong>());
const _: () = assert!(size_of::<mode_t>() == size_of::<c_uint>());
const _: () = assert!(size_of::<nlink_t>() == size_of::<c_ulonglong>());
const _: () = assert!(size_of::<off_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<pid_t>() == size_of::<c_int>());
const _: () = assert!(size_of::<reclen_t>() == size_of::<c_ushort>());
const _: () = assert!(size_of::<c_size_t>() == size_of::<c_uint>());
const _: () = assert!(size_of::<c_ssize_t>() == size_of::<c_int>());
const _: () = assert!(size_of::<time_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<uid_t>() == size_of::<c_uint>());

//==================================================================================================
// Static Assertions: Time Types (<time.h>)
//==================================================================================================

const _: () = assert!(size_of::<timespec>() == size_of::<time_t>() + size_of::<c_long>());
const _: () = assert!(align_of::<timespec>() == align_of::<time_t>());

//==================================================================================================
// Static Assertions: Scheduling Types (<sched.h>)
//==================================================================================================

const _: () = assert!(size_of::<sched_param>() == size_of::<c_int>());
const _: () = assert!(align_of::<sched_param>() == align_of::<c_int>());

//==================================================================================================
// Link-Time Symbol Presence Check
//==================================================================================================

// Force the linker to resolve all #[no_mangle] extern "C" symbols exported by the syscall crate.
// If any symbol is accidentally removed (e.g., by a feature gate), linking this binary will fail.
//
// Symbols are declared as opaque statics (not functions) to avoid ABI signature mismatches that
// could trigger LLVM LTO type-mismatch failures. The linker only needs to resolve the symbol
// address; the actual calling convention is irrelevant here.
unsafe extern "C" {
    static __nanvix_sys_cached_pid: u8;
    static __errno_location: u8;
    static _exit: u8;
    static accept: u8;
    static access: u8;
    static bind: u8;
    static chdir: u8;
    static chown: u8;
    static chroot: u8;
    static clock_getres: u8;
    static clock_gettime: u8;
    static close: u8;
    static closedir: u8;
    static connect: u8;
    static dirfd: u8;
    static dup: u8;
    static dup2: u8;
    static execv: u8;
    static execve: u8;
    static faccessat: u8;
    static fchdir: u8;
    static fchown: u8;
    static fchownat: u8;
    static fcntl: u8;
    static fdatasync: u8;
    static fork: u8;
    static fstat: u8;
    static fsync: u8;
    static ftruncate: u8;
    static getcwd: u8;
    static getegid: u8;
    static getentropy: u8;
    static geteuid: u8;
    static getgid: u8;
    static gethostname: u8;
    static getpeername: u8;
    static getpid: u8;
    static getsockname: u8;
    static getsockopt: u8;
    static gettimeofday: u8;
    static getuid: u8;
    static isatty: u8;
    static kill: u8;
    static lchown: u8;
    static link: u8;
    static linkat: u8;
    static listen: u8;
    static lseek: u8;
    static mmap: u8;
    static mprotect: u8;
    static munmap: u8;
    static nanosleep: u8;
    static open: u8;
    static opendir: u8;
    static pipe: u8;
    static poll: u8;
    static posix_fadvise: u8;
    static posix_fallocate: u8;
    static pread: u8;
    static pthread_atfork: u8;
    static pthread_attr_destroy: u8;
    static pthread_attr_getstack: u8;
    static pthread_attr_init: u8;
    static pthread_attr_setstacksize: u8;
    static pthread_cond_broadcast: u8;
    static pthread_cond_destroy: u8;
    static pthread_cond_init: u8;
    static pthread_cond_signal: u8;
    static pthread_cond_timedwait: u8;
    static pthread_cond_wait: u8;
    static pthread_condattr_destroy: u8;
    static pthread_condattr_getclock: u8;
    static pthread_condattr_init: u8;
    static pthread_condattr_setclock: u8;
    static pthread_create: u8;
    static pthread_getattr_np: u8;
    static pthread_getschedparam: u8;
    static pthread_getspecific: u8;
    static pthread_join: u8;
    static pthread_key_create: u8;
    static pthread_kill: u8;
    static pthread_mutex_destroy: u8;
    static pthread_mutex_init: u8;
    static pthread_mutex_lock: u8;
    static pthread_mutex_unlock: u8;
    static pthread_rwlock_destroy: u8;
    static pthread_rwlock_init: u8;
    static pthread_rwlock_rdlock: u8;
    static pthread_rwlock_unlock: u8;
    static pthread_rwlock_wrlock: u8;
    static pthread_self: u8;
    static pthread_setcancelstate: u8;
    static pthread_setspecific: u8;
    static pthread_sigmask: u8;
    static pwrite: u8;
    static read: u8;
    static readdir: u8;
    static readlink: u8;
    static readlinkat: u8;
    static recv: u8;
    static recvfrom: u8;
    static recvmsg: u8;
    static renameat: u8;
    static rmdir: u8;
    static sched_yield: u8;
    static select: u8;
    static sem_destroy: u8;
    static sem_init: u8;
    static sem_post: u8;
    static sem_wait: u8;
    static send: u8;
    static sendmsg: u8;
    static sendto: u8;
    static setegid: u8;
    static seteuid: u8;
    static setgid: u8;
    static setgroups: u8;
    static setsockopt: u8;
    static setuid: u8;
    static shutdown: u8;
    static sleep: u8;
    static socket: u8;
    static socketpair: u8;
    static stat: u8;
    static symlink: u8;
    static symlinkat: u8;
    static sysconf: u8;
    static unlink: u8;
    static unlinkat: u8;
    static usleep: u8;
    static waitpid: u8;
    static write: u8;
}

/// Wrapper to make raw pointers usable in statics (extern statics are inherently immutable).
#[repr(transparent)]
struct SymAddr(*const u8);

// SAFETY: These are addresses of extern symbols resolved at link time; they are never mutated.
unsafe impl Sync for SymAddr {}

/// Force the linker to retain all syscall symbols by referencing their addresses.
#[used]
static SYSCALL_SYMBOLS: [SymAddr; 132] = [
    SymAddr(&raw const __nanvix_sys_cached_pid),
    SymAddr(&raw const __errno_location),
    SymAddr(&raw const _exit),
    SymAddr(&raw const accept),
    SymAddr(&raw const access),
    SymAddr(&raw const bind),
    SymAddr(&raw const chdir),
    SymAddr(&raw const chown),
    SymAddr(&raw const chroot),
    SymAddr(&raw const clock_getres),
    SymAddr(&raw const clock_gettime),
    SymAddr(&raw const close),
    SymAddr(&raw const closedir),
    SymAddr(&raw const connect),
    SymAddr(&raw const dirfd),
    SymAddr(&raw const dup),
    SymAddr(&raw const dup2),
    SymAddr(&raw const execv),
    SymAddr(&raw const execve),
    SymAddr(&raw const faccessat),
    SymAddr(&raw const fchdir),
    SymAddr(&raw const fchown),
    SymAddr(&raw const fchownat),
    SymAddr(&raw const fcntl),
    SymAddr(&raw const fdatasync),
    SymAddr(&raw const fork),
    SymAddr(&raw const fstat),
    SymAddr(&raw const fsync),
    SymAddr(&raw const ftruncate),
    SymAddr(&raw const getcwd),
    SymAddr(&raw const getegid),
    SymAddr(&raw const getentropy),
    SymAddr(&raw const geteuid),
    SymAddr(&raw const getgid),
    SymAddr(&raw const gethostname),
    SymAddr(&raw const getpeername),
    SymAddr(&raw const getpid),
    SymAddr(&raw const getsockname),
    SymAddr(&raw const getsockopt),
    SymAddr(&raw const gettimeofday),
    SymAddr(&raw const getuid),
    SymAddr(&raw const isatty),
    SymAddr(&raw const kill),
    SymAddr(&raw const lchown),
    SymAddr(&raw const link),
    SymAddr(&raw const linkat),
    SymAddr(&raw const listen),
    SymAddr(&raw const lseek),
    SymAddr(&raw const mmap),
    SymAddr(&raw const mprotect),
    SymAddr(&raw const munmap),
    SymAddr(&raw const nanosleep),
    SymAddr(&raw const open),
    SymAddr(&raw const opendir),
    SymAddr(&raw const pipe),
    SymAddr(&raw const poll),
    SymAddr(&raw const posix_fadvise),
    SymAddr(&raw const posix_fallocate),
    SymAddr(&raw const pread),
    SymAddr(&raw const pthread_atfork),
    SymAddr(&raw const pthread_attr_destroy),
    SymAddr(&raw const pthread_attr_getstack),
    SymAddr(&raw const pthread_attr_init),
    SymAddr(&raw const pthread_attr_setstacksize),
    SymAddr(&raw const pthread_cond_broadcast),
    SymAddr(&raw const pthread_cond_destroy),
    SymAddr(&raw const pthread_cond_init),
    SymAddr(&raw const pthread_cond_signal),
    SymAddr(&raw const pthread_cond_timedwait),
    SymAddr(&raw const pthread_cond_wait),
    SymAddr(&raw const pthread_condattr_destroy),
    SymAddr(&raw const pthread_condattr_getclock),
    SymAddr(&raw const pthread_condattr_init),
    SymAddr(&raw const pthread_condattr_setclock),
    SymAddr(&raw const pthread_create),
    SymAddr(&raw const pthread_getattr_np),
    SymAddr(&raw const pthread_getschedparam),
    SymAddr(&raw const pthread_getspecific),
    SymAddr(&raw const pthread_join),
    SymAddr(&raw const pthread_key_create),
    SymAddr(&raw const pthread_kill),
    SymAddr(&raw const pthread_mutex_destroy),
    SymAddr(&raw const pthread_mutex_init),
    SymAddr(&raw const pthread_mutex_lock),
    SymAddr(&raw const pthread_mutex_unlock),
    SymAddr(&raw const pthread_rwlock_destroy),
    SymAddr(&raw const pthread_rwlock_init),
    SymAddr(&raw const pthread_rwlock_rdlock),
    SymAddr(&raw const pthread_rwlock_unlock),
    SymAddr(&raw const pthread_rwlock_wrlock),
    SymAddr(&raw const pthread_self),
    SymAddr(&raw const pthread_setcancelstate),
    SymAddr(&raw const pthread_setspecific),
    SymAddr(&raw const pthread_sigmask),
    SymAddr(&raw const pwrite),
    SymAddr(&raw const read),
    SymAddr(&raw const readdir),
    SymAddr(&raw const readlink),
    SymAddr(&raw const readlinkat),
    SymAddr(&raw const recv),
    SymAddr(&raw const recvfrom),
    SymAddr(&raw const recvmsg),
    SymAddr(&raw const renameat),
    SymAddr(&raw const rmdir),
    SymAddr(&raw const sched_yield),
    SymAddr(&raw const select),
    SymAddr(&raw const sem_destroy),
    SymAddr(&raw const sem_init),
    SymAddr(&raw const sem_post),
    SymAddr(&raw const sem_wait),
    SymAddr(&raw const send),
    SymAddr(&raw const sendmsg),
    SymAddr(&raw const sendto),
    SymAddr(&raw const setegid),
    SymAddr(&raw const seteuid),
    SymAddr(&raw const setgid),
    SymAddr(&raw const setgroups),
    SymAddr(&raw const setsockopt),
    SymAddr(&raw const setuid),
    SymAddr(&raw const shutdown),
    SymAddr(&raw const sleep),
    SymAddr(&raw const socket),
    SymAddr(&raw const socketpair),
    SymAddr(&raw const stat),
    SymAddr(&raw const symlink),
    SymAddr(&raw const symlinkat),
    SymAddr(&raw const sysconf),
    SymAddr(&raw const unlink),
    SymAddr(&raw const unlinkat),
    SymAddr(&raw const usleep),
    SymAddr(&raw const waitpid),
    SymAddr(&raw const write),
];

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Magic string for CI harness.
    {
        let magic_string: &[u8] = b"ok";
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
