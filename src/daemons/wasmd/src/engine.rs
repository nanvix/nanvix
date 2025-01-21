// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    pal::Descriptor,
    wasi::{
        types::Errno,
        PreopenedDirectory,
        WasiCtx,
    },
    WasmBinary,
};
use ::alloc::{
    string::{
        String,
        ToString,
    },
    sync::Arc,
    vec::Vec,
};
use ::posix::unistd;
use ::wasmi::{
    errors::ErrorKind,
    Caller,
    Config,
    Engine,
    Func,
    Linker,
    Module,
    Store,
    TypedFunc,
};

//==================================================================================================
// Types
//==================================================================================================

type HostState = u32;

//==================================================================================================
// Structures
//==================================================================================================

pub struct WasmEngine {
    _ctx: Arc<WasiCtx>,
    _engine: Engine,
    store: Store<HostState>,
    _linker: Linker<HostState>,
    _wasm_main: Func,
    start_fn: TypedFunc<(), ()>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl WasmEngine {
    pub fn new(wasm_binary: &WasmBinary, data: HostState) -> Self {
        let mut config: Config = Config::default();
        config.compilation_mode(wasmi::CompilationMode::Eager);
        let engine: Engine = Engine::new(&config);
        let mut store: Store<HostState> = Store::new(&engine, data);
        let mut linker: Linker<HostState> = Linker::new(&engine);
        let mut preopen_dirs: Vec<PreopenedDirectory> = Vec::new();
        preopen_dirs.push(PreopenedDirectory::new(Descriptor::new(3), "/".to_string()));

        let mut envs: Vec<String> = Vec::new();
        envs.push("OS=nanvix".to_string());
        envs.push("HOME=/".to_string());

        let ctx = WasiCtx::new(
            Descriptor::new(posix::unistd::STDIN_FILENO),
            Descriptor::new(posix::unistd::STDOUT_FILENO),
            Descriptor::new(posix::unistd::STDERR_FILENO),
            preopen_dirs,
            envs,
        );

        let ctx: Arc<WasiCtx> = Arc::new(ctx);

        Self::define_args_get(&mut linker, &mut store);
        Self::define_args_sizes_get(&mut linker, &mut store);
        Self::define_clock_res_get(&mut linker, &mut store);
        Self::define_clock_time_get(&mut linker, &mut store);
        Self::define_environ_get(ctx.clone(), &mut linker, &mut store);
        Self::define_environ_sizes_get(ctx.clone(), &mut linker, &mut store);
        Self::define_fd_advise(&mut linker, &mut store);
        Self::define_fd_allocate(&mut linker, &mut store);
        Self::define_fd_close(&mut linker, &mut store);
        Self::define_fd_datasync(&mut linker, &mut store);
        Self::define_fd_fdstat_get(&mut linker, &mut store);
        Self::define_fd_fdstat_set_flags(&mut linker, &mut store);
        Self::define_fd_fdstat_set_rights(&mut linker, &mut store);
        Self::define_fd_filestat_get(&mut linker, &mut store);
        Self::define_fd_filestat_set_size(&mut linker, &mut store);
        Self::define_fd_filestat_set_times(&mut linker, &mut store);
        Self::define_fd_pread(&mut linker, &mut store);
        Self::define_fd_prestat_dir_name(ctx.clone(), &mut linker, &mut store);
        Self::define_fd_prestat_get(ctx.clone(), &mut linker, &mut store);
        Self::define_fd_pwrite(&mut linker, &mut store);
        Self::define_fd_read(&mut linker, &mut store);
        Self::define_fd_readdir(&mut linker, &mut store);
        Self::define_fd_renumber(&mut linker, &mut store);
        Self::define_fd_seek(&mut linker, &mut store);
        Self::define_fd_sync(&mut linker, &mut store);
        Self::define_fd_tell(&mut linker, &mut store);
        Self::define_fd_write(&mut linker, &mut store);
        Self::define_path_create_directory(ctx.clone(), &mut linker, &mut store);
        Self::define_path_filestat_get(&mut linker, &mut store);
        Self::define_path_filestat_set_times(&mut linker, &mut store);
        Self::define_path_link(&mut linker, &mut store);
        Self::define_path_open(ctx.clone(), &mut linker, &mut store);
        Self::define_path_readlink(&mut linker, &mut store);
        Self::define_path_remove_directory(ctx.clone(), &mut linker, &mut store);
        Self::define_path_rename(&mut linker, &mut store);
        Self::define_path_symlink(&mut linker, &mut store);
        Self::define_path_unlink_file(ctx.clone(), &mut linker, &mut store);
        Self::define_poll_oneoff(&mut linker, &mut store);
        Self::define_proc_exit(&mut linker, &mut store);
        Self::define_proc_raise(&mut linker, &mut store);
        Self::define_sched_yield(&mut linker, &mut store);
        Self::define_random_get(&mut linker, &mut store);
        Self::define_sock_accept(&mut linker, &mut store);
        Self::define_sock_recv(&mut linker, &mut store);
        Self::define_sock_send(&mut linker, &mut store);
        Self::define_sock_shutdown(&mut linker, &mut store);

        let module = match Module::new(&engine, &wasm_binary.bytes) {
            Ok(module) => module,
            Err(err) => {
                panic!("Error: {:?}", err);
            },
        };

        let wasm_main: Func = Func::wrap(&mut store, |_caller: Caller<'_, HostState>| {
            ::nvx::log!("wasm_main");
        });

        linker.define("env", "_start", wasm_main).unwrap();
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();

        let start_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&store, "_start").unwrap();

        Self {
            _ctx: ctx,
            _engine: engine,
            store,
            _linker: linker,
            _wasm_main: wasm_main,
            start_fn,
        }
    }

    pub fn run(&mut self) {
        if let Err(e) = self.start_fn.call(&mut self.store, ()) {
            match e.kind() {
                ErrorKind::TrapCode(code) => {
                    ::nvx::log!("Trap: {:?}", code);
                },
                ErrorKind::I32ExitStatus(status) => {
                    ::nvx::log!("Exit status: {:?}", status);
                },
                e => {
                    ::nvx::log!("Error: {:?}", e);
                },
            }
            ::nvx::log!("Error: {:?}", e);
        }
    }

    fn define_args_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let args_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, arg0: i32, arg1: i32| -> i32 {
                ::nvx::log!("args_get: {arg0}, {arg1}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "args_get", args_get)
            .unwrap();
    }

    fn define_args_sizes_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let args_sizes_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, arg0: i32, arg1: i32| -> i32 {
                ::nvx::log!("args_sizes_get: {arg0}, {arg1}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "args_sizes_get", args_sizes_get)
            .unwrap();
    }

    fn define_clock_res_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let clock_res_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, id: i32, offset: i32| -> i32 {
                ::nvx::log!("clock_res_get: {id}, {offset}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "clock_res_get", clock_res_get)
            .unwrap();
    }

    fn define_clock_time_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let clock_time_get: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, id: i32, precision: i64, offset: i32| -> i32 {
                ::nvx::log!("clock_time_get: {id}, {precision}, {offset}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "clock_time_get", clock_time_get)
            .unwrap();
    }

    /// Read environment variables data.
    fn define_environ_get(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let environ_get: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>,
                  environ_ptrs_offset: i32,
                  environ_buf_offset: i32|
                  -> i32 {
                ::nvx::log!(
                    "environ_get(): environ_ptrs_offset={:?}, environ_buf_offset={:?}",
                    environ_ptrs_offset,
                    environ_buf_offset
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "environ_get", environ_get)
            .unwrap();
    }

    /// Read sizes of environment variables data.
    fn define_environ_sizes_get(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let environ_sizes_get: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>,
                  environ_count_offset: i32,
                  environ_data_size_offset: i32|
                  -> i32 {
                ::nvx::log!(
                    "environ_sizes_get(): environ_count_offset={:?}, environ_data_size_offset={:?}",
                    environ_count_offset,
                    environ_data_size_offset
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "environ_sizes_get", environ_sizes_get)
            .unwrap();
    }

    fn define_fd_advise(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_advise: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, fd: i32, offset: i64, len: i64, advice: i32| -> i32 {
                ::nvx::log!("fd_advise: {fd}, {offset}, {len}, {advice}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_advise", fd_advise)
            .unwrap();
    }

    fn define_fd_allocate(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_allocate: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, offset: i64, len: i64| -> i32 {
                ::nvx::log!("fd_allocate: {fd}, {offset}, {len}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_allocate", fd_allocate)
            .unwrap();
    }

    fn define_fd_close(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_close: Func = Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32| -> i32 {
            ::nvx::log!("fd_close: {fd}");
            Errno::Nosys.into()
        });
        linker
            .define("wasi_snapshot_preview1", "fd_close", fd_close)
            .unwrap();
    }

    fn define_fd_datasync(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_datasync: Func = Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32| -> i32 {
            ::nvx::log!("fd_datasync: {fd}");
            Errno::Nosys.into()
        });
        linker
            .define("wasi_snapshot_preview1", "fd_datasync", fd_datasync)
            .unwrap();
    }

    fn define_fd_fdstat_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_fdstat_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, buf: i32| -> i32 {
                ::nvx::log!("fd_fdstat_get: {fd}, {buf}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_fdstat_get", fd_fdstat_get)
            .unwrap();
    }

    fn define_fd_fdstat_set_flags(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_fdstat_set_flags: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, flags: i32| -> i32 {
                ::nvx::log!("fd_fdstat_set_flags: {fd}, {flags}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_fdstat_set_flags", fd_fdstat_set_flags)
            .unwrap();
    }

    fn define_fd_fdstat_set_rights(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_fdstat_set_rights: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             fs_rights_base: i64,
             fs_rights_inheriting: i64|
             -> i32 {
                ::nvx::log!("fd_fdstat_set_rights: {fd}, {fs_rights_base}, {fs_rights_inheriting}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_fdstat_set_rights", fd_fdstat_set_rights)
            .unwrap();
    }

    fn define_fd_filestat_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_filestat_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, buf: i32| -> i32 {
                ::nvx::log!("fd_filestat_get: {fd}, {buf}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_filestat_get", fd_filestat_get)
            .unwrap();
    }

    fn define_fd_filestat_set_size(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_filestat_set_size: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, size: i64| -> i32 {
                ::nvx::log!("fd_filestat_set_size: {fd}, {size}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_filestat_set_size", fd_filestat_set_size)
            .unwrap();
    }

    fn define_fd_filestat_set_times(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_filestat_set_times: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, fd: i32, atim: i64, mtim: i64, fst_flags: i32| -> i32 {
                ::nvx::log!("fd_filestat_set_times: {fd}, {atim}, {mtim}, {fst_flags}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_filestat_set_times", fd_filestat_set_times)
            .unwrap();
    }

    fn define_fd_pread(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_pread: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             iovs_ptr: i32,
             iovs_len: i32,
             offset: i64,
             nread_ptr: i32|
             -> i32 {
                ::nvx::log!("fd_pread: {fd}, {iovs_ptr}, {iovs_len}, {offset} {nread_ptr}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_pread", fd_pread)
            .unwrap();
    }

    /// Returns the path of a pre-opened directory.
    fn define_fd_prestat_dir_name(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_prestat_dir_name: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_buf_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "fd_prestat_dir_name: fd={:?}, path_buf_offset={:?}, path_len={:?}",
                    fd,
                    path_buf_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_prestat_dir_name", fd_prestat_dir_name)
            .unwrap();
    }

    /// Returns a description of a pre-opened capability.
    fn define_fd_prestat_get(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_prestat_get: Func =
            Func::wrap(store, move |_caller: Caller<'_, HostState>, fd: i32, offset: i32| -> i32 {
                ::nvx::log!("fd_prestat_get(): fd={:?}, buf={:#010x}", fd, offset);
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_prestat_get", fd_prestat_get)
            .expect("should be able to add symbol to linker");
    }

    fn define_fd_pwrite(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_pwrite: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             iovs_ptr: i32,
             iovs_len: i32,
             offset: i64,
             nwritten_ptr: i32|
             -> i32 {
                ::nvx::log!("fd_pwrite: {fd}, {iovs_ptr}, {iovs_len}, {offset}, {nwritten_ptr}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_pwrite", fd_pwrite)
            .unwrap();
    }

    fn define_fd_read(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_read: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             iov_buf: i32,
             iov_buf_len: i32,
             offset: i32|
             -> i32 {
                ::nvx::log!("fd_read: {fd}, {iov_buf}, {iov_buf_len}, {offset}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_read", fd_read)
            .unwrap();
    }

    fn define_fd_readdir(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_readdir: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             buf: i32,
             buf_len: i32,
             cookie: i64,
             buf_used: i32|
             -> i32 {
                ::nvx::log!("fd_readdir: {fd}, {buf}, {buf_len}, {cookie}, {buf_used}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_readdir", fd_readdir)
            .unwrap();
    }

    fn define_fd_renumber(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let renumber: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, to: i32| -> i32 {
                ::nvx::log!("fd_renumber: {fd}, {to}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_renumber", renumber)
            .unwrap();
    }

    fn define_fd_seek(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_seek: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, fd: i32, offset: i64, whence: i32, newoffset: i32| -> i32 {
                ::nvx::log!("fd_seek: {fd}, {offset}, {whence}, {newoffset}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_seek", fd_seek)
            .unwrap();
    }

    fn define_fd_sync(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_sync: Func = Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32| -> i32 {
            ::nvx::log!("fd_sync: {fd}");
            Errno::Nosys.into()
        });
        linker
            .define("wasi_snapshot_preview1", "fd_sync", fd_sync)
            .unwrap();
    }

    fn define_fd_tell(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_tell: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, newoffset: i32| -> i32 {
                ::nvx::log!("fd_tell: {fd}, {newoffset}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_tell", fd_tell)
            .unwrap();
    }

    fn define_fd_write(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_write: Func = Func::wrap(
            store,
            |mut caller: Caller<'_, HostState>,
             fd: i32,
             iovs_ptr: i32,
             iovs_len: i32,
             nwritten_ptr: i32| {
                // Ensure fd is 1 (stdout)
                if fd != unistd::STDOUT_FILENO {
                    return Errno::Badf.into();
                }

                let memory = Self::get_memory_mut(&mut caller);

                // Read the iovec array
                let mut total_written = 0;
                for i in 0..iovs_len {
                    let iovec_base = iovs_ptr as usize + i as usize * 8;
                    let ptr =
                        u32::from_le_bytes(memory[iovec_base..iovec_base + 4].try_into().unwrap())
                            as usize;
                    let len = u32::from_le_bytes(
                        memory[iovec_base + 4..iovec_base + 8].try_into().unwrap(),
                    ) as usize;

                    let msg = core::str::from_utf8(&memory[ptr..ptr + len]).expect("Invalid utf8");
                    ::nvx::log!("{msg}");
                    total_written += len;
                }

                // Write the number of bytes written to nwritten_ptr
                let nwritten_bytes = (total_written as u32).to_le_bytes();
                let nwritten_ptr = nwritten_ptr as usize;
                memory[nwritten_ptr..nwritten_ptr + 4].copy_from_slice(&nwritten_bytes);

                0
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_write", fd_write)
            .unwrap();
    }

    fn define_path_create_directory(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_create_directory: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "path_create_directory(): fd={:?}, path_offset={:?}, path_len={:?}",
                    fd,
                    path_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_create_directory", path_create_directory)
            .unwrap();
    }

    fn define_path_filestat_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let path_filestat_get: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             flags: i32,
             buf: i32,
             len: i32,
             offset0: i32|
             -> i32 {
                ::nvx::log!("path_filestat_get: {fd}, {flags}, {buf} {len} {offset0}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_filestat_get", path_filestat_get)
            .unwrap();
    }

    fn define_path_filestat_set_times(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_filestat_set_times: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             flags: i32,
             buf: i32,
             len: i32,
             atim: i64,
             mtim: i64,
             fst_flags: i32|
             -> i32 {
                ::nvx::log!(
                    "path_filestat_set_times: {fd}, {flags}, {buf} {len} {atim} {mtim} {fst_flags}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_filestat_set_times", path_filestat_set_times)
            .unwrap();
    }

    fn define_path_link(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let path_link: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             old_fd: i32,
             old_flags: i32,
             old_path_offset: i32,
             old_path_len: i32,
             new_fd: i32,
             new_path_offset: i32,
             new_path_len: i32|
             -> i32 {
                ::nvx::log!(
                    "path_link: {old_fd}, {old_flags}, {old_path_offset}, {old_path_len}, \
                     {new_fd}, {new_path_offset}, {new_path_len}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_link", path_link)
            .unwrap();
    }

    fn define_path_open(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_open: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>,
                  fd: i32,
                  dirflags: i32,
                  path_offset: i32,
                  path_length: i32,
                  oflags: i32,
                  fs_rights_base: i64,
                  fdflags: i64,
                  fs_rights_inheriting: i32,
                  fd_offset: i32|
                  -> i32 {
                ::nvx::log!(
                    "path_open(): fd={:?}, dirflags={:?}, path_offset={:?}, path_length={:?}, \
                     oflags={:?}, fs_rights_base={:?}, fdflags={:?}, fs_rights_inheriting={:?}, \
                     fd_offset={:?}",
                    fd,
                    dirflags,
                    path_offset,
                    path_length,
                    oflags,
                    fs_rights_base,
                    fdflags,
                    fs_rights_inheriting,
                    fd_offset
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_open", path_open)
            .unwrap();
    }

    fn define_path_readlink(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let path_readlink: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             path_offset: i32,
             path_len: i32,
             buf: i32,
             buf_len: i32,
             offset: i32|
             -> i32 {
                ::nvx::log!(
                    "path_readlink: {fd}, {path_offset}, {path_len}, {buf}, {buf_len} {offset}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_readlink", path_readlink)
            .unwrap();
    }

    fn define_path_remove_directory(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_remove_directory: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "path_remove_directory(): fd={:?}, path_offset={:?}, path_len={:?}",
                    fd,
                    path_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_remove_directory", path_remove_directory)
            .unwrap();
    }

    fn define_path_rename(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let path_rename: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             old_fd: i32,
             old_path_offset: i32,
             old_path_len: i32,
             new_fd: i32,
             new_path_offset: i32,
             new_path_len: i32|
             -> i32 {
                ::nvx::log!(
                    "path_rename: {old_fd}, {old_path_offset}, {old_path_len}, {new_fd}, \
                     {new_path_offset}, {new_path_len}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_rename", path_rename)
            .unwrap();
    }

    fn define_path_symlink(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let path_symlink: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             old_path_offset: i32,
             old_path_len: i32,
             fd: i32,
             new_path_offset: i32,
             new_path_len: i32|
             -> i32 {
                ::nvx::log!(
                    "path_symlink: {old_path_offset}, {old_path_len}, {fd}, {new_path_offset}, \
                     {new_path_len}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_symlink", path_symlink)
            .unwrap();
    }

    fn define_path_unlink_file(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_unlink_file: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "path_unlink_file(): fd={:?}, path_offset={:?}, path_len={:?}",
                    fd,
                    path_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_unlink_file", path_unlink_file)
            .unwrap();
    }

    fn define_poll_oneoff(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let poll_oneoff: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             in_: i32,
             out: i32,
             nsubscriptions: i32,
             nevents: i32|
             -> i32 {
                ::nvx::log!("poll_oneoff: {in_}, {out}, {nsubscriptions}, {nevents}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "poll_oneoff", poll_oneoff)
            .unwrap();
    }

    fn define_proc_exit(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let proc_exit: Func = Func::wrap(store, |_caller: Caller<'_, u32>, arg0: i32| {
            ::nvx::log!("proc_exit: {arg0}");
        });

        linker
            .define("wasi_snapshot_preview1", "proc_exit", proc_exit)
            .unwrap();
    }

    fn define_proc_raise(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let proc_raise: Func = Func::wrap(store, |_caller: Caller<'_, u32>, sig: i32| -> i32 {
            ::nvx::log!("proc_raise: {sig}");
            Errno::Nosys.into()
        });
        linker
            .define("wasi_snapshot_preview1", "proc_raise", proc_raise)
            .unwrap();
    }

    fn define_sched_yield(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let sched_yield: Func = Func::wrap(store, |_caller: Caller<'_, u32>| -> i32 {
            ::nvx::log!("sched_yield");
            Errno::Nosys.into()
        });

        linker
            .define("wasi_snapshot_preview1", "sched_yield", sched_yield)
            .unwrap();
    }

    fn define_random_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let random_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, buf: i32, buf_len: i32| -> i32 {
                ::nvx::log!("random_get: {buf}, {buf_len}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "random_get", random_get)
            .unwrap();
    }

    fn define_sock_accept(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let sock_accept: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, sockfd: i32, flags: i32, sockfd_offset: i32| -> i32 {
                ::nvx::log!("sock_accept: {sockfd}, {flags}, {sockfd_offset}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_accept", sock_accept)
            .unwrap();
    }

    fn define_sock_recv(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let sock_recv: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             sockfd: i32,
             iov_buf: i32,
             iov_buf_len: i32,
             ri_flags: i32,
             offset0: i32,
             offset1: i32|
             -> i32 {
                ::nvx::log!(
                    "sock_recv: {sockfd}, {iov_buf}, {iov_buf_len}, {ri_flags}, {offset0}, \
                     {offset1}",
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_recv", sock_recv)
            .unwrap();
    }

    fn define_sock_send(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let sock_send: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             sockfd: i32,
             iov_buf: i32,
             iov_buf_len: i32,
             si_flags: i32,
             offset0: i32,
             offset1: i32|
             -> i32 {
                ::nvx::log!(
                    "sock_send: {sockfd}, {iov_buf}, {iov_buf_len}, {si_flags}, {offset0}, \
                     {offset1}",
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_send", sock_send)
            .unwrap();
    }

    fn define_sock_shutdown(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let sock_shutdown: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, sockfd: i32, how: i32| -> i32 {
                ::nvx::log!("sock_shutdown: {sockfd}, {how}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "sock_shutdown", sock_shutdown)
            .unwrap();
    }

    /// Convenient function for getting a mutable reference to the memory of the WASM module.
    fn get_memory_mut<'a>(caller: &'a mut Caller<'_, HostState>) -> &'a mut [u8] {
        caller
            .get_export("memory")
            .expect("memory should be present")
            .into_memory()
            .expect("should be able to cast memory")
            .data_mut(caller)
    }
}
