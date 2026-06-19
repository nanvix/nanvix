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
        pub mod exec;
        pub use self::exec::{
            do_execv,
            execv_from_c,
            execv_inherit_env_from_c,
        };
        pub mod syscall;
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
        pub mod bindings;
    }
}
