// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![forbid(clippy::all)]
#![no_std]
#![no_main]

extern crate alloc;

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::vec::Vec;
use ::core::mem;
use ::nvx::sys::error::Error;
use ::posix::{
    ffi::c_int,
    netinet::in_::{
        in_addr,
        sockaddr_in,
    },
    sys::{
        self,
        socket::{
            self,
            sockaddr,
            socklen_t,
        },
        types::{
            size_t,
            ssize_t,
        },
    },
    unistd,
};
use ::wasmi::{
    Caller,
    Engine,
    Func,
    Linker,
    Module,
    Store,
};

#[no_mangle]
fn fminf(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmax(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmin(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmaxf(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

#[no_mangle]
fn fmod(a: f64, b: f64) -> f64 {
    a % b
}

#[no_mangle]
fn fmodf(a: f32, b: f32) -> f32 {
    a % b
}

// const WASM_BYTES: &[u8] = include_bytes!("../bin/hello.wasm");

struct WasmBinary {
    bytes: Vec<u8>,
}

impl WasmBinary {
    pub fn from_network() -> Self {
        let sockfd: c_int = match socket::socket(socket::AF_INET as c_int, socket::SOCK_STREAM, 0) {
            sockfd if sockfd >= 0 => sockfd,
            errno => {
                panic!("failed to create socket (errno={})", errno);
            },
        };

        // Bind socket to address to 127.0.0.1:8080.
        let sockaddr_in: sockaddr_in = sockaddr_in {
            sin_family: sys::socket::AF_INET,
            sin_port: u16::to_be(8080),
            sin_addr: in_addr {
                s_addr: u32::from_be_bytes([127, 0, 0, 1]).to_be(),
            },
            sin_zero: [0; 8],
        };

        match sys::socket::bind(
            sockfd,
            unsafe {
                mem::transmute::<&posix::netinet::in_::sockaddr_in, &posix::sys::socket::sockaddr>(
                    &sockaddr_in,
                )
            },
            core::mem::size_of::<sys::socket::sockaddr>() as socklen_t,
        ) {
            0 => {
                ::nvx::log!("bound socket to address");
            },
            errno => {
                panic!("failed to bind socket to address: {:?}", errno);
            },
        }

        // Listen for connections on socket.
        match sys::socket::listen(sockfd, 0) {
            0 => {
                ::nvx::log!("listening for connections on socket");
            },
            errno => {
                panic!("failed to listen for connections on socket: {:?}", errno);
            },
        }

        // Accept connection on socket.
        let mut address: sockaddr = unsafe { core::mem::zeroed() };
        let mut address_len: socklen_t = 0;
        let connfd: i32 = match sys::socket::accept(sockfd, &mut address, &mut address_len) {
            connfd if connfd >= 0 => {
                ::nvx::log!("accepted connection on socket with fd {}", connfd);
                connfd
            },
            errno => {
                panic!("failed to accept connection on socket: {:?}", errno);
            },
        };

        // Read payload size.
        let mut payload_buffer: [u8; core::mem::size_of::<u32>()] =
            [0; core::mem::size_of::<u32>()];
        let payload_size = match socket::recv(
            connfd,
            &mut payload_buffer as *mut _ as *mut u8,
            payload_buffer.len() as size_t,
            0,
        ) {
            n if n == core::mem::size_of::<u32>() as ssize_t => u32::from_le_bytes(payload_buffer),
            errno => {
                panic!("failed to receive payload size: {:?}", errno);
            },
        };
        ::nvx::log!("received payload size: {}", payload_size);

        // Read payload.
        let mut wasm_bytes: Vec<u8> = alloc::vec![0; payload_size as usize];
        match socket::recv(connfd, wasm_bytes.as_mut_ptr(), payload_size as size_t, 0) {
            n if n == payload_size as ssize_t => {
                ::nvx::log!("received payload");
            },
            errno => {
                panic!("failed to receive payload: {:?}", errno);
            },
        }

        // Close connection.
        match unistd::close(connfd) {
            0 => {
                ::nvx::log!("closed connection");
            },
            errno => {
                panic!("failed to close connection: {:?}", errno);
            },
        }

        wasm_bytes.shrink_to_fit();
        ::nvx::log!("loading wasm file ({} bytes)", wasm_bytes.len());

        Self { bytes: wasm_bytes }
    }
}

#[no_mangle]
fn main() -> Result<(), Error> {
    ::nvx::log!("initializing wasm daemon...");

    let wasm_binary = WasmBinary::from_network();

    let engine: Engine = Engine::default();
    let module = match Module::new(&engine, &wasm_binary.bytes) {
        Ok(module) => module,
        Err(err) => {
            panic!("Error: {:?}", err);
        },
    };

    ::nvx::log!("wasm file loaded");

    // All Wasm objects operate within the context of a `Store`.
    // Each `Store` has a type parameter to store host-specific data,
    // which in this case we are using `42` for.
    type HostState = u32;
    let mut store = Store::new(&engine, 42);

    let wasm_main: Func = Func::wrap(&mut store, |_caller: Caller<'_, HostState>| {
        ::nvx::log!("wasm_main");
    });

    let sched_yield: Func = Func::wrap(&mut store, |_caller: Caller<'_, u32>| -> i32 {
        ::nvx::log!("sched_yield");
        0
    });

    let proc_exit: Func = Func::wrap(&mut store, |_caller: Caller<'_, u32>, arg0: i32| {
        ::nvx::log!("proc_exit: {arg0}");
    });

    let args_get: Func =
        Func::wrap(&mut store, |_caller: Caller<'_, u32>, arg0: i32, arg1: i32| -> i32 {
            ::nvx::log!("args_get: {arg0}, {arg1}");
            0
        });

    let args_sizes_get: Func =
        Func::wrap(&mut store, |_caller: Caller<'_, u32>, arg0: i32, arg1: i32| -> i32 {
            ::nvx::log!("args_sizes_get: {arg0}, {arg1}");
            0
        });

    // clock_time_get
    let clock_time_get: Func = Func::wrap(
        &mut store,
        |_caller: Caller<'_, u32>, id: i32, precision: i64, offset: i32| -> i32 {
            ::nvx::log!("clock_time_get: {id}, {precision}, {offset}");
            0
        },
    );

    // environ_get
    let environ_get: Func =
        Func::wrap(&mut store, |_caller: Caller<'_, u32>, environ: i32, environ_buf: i32| -> i32 {
            ::nvx::log!("environ_get: {environ}, {environ_buf}");
            0
        });

    // environ_sizes_get
    let environ_sizes_get: Func = Func::wrap(
        &mut store,
        |_caller: Caller<'_, u32>, environ_count: i32, environ_buf_size: i32| -> i32 {
            ::nvx::log!("environ_sizes_get: {environ_count}, {environ_buf_size}");
            0
        },
    );

    let fd_write: Func = Func::wrap(
        &mut store,
        |mut caller: Caller<'_, HostState>,
         fd: i32,
         iovs_ptr: i32,
         iovs_len: i32,
         nwritten_ptr: i32| {
            // Ensure fd is 1 (stdout)
            if fd != 1 {
                return -1;
            }

            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data_mut(&mut caller);

            // Read the iovec array
            let mut total_written = 0;
            for i in 0..iovs_len {
                let iovec_base = iovs_ptr as usize + i as usize * 8;
                let ptr = u32::from_le_bytes(data[iovec_base..iovec_base + 4].try_into().unwrap())
                    as usize;
                let len =
                    u32::from_le_bytes(data[iovec_base + 4..iovec_base + 8].try_into().unwrap())
                        as usize;

                let msg = core::str::from_utf8(&data[ptr..ptr + len]).expect("Invalid utf8");
                ::nvx::log!("{msg}");
                total_written += len;
            }

            // Write the number of bytes written to nwritten_ptr
            let nwritten_bytes = (total_written as u32).to_le_bytes();
            let nwritten_ptr = nwritten_ptr as usize;
            data[nwritten_ptr..nwritten_ptr + 4].copy_from_slice(&nwritten_bytes);

            0
        },
    );

    // random_get
    let random_get: Func =
        Func::wrap(&mut store, |_caller: Caller<'_, u32>, buf: i32, buf_len: i32| -> i32 {
            ::nvx::log!("random_get: {buf}, {buf_len}");
            0
        });

    // poll_oneoff
    let poll_oneoff: Func = Func::wrap(
        &mut store,
        |_caller: Caller<'_, u32>, in_: i32, out: i32, nsubscriptions: i32, nevents: i32| -> i32 {
            ::nvx::log!("poll_oneoff: {in_}, {out}, {nsubscriptions}, {nevents}");
            0
        },
    );

    // fd_close
    let fd_close: Func = Func::wrap(&mut store, |_caller: Caller<'_, u32>, fd: i32| -> i32 {
        ::nvx::log!("fd_close: {fd}");
        0
    });

    // fd_fdstat_get
    let fd_fdstat_get: Func =
        Func::wrap(&mut store, |_caller: Caller<'_, u32>, fd: i32, buf: i32| -> i32 {
            ::nvx::log!("fd_fdstat_get: {fd}, {buf}");
            0
        });

    // fd_fdsat_set_flags
    let fd_fdstat_set_flags: Func =
        Func::wrap(&mut store, |_caller: Caller<'_, u32>, fd: i32, flags: i32| -> i32 {
            ::nvx::log!("fd_fdstat_set_flags: {fd}, {flags}");
            0
        });

    // fd_prestat_get
    let fd_prestat_get: Func =
        Func::wrap(&mut store, |_caller: Caller<'_, u32>, fd: i32, buf: i32| -> i32 {
            ::nvx::log!("fd_prestat_get: {fd}, {buf}");
            0
        });

    // fd_prestat_dir_name
    let fd_prestat_dir_name: Func = Func::wrap(
        &mut store,
        |_caller: Caller<'_, u32>, fd: i32, path: i32, path_len: i32| -> i32 {
            ::nvx::log!("fd_prestat_dir_name: {fd}, {path}, {path_len}");
            0
        },
    );

    // In order to create Wasm module instances and link their imports
    // and exports we require a `Linker`.
    let mut linker = <Linker<HostState>>::new(&engine);
    linker
        .define("wasi_snapshot_preview1", "sched_yield", sched_yield)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "proc_exit", proc_exit)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "args_get", args_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "args_sizes_get", args_sizes_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "clock_time_get", clock_time_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "environ_get", environ_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "environ_sizes_get", environ_sizes_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "fd_write", fd_write)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "random_get", random_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "poll_oneoff", poll_oneoff)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "fd_close", fd_close)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "fd_fdstat_get", fd_fdstat_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "fd_fdstat_set_flags", fd_fdstat_set_flags)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "fd_prestat_get", fd_prestat_get)
        .unwrap();

    linker
        .define("wasi_snapshot_preview1", "fd_prestat_dir_name", fd_prestat_dir_name)
        .unwrap();

    // linker.define("env", "print", print).unwrap();
    // Instantiation of a Wasm module requires defining its imports and then
    // afterwards we can fetch exports by name, as well as asserting the
    // type signature of the function with `get_typed_func`.
    //
    // Also before using an instance created this way we need to start it.
    linker.define("env", "_start", wasm_main).unwrap();
    let instance = linker
        .instantiate(&mut store, &module)
        .unwrap()
        .start(&mut store)
        .unwrap();
    let hello = instance.get_typed_func::<(), ()>(&store, "_start").unwrap();

    // And finally we can call the wasm!
    hello.call(&mut store, ()).unwrap();

    ::nvx::log!("shutting down wasm daemon...");

    Ok(())
}
