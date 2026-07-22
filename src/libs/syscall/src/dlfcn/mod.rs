// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(all(feature = "syscall", feature = "dlfcn"))] {
        mod syscall;
        pub use syscall::DlHandle;
        pub use syscall::dlclose;
        pub use syscall::dlopen;
        pub use syscall::dlsym;
        pub use syscall::dladdr;
        pub use syscall::dlinit;
        pub use syscall::dllink_executable;
        pub use syscall::dlfini_executable;
    }
}

//==================================================================================================
// Exports
//==================================================================================================

pub use ::sysapi::dlfcn::DlInfo;
