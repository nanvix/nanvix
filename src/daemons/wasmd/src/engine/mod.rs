// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod clock;
mod environ;
mod fd;
mod path;
mod proc;
mod sock;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pal::Descriptor,
    wasi::{
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

        let args: Vec<String> = Vec::new();

        let ctx = WasiCtx::new(
            Descriptor::new(posix::unistd::STDIN_FILENO),
            Descriptor::new(posix::unistd::STDOUT_FILENO),
            Descriptor::new(posix::unistd::STDERR_FILENO),
            preopen_dirs,
            envs,
            args,
        );

        let ctx: Arc<WasiCtx> = Arc::new(ctx);

        Self::define_args_get(&mut linker, &mut store);
        Self::define_args_sizes_get(ctx.clone(), &mut linker, &mut store);
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
