// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

// The getopt back-end is pure Rust and host-testable, so it is compiled whenever the `syscall`
// feature, the `std` feature, or `test` is enabled. The remaining modules depend on kernel-call
// machinery and are therefore gated behind the `syscall` feature.
#[cfg(any(feature = "syscall", feature = "std", test))]
mod getopt;

// The scatter/gather chunking helper in `util` is pure Rust and host-testable, so it is also
// compiled under `test`. Its only non-test consumers (`read`/`write`) are gated behind the
// `syscall` feature, which keeps `util` available there too.
#[cfg(any(feature = "syscall", test))]
mod util;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod _exit;
        mod cancel;
        mod chdir;
        mod close;
        mod dup;
        mod dup2;
        mod faccessat;
        mod fchdir;
        mod fchown;
        mod fchownat;
        mod fdatasync;
        mod fsync;
        mod ftruncate;
        mod getcwd;
        mod getegid;
        mod geteuid;
        mod getgid;
        mod gethostname;
        mod getpid;
        mod getppid;
        mod getuid;
        mod isatty;
        mod link;
        mod linkat;
        mod lseek;
        mod pipe;
        mod pread;
        mod pwrite;
        mod read;
        mod readlink;
        mod readlinkat;
        mod symlink;
        mod symlinkat;
        mod truncate;
        mod unlink;
        mod write;
    }
}

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(any(feature = "syscall", feature = "std", test))]
pub use self::getopt::{
    getopt,
    GetoptState,
};

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        pub use self::{
            _exit::_exit,
            chdir::chdir,
            close::close,
            dup::dup,
            dup2::dup2,
            faccessat::faccessat,
            fchdir::fchdir,
            fchown::fchown,
            fchownat::fchownat,
            fdatasync::fdatasync,
            fsync::fsync,
            ftruncate::ftruncate,
            getcwd::getcwd,
            getegid::getegid,
            geteuid::geteuid,
            getgid::getgid,
            gethostname::gethostname,
            getpid::getpid,
            getppid::getppid,
            getuid::getuid,
            isatty::isatty,
            link::link,
            linkat::linkat,
            lseek::lseek,
            pipe::pipe,
            pread::pread,
            pwrite::pwrite,
            read::read,
            readlink::readlink,
            readlinkat::readlinkat,
            symlink::symlink,
            symlinkat::symlinkat,
            truncate::truncate,
            unlink::unlink,
            write::write,
        };
    }
}
