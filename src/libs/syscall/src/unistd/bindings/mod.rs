// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod _exit;
pub mod access;
pub mod chdir;
pub mod chown;
pub mod chroot;
pub mod dup;
pub mod dup2;
pub mod execv;
pub mod execve;
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
pub mod gethostname;
pub mod getpid;
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
pub mod setgroups;
pub mod write;

#[cfg(all(feature = "syscall", feature = "sbrk"))]
pub mod sbrk;
