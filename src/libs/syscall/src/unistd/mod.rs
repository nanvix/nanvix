// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

//==================================================================================================

// The getopt back-end and its C ABI binding are host-testable, so the `syscall` and `bindings`
// modules are compiled whenever the `syscall` feature, the `std` feature, or `test` is enabled.
// Only the getopt items are exported under this relaxed gate; the remaining items depend on the
// kernel-call machinery and require the `syscall` feature.
#[cfg(any(feature = "syscall", feature = "std", test))]
pub mod syscall;

#[cfg(any(feature = "syscall", feature = "std", test))]
pub mod bindings;

#[cfg(any(feature = "syscall", feature = "std", test))]
pub use self::syscall::getopt;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        #[cfg(feature = "standalone")]
        pub mod fork;
        pub mod exec;
        pub use self::exec::{
            do_execv,
            exec_startup_barrier,
            execv_from_c,
            execv_inherit_env_from_c,
        };
        pub use self::syscall::{
            faccessat,
            chdir,
            close,
            dup,
            dup2,
            _exit,
            fdatasync,
            fchown,
            fchownat,
            ftruncate,
            fsync,
            getegid,
            geteuid,
            getgid,
            getpid,
            getppid,
            getuid,
            gethostname,
            link,
            linkat,
            lseek,
            symlinkat,
            pread,
            pwrite,
            read,
            readlink,
            readlinkat,
            symlink,
            unlink,
            write,
            pipe,
            getcwd,
            fchdir,
            isatty,
        };
    }
}
