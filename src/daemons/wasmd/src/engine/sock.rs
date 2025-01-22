// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wasi::types::Errno;
use ::wasmi::{
    Caller,
    Func,
    Linker,
    Store,
};

use super::{
    HostState,
    WasmEngine,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl WasmEngine {
    pub(super) fn define_sock_accept(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
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

    pub(super) fn define_sock_recv(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
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

    pub(super) fn define_sock_send(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
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

    pub(super) fn define_sock_shutdown(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let sock_shutdown: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, sockfd: i32, how: i32| -> i32 {
                ::nvx::log!("sock_shutdown: {sockfd}, {how}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "sock_shutdown", sock_shutdown)
            .unwrap();
    }
}
