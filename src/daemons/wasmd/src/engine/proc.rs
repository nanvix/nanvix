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
    pub(super) fn define_proc_exit(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let proc_exit: Func = Func::wrap(store, |_caller: Caller<'_, u32>, arg0: i32| {
            ::nvx::trace!("proc_exit: {arg0}");
        });

        linker
            .define("wasi_snapshot_preview1", "proc_exit", proc_exit)
            .unwrap();
    }

    pub(super) fn define_proc_raise(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let proc_raise: Func = Func::wrap(store, |_caller: Caller<'_, u32>, sig: i32| -> i32 {
            ::nvx::trace!("proc_raise: {sig}");
            Errno::Nosys.into()
        });
        linker
            .define("wasi_snapshot_preview1", "proc_raise", proc_raise)
            .unwrap();
    }

    pub(super) fn define_sched_yield(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let sched_yield: Func = Func::wrap(store, |_caller: Caller<'_, u32>| -> i32 {
            ::nvx::trace!("sched_yield");
            Errno::Nosys.into()
        });

        linker
            .define("wasi_snapshot_preview1", "sched_yield", sched_yield)
            .unwrap();
    }

    pub(super) fn define_random_get(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let random_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, buf: i32, buf_len: i32| -> i32 {
                ::nvx::trace!("random_get: {buf}, {buf_len}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "random_get", random_get)
            .unwrap();
    }
}
