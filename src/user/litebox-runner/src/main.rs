// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================
extern crate alloc;

use alloc::vec::Vec;
use litebox::{
    fs::{
        tar_ro::empty_tar_file,
        FileSystem,
    },
    LiteBox,
};
use litebox_nanvix::NanvixUserland;
use posix::nvx::sys::error::Error;

static PROG_DATA: &[u8] = include_bytes!("../test-artifacts/hello.hooked");

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    // TODO(jb): Clean up platform initialization once we have https://github.com/MSRSSP/litebox/issues/24
    //
    let platform = &NanvixUserland;
    let litebox = LiteBox::new(platform);
    let initial_file_system = {
        let mut in_mem = litebox::fs::in_mem::FileSystem::new(&litebox);
        in_mem.with_root_privileges(|fs| {
            let fd = fs
                .open(
                    "/hello",
                    litebox::fs::OFlags::WRONLY | litebox::fs::OFlags::CREAT,
                    litebox::fs::Mode::from_bits(0o755).unwrap(),
                )
                .unwrap();
            let mut data = PROG_DATA;
            while !data.is_empty() {
                let len = fs.write(&fd, data, None).unwrap();
                data = &data[len..];
            }
            fs.close(fd).unwrap();
        });
        let tar_ro = litebox::fs::tar_ro::FileSystem::new(&litebox, empty_tar_file());
        let dev_stdio = litebox::fs::devices::stdio::FileSystem::new(&litebox);
        litebox::fs::layered::FileSystem::new(
            &litebox,
            in_mem,
            litebox::fs::layered::FileSystem::new(
                &litebox,
                dev_stdio,
                tar_ro,
                litebox::fs::layered::LayeringSemantics::LowerLayerReadOnly,
            ),
            litebox::fs::layered::LayeringSemantics::LowerLayerWritableFiles,
        )
    };
    litebox_shim_linux::set_fs(initial_file_system);
    litebox_platform_multiplex::set_platform(platform);

    let argv = Vec::new();
    let envp = Vec::new();

    let loaded_program = litebox_shim_linux::loader::load_program("/hello", argv, envp).unwrap();

    unsafe {
        trampoline::jump_to_entry_point(loaded_program.entry_point, loaded_program.user_stack_top)
    }
}

mod trampoline {
    #[cfg(target_arch = "x86_64")]
    core::arch::global_asm!(
        "
    .text
    .align  4
    .globl  jump_to_entry_point
    .type   jump_to_entry_point,@function
jump_to_entry_point:
    xor rdx, rdx
    mov     rsp, rsi
    jmp     rdi
    /* Should not reach. */
    hlt"
    );
    #[cfg(target_arch = "x86")]
    core::arch::global_asm!(
        "
    .text
    .align  4
    .globl  jump_to_entry_point
    .type   jump_to_entry_point,@function
jump_to_entry_point:
    xor     edx, edx
    mov     ebx, [esp + 4]
    mov     eax, [esp + 8]
    mov     esp, eax
    jmp     ebx
    /* Should not reach. */
    hlt"
    );
    unsafe extern "C" {
        pub(crate) fn jump_to_entry_point(entry_point: usize, stack_pointer: usize) -> !;
    }
}
