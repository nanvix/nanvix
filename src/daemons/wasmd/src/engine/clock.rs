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
    wasi::types::Errno,
};
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
    pub(super) fn define_clock_res_get(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let clock_res_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, id: i32, offset: i32| -> i32 {
                ::syslog::trace!("clock_res_get: {id}, {offset}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "clock_res_get", clock_res_get)
            .unwrap();
    }

    pub(super) fn define_clock_time_get(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let clock_time_get: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, id: i32, precision: i64, offset: i32| -> i32 {
                ::syslog::trace!("clock_time_get: {id}, {precision}, {offset}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "clock_time_get", clock_time_get)
            .unwrap();
    }
}
