// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    engine::{
        HostState,
        WasmEngine,
    },
    memory::WriteBytes,
    wasi::{
        types::Errno,
        WasiCtx,
    },
};
use ::alloc::{
    ffi::CString,
    string::String,
    sync::Arc,
    vec::Vec,
};
use ::core::mem;
use ::wasmi::{
    Caller,
    Func,
    Linker,
    Store,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl WasmEngine {
    pub(super) fn define_args_get(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let args_get: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>, argv_offset: i32, argv_buf_offset: i32| -> i32 {
                ::nvx::trace!(
                    "args_get(): argv_offset={:?}, argv_buf_offset={:?}",
                    argv_offset,
                    argv_buf_offset
                );

                let memory: &mut [u8] = WasmEngine::get_memory_mut(&mut caller);

                // Attempt to convent command-line argument offset.
                let argv_offset: u32 = match argv_offset.try_into() {
                    Ok(argv_offset) => argv_offset,
                    _ => {
                        ::nvx::error!("args_get(): invalid argv_offset {:#010x}", argv_offset);
                        return Errno::Inval.into();
                    },
                };

                // Attempt to convert command-line argument data offset.
                let argv_buf_offset: u32 = match argv_buf_offset.try_into() {
                    Ok(argv_buf_offset) => argv_buf_offset,
                    _ => {
                        ::nvx::error!(
                            "args_get(): invalid argv_buf_offset {:#010x}",
                            argv_buf_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                // Get command-line arguments.
                let args: Vec<String> = match ctx.args_get() {
                    Ok(args) => args,
                    Err(err) => {
                        ::nvx::error!("args_get(): {:?}", err);
                        return err.into();
                    },
                };

                let write_args = |memory: &mut [u8],
                                  mut argv_offset: u32,
                                  mut argv_buf_offset: u32,
                                  dry_run: bool|
                 -> Result<(), Errno> {
                    for arg in &args {
                        ::nvx::trace!("args_get(): arg={:?}", arg);

                        let arg_cstr: CString = match CString::new(arg.as_str()) {
                            Ok(arg_cstr) => arg_cstr,
                            Err(_) => {
                                ::nvx::error!("args_get(): skipping invalid command-line argument");
                                continue;
                            },
                        };

                        let arg_cstr_bytes: &[u8] = arg_cstr.as_bytes_with_nul();

                        // Check if memory is too small to store the command-line argument pointer.
                        if memory.len() < argv_offset as usize + mem::size_of_val(&argv_offset) {
                            ::nvx::error!(
                                "args_get(): buffer too small (size={:?}, required={:?})",
                                memory.len(),
                                argv_offset as usize + mem::size_of_val(&argv_offset)
                            );
                            return Err(Errno::Inval.into());
                        }

                        // Check if memory is too small to store the command-line argument data.
                        if memory.len() < argv_buf_offset as usize + arg_cstr_bytes.len() {
                            ::nvx::error!(
                                "args_get(): buffer too small (size={:?}, required={:?})",
                                memory.len(),
                                argv_buf_offset as usize + arg_cstr_bytes.len()
                            );
                            return Err(Errno::Inval.into());
                        }

                        // Check if changes should be written to memory.
                        if !dry_run {
                            // Write the command-line argument pointer to data buffer.
                            argv_buf_offset.write_le_bytes(&mut memory[argv_offset as usize..]);
                            // Write the command-line argument data to data buffer.
                            arg_cstr_bytes.write_le_bytes(&mut memory[argv_buf_offset as usize..]);
                        }

                        argv_offset += mem::size_of::<u32>() as u32;
                        argv_buf_offset += arg_cstr_bytes.len() as u32;
                    }

                    Ok(())
                };

                // Run a dry-run first to check if we encounter any errors.
                if let Err(err) = write_args(memory, argv_offset, argv_buf_offset, false) {
                    return err.into();
                }

                // Re-run to persist changes.
                if write_args(memory, argv_offset, argv_buf_offset, true).is_err() {
                    unreachable!("any errors should have been caught in the dry-run");
                }

                Errno::Success.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "args_get", args_get)
            .unwrap();
    }

    pub(super) fn define_args_sizes_get(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let args_sizes_get: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  args_count_offset: i32,
                  args_data_size_offset: i32|
                  -> i32 {
                ::nvx::trace!(
                    "args_sizes_get(): args_count_offset={:?}, args_data_size_offset={:?}",
                    args_count_offset,
                    args_data_size_offset
                );

                let memory: &mut [u8] = WasmEngine::get_memory_mut(&mut caller);

                // Attempt to convert args_count_offset.
                let args_count_offset: usize = match args_count_offset.try_into() {
                    Ok(args_count_offset) => args_count_offset,
                    _ => {
                        ::nvx::error!(
                            "args_sizes_get(): invalid args_count_offset {:#010x}",
                            args_count_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                // Attempt to convert args_data_size_offset.
                let args_data_size_offset: usize = match args_data_size_offset.try_into() {
                    Ok(args_data_size_offset) => args_data_size_offset,
                    _ => {
                        ::nvx::error!(
                            "args_sizes_get(): invalid args_data_size_offset {:#010x}",
                            args_data_size_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                let (args_count, args_data_size): (u32, u32) = match ctx.args_sizes_get() {
                    Ok((args_count, args_data_size)) => (args_count.into(), args_data_size.into()),
                    Err(err) => {
                        ::nvx::error!("args_sizes_get(): {:?}", err);
                        return err.into();
                    },
                };

                // Ensure that data buffer is large enough to store the command-line arguments.
                if memory.len() < args_count_offset + mem::size_of_val(&args_count) {
                    ::nvx::error!(
                        "args_sizes_get(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        args_count_offset + mem::size_of_val(&args_count)
                    );
                    return Errno::Inval.into();
                }

                // Ensure that data buffer is large enough to store the command-line data size.
                if memory.len() < args_data_size_offset + mem::size_of_val(&args_data_size) {
                    ::nvx::error!(
                        "args_sizes_get(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        args_data_size_offset + mem::size_of_val(&args_data_size)
                    );
                    return Errno::Inval.into();
                }

                // NOTE: The offsets have been validated, so we can safely write data to memory.

                // Write the number of command-line arguments to memory.
                args_count.write_le_bytes(&mut memory[args_count_offset..]);

                // Write the size of the command-line data to memory.
                args_data_size.write_le_bytes(&mut memory[args_data_size_offset..]);

                Errno::Success.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "args_sizes_get", args_sizes_get)
            .unwrap();
    }

    /// Read environment variables data.
    pub(super) fn define_environ_get(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let environ_get: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  environ_ptrs_offset: i32,
                  environ_buf_offset: i32|
                  -> i32 {
                ::nvx::trace!(
                    "environ_get(): environ_ptrs_offset={:?}, environ_buf_offset={:?}",
                    environ_ptrs_offset,
                    environ_buf_offset
                );

                let memory: &mut [u8] = WasmEngine::get_memory_mut(&mut caller);

                // Attempt to convert environ_ptrs_offset.
                let environ_ptrs_offset: u32 = match environ_ptrs_offset.try_into() {
                    Ok(environ_ptrs_offset) => environ_ptrs_offset,
                    _ => {
                        ::nvx::error!(
                            "environ_get(): invalid environ_ptrs_offset {:#010x}",
                            environ_ptrs_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                // Attempt to convert environ_buf_offset.
                let env_buf_offset: u32 = match environ_buf_offset.try_into() {
                    Ok(environ_buf_offset) => environ_buf_offset,
                    _ => {
                        ::nvx::error!(
                            "environ_get(): invalid environ_buf_offset {:#010x}",
                            environ_buf_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                // Get environment variables.
                let envs: Vec<String> = match ctx.environ_get() {
                    Ok(envs) => envs,
                    Err(err) => {
                        ::nvx::error!("environ_get(): {:?}", err);
                        return err.into();
                    },
                };

                let write_envs = |memory: &mut [u8],
                                  mut environ_ptrs_offset: u32,
                                  mut env_buf_offset: u32,
                                  dry_run: bool|
                 -> Result<(), Errno> {
                    for env in &envs {
                        ::nvx::trace!("environ_get(): env={:?}", env);

                        let env_cstr: CString = match CString::new(env.as_str()) {
                            Ok(env_cstr) => env_cstr,
                            Err(_) => {
                                ::nvx::error!(
                                    "environ_get(): skipping invalid environment variable"
                                );
                                continue;
                            },
                        };

                        let env_cstr_bytes: &[u8] = env_cstr.as_bytes_with_nul();

                        // Check if memory is too small to store the environment pointer.
                        if memory.len()
                            < environ_ptrs_offset as usize + mem::size_of_val(&environ_ptrs_offset)
                        {
                            ::nvx::error!(
                                "environ_get(): buffer too small (size={:?}, required={:?})",
                                memory.len(),
                                environ_ptrs_offset as usize
                                    + mem::size_of_val(&environ_ptrs_offset)
                            );
                            return Err(Errno::Inval.into());
                        }

                        // Check if memory is too small to store the environment data.
                        if memory.len() < env_buf_offset as usize + env_cstr_bytes.len() {
                            ::nvx::error!(
                                "environ_get(): buffer too small (size={:?}, required={:?})",
                                memory.len(),
                                env_buf_offset as usize + env_cstr_bytes.len()
                            );
                            return Err(Errno::Inval.into());
                        }

                        // Check if changes should be written to memory.
                        if !dry_run {
                            // Write the environment pointer to data buffer.
                            env_buf_offset
                                .write_le_bytes(&mut memory[environ_ptrs_offset as usize..]);
                            // Write the environment data to data buffer.
                            env_cstr_bytes.write_le_bytes(&mut memory[env_buf_offset as usize..]);
                        }

                        environ_ptrs_offset += mem::size_of::<u32>() as u32;
                        env_buf_offset += env_cstr_bytes.len() as u32;
                    }

                    Ok(())
                };

                // Run a dry-run first to check if we encounter any errors.
                if let Err(err) = write_envs(memory, environ_ptrs_offset, env_buf_offset, false) {
                    return err.into();
                }

                // Re-run to persist changes.
                if write_envs(memory, environ_ptrs_offset, env_buf_offset, true).is_err() {
                    unreachable!("any errors should have been caught in the dry-run");
                }

                Errno::Success.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "environ_get", environ_get)
            .unwrap();
    }

    /// Read sizes of environment variables data.
    pub(super) fn define_environ_sizes_get(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let environ_sizes_get: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  environ_count_offset: i32,
                  environ_data_size_offset: i32|
                  -> i32 {
                ::nvx::trace!(
                    "environ_sizes_get(): environ_count_offset={:?}, environ_data_size_offset={:?}",
                    environ_count_offset,
                    environ_data_size_offset
                );

                let memory: &mut [u8] = WasmEngine::get_memory_mut(&mut caller);

                // Attempt to convert environ_count_offset.
                let environ_count_offset: usize = match environ_count_offset.try_into() {
                    Ok(environ_count_offset) => environ_count_offset,
                    _ => {
                        ::nvx::error!(
                            "environ_sizes_get(): invalid environ_count_offset {:#010x}",
                            environ_count_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                // Attempt to convert environ_data_size_offset.
                let environ_data_size_offset: usize = match environ_data_size_offset.try_into() {
                    Ok(environ_data_size_offset) => environ_data_size_offset,
                    _ => {
                        ::nvx::error!(
                            "environ_sizes_get(): invalid environ_data_size_offset {:#010x}",
                            environ_data_size_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                let (environ_count, environ_data_size): (u32, u32) = match ctx.environ_sizes_get() {
                    Ok((environ_count, environ_data_size)) => {
                        (environ_count.into(), environ_data_size.into())
                    },
                    Err(err) => {
                        ::nvx::error!("environ_sizes_get(): {:?}", err);
                        return err.into();
                    },
                };

                // Ensure that data buffer is large enough to store the environment data.
                if memory.len() < environ_count_offset + mem::size_of_val(&environ_count) {
                    ::nvx::error!(
                        "environ_sizes_get(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        environ_count_offset + mem::size_of_val(&environ_count)
                    );
                    return Errno::Inval.into();
                }

                // Ensure that data buffer is large enough to store the environment data size.
                if memory.len() < environ_data_size_offset + mem::size_of_val(&environ_data_size) {
                    ::nvx::error!(
                        "environ_sizes_get(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        environ_data_size_offset + mem::size_of_val(&environ_data_size)
                    );
                    return Errno::Inval.into();
                }

                // NOTE: The offsets have been validated, so we can safely write data to memory.

                // Write the number of environment variables to memory.
                environ_count.write_le_bytes(&mut memory[environ_count_offset..]);

                // Write the size of the environment data to memory.
                environ_data_size.write_le_bytes(&mut memory[environ_data_size_offset..]);

                Errno::Success.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "environ_sizes_get", environ_sizes_get)
            .unwrap();
    }
}
