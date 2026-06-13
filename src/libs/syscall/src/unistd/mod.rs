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

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
       #[cfg(feature = "standalone")]
       pub mod fork;
       pub  mod syscall;
        pub use self::syscall::{
            faccessat,
            chdir,
            close,
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
       pub mod bindings;
    }
}
