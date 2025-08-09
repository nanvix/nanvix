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
    pal::{
        self,
        fs::{
            File,
            Path,
        },
        socket::Socket,
    },
    wasi::{
        Fd,
        Rights,
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
use ::sys::error::{Error, ErrorCode};
use ::syscall::sys::socket::SocketAddr;
use ::wasmi::{
    errors::ErrorKind,
    Caller,
    Config,
    Engine,
    Func,
    Instance,
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

pub struct WasiFile {
    wasi_fd: Fd,
    os_file: File,
    base_rights: Rights,
    _inherited_rights: Rights,
}

pub struct WasiSocket {
    wasi_socket: Fd,
    os_socket: Socket,
    base_rights: Rights,
    _inherited_rights: Rights,
}

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

impl WasiFile {
    pub fn new(wasi_fd: Fd, os_file: File, base_rights: Rights, inherited_rights: Rights) -> Self {
        Self {
            wasi_fd,
            os_file,
            base_rights,
            _inherited_rights: inherited_rights,
        }
    }

    pub fn file(&self) -> &File {
        &self.os_file
    }

    pub fn file_mut(&mut self) -> &mut File {
        &mut self.os_file
    }

    pub fn fd(&self) -> Fd {
        self.wasi_fd
    }

    pub fn rights_base(&self) -> &Rights {
        &self.base_rights
    }
}

impl WasiSocket {
    pub fn new(
        wasi_socket: Fd,
        os_socket: Socket,
        base_rights: &Rights,
        inherited_rights: &Rights,
    ) -> Self {
        Self {
            wasi_socket,
            os_socket,
            base_rights: base_rights.clone(),
            _inherited_rights: inherited_rights.clone(),
        }
    }

    pub fn fd(&self) -> Fd {
        self.wasi_socket
    }

    pub fn socket(&self) -> &Socket {
        &self.os_socket
    }

    pub fn rights_base(&self) -> &Rights {
        &self.base_rights
    }

    pub fn rights_inheriting(&self) -> &Rights {
        &self._inherited_rights
    }
}

impl WasmEngine {
    pub fn new(wasm_binary: &WasmBinary, data: HostState, sockaddr: &Option<SocketAddr>) -> Result<Self, Error> {
        let mut next_wasi_fd: Fd = 0;
        let mut config: Config = Config::default();
        config.compilation_mode(wasmi::CompilationMode::Eager);
        let engine: Engine = Engine::new(&config);
        let mut store: Store<HostState> = Store::new(&engine, data);
        let mut linker: Linker<HostState> = Linker::new(&engine);
        let mut preopen_dirs: Vec<(WasiFile, String)> = Vec::new();
        let mut preopen_sockets: Vec<WasiSocket> = Vec::new();

        let mut files: Vec<WasiFile> = Vec::new();

        // Standard input (stdin)
        let stdin: WasiFile = WasiFile::new(
            next_wasi_fd,
            File::stdin(),
            Rights::base_rights(),
            Rights::base_rights(),
        );
        files.push(stdin);
        next_wasi_fd += 1;
        // Standard output (stdout)
        let stdout: WasiFile = WasiFile::new(
            next_wasi_fd,
            File::stdout(),
            Rights::base_rights(),
            Rights::base_rights(),
        );
        files.push(stdout);
        next_wasi_fd += 1;
        // Standard error (stderr)
        let stderr: WasiFile = WasiFile::new(
            next_wasi_fd,
            File::stderr(),
            Rights::base_rights(),
            Rights::base_rights(),
        );
        files.push(stderr);
        next_wasi_fd += 1;

        // Root directory.
        let file: File = match File::open(&Path::new(".")) {
            Ok(file) => file,
            Err(err) => {
                ::syslog::error!("failed to open root directory: {:?}", err);
                return Err(Error::new(ErrorCode::IoErr, "failed to open root directory"));
            },
        };
        let root: WasiFile =
            WasiFile::new(next_wasi_fd, file, Rights::base_rights(), Rights::base_rights());
        next_wasi_fd += 1;
        preopen_dirs.push((root, ".".to_string()));

        // Populate pre-open sockets.
        if let Some(sockaddr) = sockaddr {
            let os_socket: Socket = match pal::setup_network(sockaddr) {
                Ok(socket) => socket,
                Err(err) => {
                    ::syslog::error!("failed to setup network: {:?}", err);
                    return Err(Error::new(ErrorCode::NetworkDown, "failed to setup network"));
                },
            };
            let localhost: WasiSocket = WasiSocket::new(
                next_wasi_fd,
                os_socket,
                &Rights::base_rights(),
                &Rights::base_rights(),
            );
            preopen_sockets.push(localhost);
            next_wasi_fd += 1;
        }

        let envs: Vec<String> = alloc::vec!["OS=nanvix".to_string(), "HOME=/".to_string()];

        let mut args: Vec<String> = Vec::new();
        args.push(wasm_binary.name.clone());
        args.extend(wasm_binary.args.clone());

        let ctx: WasiCtx =
            WasiCtx::new(next_wasi_fd, files, preopen_dirs, preopen_sockets, envs, args);

        let ctx: Arc<WasiCtx> = Arc::new(ctx);

        Self::define_args_get(ctx.clone(), &mut linker, &mut store);
        Self::define_args_sizes_get(ctx.clone(), &mut linker, &mut store);
        Self::define_clock_res_get(&mut linker, &mut store);
        Self::define_clock_time_get(&mut linker, &mut store);
        Self::define_environ_get(ctx.clone(), &mut linker, &mut store);
        Self::define_environ_sizes_get(ctx.clone(), &mut linker, &mut store);
        Self::define_fd_advise(&mut linker, &mut store);
        Self::define_fd_allocate(&mut linker, &mut store);
        Self::define_fd_close(ctx.clone(), &mut linker, &mut store);
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
        Self::define_fd_read(ctx.clone(), &mut linker, &mut store);
        Self::define_fd_readdir(&mut linker, &mut store);
        Self::define_fd_renumber(&mut linker, &mut store);
        Self::define_fd_seek(ctx.clone(), &mut linker, &mut store);
        Self::define_fd_sync(&mut linker, &mut store);
        Self::define_fd_tell(ctx.clone(), &mut linker, &mut store);
        Self::define_fd_write(ctx.clone(), &mut linker, &mut store);
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
        Self::define_sock_accept(ctx.clone(), &mut linker, &mut store);
        Self::define_sock_recv(ctx.clone(), &mut linker, &mut store);
        Self::define_sock_send(ctx.clone(), &mut linker, &mut store);
        Self::define_sock_shutdown(ctx.clone(), &mut linker, &mut store);

        let module: Module = match Module::new(&engine, &wasm_binary.bytes) {
            Ok(module) => module,
            Err(err) => {
                ::syslog::error!("failed to create WASM module: {:?}", err);
                return Err(Error::new(ErrorCode::InvalidExecutableFormat, "failed to create WASM module"));
            },
        };

        let wasm_main: Func = Func::wrap(&mut store, |_caller: Caller<'_, HostState>| {
            ::syslog::trace!("wasm_main");
        });

        if let Err(err) = linker.define("env", "_start", wasm_main) {
            ::syslog::error!("failed to define _start function: {:?}", err);
            return Err(Error::new(ErrorCode::InvalidExecutableFormat, "failed to define _start function"));
        }

        let instance: Instance = match linker.instantiate_and_start(&mut store, &module) {
            Ok(instance) => instance,
            Err(err) => {
                ::syslog::error!("failed to instantiate and start WASM module: {:?}", err);
                return Err(Error::new(ErrorCode::InvalidExecutableFormat, "failed to instantiate and start WASM module"));
            },
        };

        let start_fn: TypedFunc<(), ()> = match instance.get_typed_func::<(), ()>(&store, "_start") {
            Ok(func) => func,
            Err(err) => {
                ::syslog::error!("failed to get _start function: {:?}", err);
                return Err(Error::new(ErrorCode::InvalidExecutableFormat, "failed to get _start function"));
            },
        };

        Ok(Self {
            _ctx: ctx,
            _engine: engine,
            store,
            _linker: linker,
            _wasm_main: wasm_main,
            start_fn,
        })
    }

    pub fn run(&mut self) {
        if let Err(e) = self.start_fn.call(&mut self.store, ()) {
            match e.kind() {
                ErrorKind::TrapCode(code) => {
                    ::syslog::error!("Trap: {:?}", code);
                },
                ErrorKind::I32ExitStatus(status) => {
                    ::syslog::error!("Exit status: {:?}", status);
                },
                e => {
                    ::syslog::error!("Error: {:?}", e);
                },
            }
            ::syslog::error!("Error: {:?}", e);
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
