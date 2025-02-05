// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    memory::WriteBytes,
    wasi::{
        types::Errno,
        Fd,
        FdFlags,
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

use super::{
    HostState,
    WasmEngine,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl WasmEngine {
    pub(super) fn define_sock_accept(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let sock_accept: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  sockfd: i32,
                  fdflags: i32,
                  sockfd_offset: i32|
                  -> i32 {
                ::nvx::log!(
                    "sock_accept(): sockfd={:?}, fdflags={:?}, sockfd_offset={:?}",
                    sockfd,
                    fdflags,
                    sockfd_offset,
                );

                let memory: &mut [u8] = WasmEngine::get_memory_mut(&mut caller);

                // Convert socket file descriptor.
                let sockfd: Fd = sockfd as Fd;

                // Reconstruct file descriptor flags.
                let fdflags: FdFlags = fdflags.into();

                // Check for invalid/unsupported file descriptor flags.
                if fdflags.append {
                    ::nvx::log!("sock_accept(): append to a file is invalid");
                    return Errno::Inval.into();
                };
                if fdflags.sync {
                    ::nvx::log!("sock_accept(): sync to a file is invalid");
                    return Errno::Inval.into();
                };
                if fdflags.dsync {
                    ::nvx::log!("sock_accept(): dsync to a file is invalid");
                    return Errno::Inval.into();
                };
                if fdflags.rsync {
                    ::nvx::log!("sock_accept(): rsync to a file is invalid");
                    return Errno::Inval.into();
                };
                if fdflags.nonblock {
                    ::nvx::log!("sock_accept(): nonblock to a file is not supported");
                    return Errno::Notsup.into();
                };

                // Attempt to convert socket file descriptor offset.
                let sockfd_offset: usize = match sockfd_offset.try_into() {
                    Ok(offset) => offset,
                    Err(_) => {
                        ::nvx::log!(
                            "sock_accept(): invalid socket file descriptor offset ({:?})",
                            sockfd_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                // Check if memory is too small to store the socket file descriptor.
                if sockfd_offset + ::core::mem::size_of::<Fd>() > memory.len() {
                    ::nvx::log!(
                        "sock_accept(): memory is too small to store the socket file descriptor"
                    );
                    return Errno::Inval.into();
                }

                // Accept connection.
                match ctx.sock_accept(sockfd, fdflags) {
                    Ok(fd) => {
                        // Store file descriptor.
                        fd.write_le_bytes(&mut memory[sockfd_offset..]);
                        Errno::Success.into()
                    },
                    Err(errno) => errno.into(),
                }
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
