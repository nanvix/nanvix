// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mm::VirtualAddress;

//==================================================================================================
// Structures
//==================================================================================================

/// Argument structure used with the `execv()` kernel call.
///
/// The kernel does not perform any filesystem I/O, so the calling process is responsible for
/// loading the program image into its own address space before issuing the kernel call. This
/// structure describes, in user space, the raw ELF image to load as well as the argument and
/// environment strings to install for the new image.
///
/// The argument and environment strings follow the same single-string, space-separated convention
/// used by the process spawner at boot: each is a run of tokens separated by spaces, without a
/// terminating NUL (the length is carried explicitly).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ExecvArgs {
    /// Base address of the raw ELF image in the caller's address space.
    pub elf_ptr: VirtualAddress,

    /// Size of the raw ELF image, in bytes.
    pub elf_len: usize,

    /// Base address of the argument string in the caller's address space.
    pub args_ptr: VirtualAddress,

    /// Length of the argument string, in bytes (excluding any terminating NUL).
    pub args_len: usize,

    /// Base address of the environment string in the caller's address space.
    pub env_ptr: VirtualAddress,

    /// Length of the environment string, in bytes (excluding any terminating NUL).
    pub env_len: usize,
}

impl Default for ExecvArgs {
    fn default() -> Self {
        Self {
            elf_ptr: VirtualAddress::from_raw_value(0),
            elf_len: 0,
            args_ptr: VirtualAddress::from_raw_value(0),
            args_len: 0,
            env_ptr: VirtualAddress::from_raw_value(0),
            env_len: 0,
        }
    }
}
