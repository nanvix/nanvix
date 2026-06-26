// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

// The getopt binding wraps a host-testable back-end, so it is compiled whenever the `syscall`
// feature, the `std` feature, or `test` is enabled. The remaining bindings depend on kernel-call
// machinery and are therefore gated behind the `syscall` feature.
#[cfg(any(feature = "syscall", feature = "std", test))]
pub mod getopt;
#[cfg(any(feature = "syscall", feature = "std", test))]
pub mod getopt_long;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        pub mod _exit;
        pub mod access;
        pub mod alarm;
        pub mod chdir;
        pub mod chown;
        pub mod chroot;
        pub mod dup;
        pub mod dup2;
        pub mod execv;
        pub mod execve;
        pub mod execvp;
        pub mod faccessat;
        pub mod fchdir;
        pub mod fchown;
        pub mod fchownat;
        pub mod fdatasync;
        pub mod fork;
        pub mod fsync;
        pub mod ftruncate;
        pub mod getcwd;
        pub mod getegid;
        pub mod getentropy;
        pub mod geteuid;
        pub mod getgid;
        pub mod getgroups;
        pub mod gethostid;
        pub mod gethostname;
        pub mod getlogin_r;
        pub mod getpagesize;
        pub mod getpid;
        pub mod getppid;
        pub mod getsid;
        pub mod getuid;
        pub mod isatty;
        pub mod lchown;
        pub mod link;
        pub mod linkat;
        pub mod lseek;
        pub mod pread;
        pub mod pwrite;
        pub mod read;
        pub mod readlink;
        pub mod readlinkat;
        pub mod rmdir;
        pub mod setegid;
        pub mod seteuid;
        pub mod setgid;
        pub mod setgroups;
        pub mod setsid;
        pub mod setuid;
        pub mod sleep;
        pub mod symlink;
        pub mod symlinkat;
        pub mod sync;
        pub mod sysconf;
        pub mod ttyname_r;
        pub mod unlink;
        pub mod vfork;
        pub mod wait;
        pub mod waitpid;
        pub mod write;
    }
}
