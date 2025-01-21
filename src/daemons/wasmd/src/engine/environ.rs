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
    wasi::{
        types::Errno,
        WasiCtx,
    },
};
use ::alloc::sync::Arc;
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
    pub(super) fn define_args_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let args_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, arg0: i32, arg1: i32| -> i32 {
                ::nvx::log!("args_get: {arg0}, {arg1}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "args_get", args_get)
            .unwrap();
    }

    pub(super) fn define_args_sizes_get(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let args_sizes_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, arg0: i32, arg1: i32| -> i32 {
                ::nvx::log!("args_sizes_get: {arg0}, {arg1}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "args_sizes_get", args_sizes_get)
            .unwrap();
    }

    /// Read environment variables data.
    pub(super) fn define_environ_get(
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
    pub(super) fn define_environ_sizes_get(
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
}
