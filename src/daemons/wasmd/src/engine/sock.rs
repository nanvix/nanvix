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
    memory::WriteBytes,
    wasi::{
        types::Errno,
        Address,
        Fd,
        FdFlags,
        IoVec,
        Pointer,
        RiFlags,
        SdFlags,
        SiFlags,
        Size,
        Slice,
        WasiCtx,
    },
};
use ::alloc::{
    sync::Arc,
    vec::Vec,
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

                // Attempt to accept a new connection.
                match ctx.sock_accept(sockfd, fdflags) {
                    Ok(connfd) => {
                        // Store new file descriptor of the accepted connection in guest memory.
                        connfd.write_le_bytes(&mut memory[sockfd_offset..]);
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

    pub(super) fn define_sock_recv(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let sock_recv: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  connfd: i32,
                  iov_buf: i32,
                  iov_buf_len: i32,
                  riflags: i32,
                  nrecv_ptr: i32,
                  roflags_ptr: i32|
                  -> i32 {
                ::nvx::log!(
                    "sock_recv(): connfd={:?}, iov_buf={:?}, iov_buf_len={:?}, riflags={:?}, \
                     nrecv_ptr={:?}, roflags_ptr={:?}",
                    connfd,
                    iov_buf,
                    iov_buf_len,
                    riflags,
                    nrecv_ptr,
                    roflags_ptr,
                );

                let memory: &mut [u8] = Self::get_memory_mut(&mut caller);

                // Convert socket file descriptor.
                let connfd: Fd = connfd as Fd;

                // Attempt to convert receive flags.
                let riflags: RiFlags = match riflags.try_into() {
                    Ok(riflags) => riflags,
                    Err(_) => {
                        ::nvx::log!("sock_recv(): invalid riflags {:#010x}", riflags);
                        return Errno::Inval.into();
                    },
                };

                // Check for unsupported receive flags.
                if riflags.peek {
                    ::nvx::log!("sock_recv(): peek is not supported");
                    return Errno::Notsup.into();
                }
                if riflags.oob {
                    ::nvx::log!("sock_recv(): oob is not supported");
                    return Errno::Notsup.into();
                }

                // Attempt to convert I/O vector base pointer.
                let iovs_buf: Pointer<IoVec> =
                    match Pointer::<IoVec>::new(Address::new(iov_buf as u32)) {
                        Ok(iov_buf) => iov_buf,
                        Err(_) => {
                            ::nvx::log!("sock_recv(): invalid iov_buf {:#010x}", iov_buf);
                            return Errno::Inval.into();
                        },
                    };

                // Attempt to convert I/O vector length.
                let iovs_len: Size = match iov_buf_len.try_into() {
                    Ok(iovs_len) => iovs_len,
                    Err(_) => {
                        ::nvx::log!("sock_recv(): invalid iovs_len {:#010x}", iov_buf_len);
                        return Errno::Inval.into();
                    },
                };

                let iovecs: Vec<IoVec> = {
                    let iovecs: Slice<'_, IoVec> =
                        Slice::<IoVec>::for_raw_parts(memory, iovs_buf, iovs_len);
                    match iovecs.as_ref() {
                        Ok(iovecs) => iovecs.to_vec(),
                        Err(_) => {
                            ::nvx::log!("sock_recv(): failed to get slice from memory");
                            return Errno::Inval.into();
                        },
                    }
                };

                // Attempt to convert pointer to number of bytes received.
                let nrecv_ptr: usize = match nrecv_ptr.try_into() {
                    Ok(nrecv_ptr) => nrecv_ptr,
                    Err(_) => {
                        ::nvx::log!("sock_recv(): invalid nrecv_ptr {:#010x}", nrecv_ptr);
                        return Errno::Inval.into();
                    },
                };

                // Check if memory is too small to store the number of bytes received.
                if nrecv_ptr + size_of::<Size>() > memory.len() {
                    ::nvx::log!(
                        "sock_recv(): memory is too small to store the number of bytes received"
                    );
                    return Errno::Inval.into();
                }

                // Attempt to convert pointer to receive flags.
                let roflags_ptr: usize = match roflags_ptr.try_into() {
                    Ok(roflags_ptr) => roflags_ptr,
                    Err(_) => {
                        ::nvx::log!("sock_recv(): invalid roflags_ptr {:#010x}", roflags_ptr);
                        return Errno::Inval.into();
                    },
                };

                // Check if memory is too small to store the receive flags.
                if roflags_ptr + size_of::<Size>() > memory.len() {
                    ::nvx::log!("sock_recv(): memory is too small to store the receive flags");
                    return Errno::Inval.into();
                }

                match ctx.sock_recv(memory, connfd, &iovecs, riflags) {
                    Ok((nrecv, roflags)) => {
                        nrecv.write_le_bytes(&mut memory[nrecv_ptr..]);
                        roflags.write_le_bytes(&mut memory[roflags_ptr..]);
                        Errno::Success.into()
                    },
                    Err(errno) => errno.into(),
                }
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_recv", sock_recv)
            .unwrap();
    }

    pub(super) fn define_sock_send(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let sock_send: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  sockfd: i32,
                  iov_buf: i32,
                  iov_buf_len: i32,
                  siflags: i32,
                  nsent_ptr: i32|
                  -> i32 {
                ::nvx::log!(
                    "sock_send(): sockfd={:?}, iov_buf={:?}, iov_buf_len={:?}, siflags={:?}, \
                     nsent_ptr={:?}",
                    sockfd,
                    iov_buf,
                    iov_buf_len,
                    siflags,
                    nsent_ptr,
                );

                let memory: &mut [u8] = Self::get_memory_mut(&mut caller);

                // Convert socket file descriptor.
                let sockfd: Fd = sockfd as Fd;

                // Attempt to convert I/O vector base pointer.
                let iovs_buf: Pointer<IoVec> =
                    match Pointer::<IoVec>::new(Address::new(iov_buf as u32)) {
                        Ok(iov_buf) => iov_buf,
                        Err(_) => {
                            ::nvx::log!("sock_send(): invalid iov_buf {:#010x}", iov_buf);
                            return Errno::Inval.into();
                        },
                    };

                // Attempt to convert I/O vector length.
                let iovs_len: Size = match iov_buf_len.try_into() {
                    Ok(iovs_len) => iovs_len,
                    Err(_) => {
                        ::nvx::log!("sock_send(): invalid iovs_len {:#010x}", iov_buf_len);
                        return Errno::Inval.into();
                    },
                };

                let iovecs: Vec<IoVec> = {
                    let iovecs: Slice<'_, IoVec> =
                        Slice::<IoVec>::for_raw_parts(memory, iovs_buf, iovs_len);
                    match iovecs.as_ref() {
                        Ok(iovecs) => iovecs.to_vec(),
                        Err(_) => {
                            ::nvx::log!("sock_send(): failed to get slice from memory");
                            return Errno::Inval.into();
                        },
                    }
                };

                // Attempt to convert send flags.
                let siflags: SiFlags = match siflags.try_into() {
                    Ok(siflags) => siflags,
                    Err(_) => {
                        ::nvx::log!("sock_send(): invalid siflags {:#010x}", siflags);
                        return Errno::Inval.into();
                    },
                };

                // Check for unsupported send flags.
                if siflags.zero {
                    ::nvx::log!("sock_send(): siflags must be zero");
                    return Errno::Notsup.into();
                }

                // Attempt to convert pointer to number of bytes sent.
                let nsent_ptr: usize = match nsent_ptr.try_into() {
                    Ok(nsent_ptr) => nsent_ptr,
                    Err(_) => {
                        ::nvx::log!("sock_send(): invalid nsent_ptr {:#010x}", nsent_ptr);
                        return Errno::Inval.into();
                    },
                };

                // Check if memory is too small to store the number of bytes sent.
                if nsent_ptr + size_of::<Size>() > memory.len() {
                    ::nvx::log!(
                        "sock_send(): memory is too small to store the number of bytes sent"
                    );
                    return Errno::Inval.into();
                }

                match ctx.sock_send(memory, sockfd, &iovecs, siflags) {
                    Ok(nsent) => {
                        nsent.write_le_bytes(&mut memory[nsent_ptr..]);
                        Errno::Success.into()
                    },
                    Err(errno) => errno.into(),
                }
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_send", sock_send)
            .unwrap();
    }

    pub(super) fn define_sock_shutdown(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let sock_shutdown: Func =
            Func::wrap(store, move |_caller: Caller<'_, u32>, sockfd: i32, how: i32| -> i32 {
                ::nvx::log!("sock_shutdown(): sockfd={:?}, how={:?}", sockfd, how);

                // Convert socket file descriptor.
                let sockfd: Fd = sockfd as Fd;

                // Attempt to convert shutdown flags.
                let how: SdFlags = match how.try_into() {
                    Ok(how) => how,
                    Err(_) => {
                        ::nvx::log!("sock_shutdown(): invalid how {:#010x}", how);
                        return Errno::Inval.into();
                    },
                };

                match ctx.sock_shutdown(sockfd, how) {
                    Ok(()) => Errno::Success.into(),
                    Err(errno) => errno.into(),
                }
            });
        linker
            .define("wasi_snapshot_preview1", "sock_shutdown", sock_shutdown)
            .unwrap();
    }
}
