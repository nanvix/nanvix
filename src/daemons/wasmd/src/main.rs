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
use ::nvx::sys::error::Error;
use ::wasmd::{
    LoadMessage,
    WasmdMessage,
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

#[no_mangle]
fn main() -> Result<(), Error> {
    ::nvx::log!("initializing wasm daemon...");

    let mut wasm_bytes: Vec<u8> = Vec::new();

    let mut nmessages = 0;
    loop {
        match ::nvx::ipc::recv() {
            Ok(message) => match WasmdMessage::from_bytes(message.payload) {
                Ok(message) => {
                    let message = LoadMessage::from_bytes(message.payload);
                    if message.len == 0 {
                        ::nvx::log!("received last message");
                        break;
                    }

                    let message_len = message.len as usize;
                    if wasm_bytes.capacity() < wasm_bytes.len() + message_len {
                        wasm_bytes.reserve(message_len);
                    }
                    nmessages += 1;
                    // ::nvx::log!(
                    //     "received message (len={:?}/{}, nmessages={:?})",
                    //     message_len,
                    //     LoadMessage::PAYLOAD_SIZE,
                    //     nmessages
                    // );
                    wasm_bytes.extend_from_slice(&message.payload[..message_len]);
                },
                Err(e) => {
                    panic!("failed to receive message (error={:?})", e);
                },
            },
            Err(e) => {
                panic!("failed to receive message (error={:?}, nmessage={:?})", e, nmessages);
            },
        }
    }

    wasm_bytes.shrink_to_fit();

    ::nvx::log!("loading wasm file ({} bytes)", wasm_bytes.len());
    let engine: Engine = Engine::default();
    let module = match Module::new(&engine, &wasm_bytes) {
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
