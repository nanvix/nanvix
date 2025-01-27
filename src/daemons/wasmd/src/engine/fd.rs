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
        types::{
            Address,
            Errno,
            Fd,
            FileSize,
            IoVec,
            Pointer,
            Prestat,
            Size,
            Slice,
        },
        WasiCtx,
    },
};
use ::alloc::{
    sync::Arc,
    vec::Vec,
};
use ::core::mem;
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
    pub(super) fn define_fd_advise(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_advise: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, fd: i32, offset: i64, len: i64, advice: i32| -> i32 {
                ::nvx::log!("fd_advise: {fd}, {offset}, {len}, {advice}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_advise", fd_advise)
            .unwrap();
    }

    pub(super) fn define_fd_allocate(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_allocate: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, offset: i64, len: i64| -> i32 {
                ::nvx::log!("fd_allocate: {fd}, {offset}, {len}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_allocate", fd_allocate)
            .unwrap();
    }

    pub(super) fn define_fd_close(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_close: Func = Func::wrap(store, move |_caller: Caller<'_, u32>, fd: i32| -> i32 {
            ::nvx::log!("fd_close(): {:?}", fd);

            // Convert file descriptor.
            let fd: Fd = fd;

            match ctx.fd_close(fd) {
                Ok(()) => Errno::Success.into(),
                Err(e) => e.into(),
            }
        });
        linker
            .define("wasi_snapshot_preview1", "fd_close", fd_close)
            .unwrap();
    }

    pub(super) fn define_fd_datasync(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_datasync: Func = Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32| -> i32 {
            ::nvx::log!("fd_datasync: {fd}");
            Errno::Nosys.into()
        });
        linker
            .define("wasi_snapshot_preview1", "fd_datasync", fd_datasync)
            .unwrap();
    }

    pub(super) fn define_fd_fdstat_get(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_fdstat_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, buf: i32| -> i32 {
                ::nvx::log!("fd_fdstat_get: {fd}, {buf}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_fdstat_get", fd_fdstat_get)
            .unwrap();
    }

    pub(super) fn define_fd_fdstat_set_flags(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_fdstat_set_flags: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, flags: i32| -> i32 {
                ::nvx::log!("fd_fdstat_set_flags: {fd}, {flags}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_fdstat_set_flags", fd_fdstat_set_flags)
            .unwrap();
    }

    pub(super) fn define_fd_fdstat_set_rights(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_fdstat_set_rights: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             fs_rights_base: i64,
             fs_rights_inheriting: i64|
             -> i32 {
                ::nvx::log!("fd_fdstat_set_rights: {fd}, {fs_rights_base}, {fs_rights_inheriting}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_fdstat_set_rights", fd_fdstat_set_rights)
            .unwrap();
    }

    pub(super) fn define_fd_filestat_get(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_filestat_get: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, buf: i32| -> i32 {
                ::nvx::log!("fd_filestat_get: {fd}, {buf}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_filestat_get", fd_filestat_get)
            .unwrap();
    }

    pub(super) fn define_fd_filestat_set_size(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_filestat_set_size: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, size: i64| -> i32 {
                ::nvx::log!("fd_filestat_set_size: {fd}, {size}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_filestat_set_size", fd_filestat_set_size)
            .unwrap();
    }

    pub(super) fn define_fd_filestat_set_times(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_filestat_set_times: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, fd: i32, atim: i64, mtim: i64, fst_flags: i32| -> i32 {
                ::nvx::log!("fd_filestat_set_times: {fd}, {atim}, {mtim}, {fst_flags}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_filestat_set_times", fd_filestat_set_times)
            .unwrap();
    }

    pub(super) fn define_fd_pread(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_pread: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             iovs_ptr: i32,
             iovs_len: i32,
             offset: i64,
             nread_ptr: i32|
             -> i32 {
                ::nvx::log!("fd_pread: {fd}, {iovs_ptr}, {iovs_len}, {offset} {nread_ptr}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_pread", fd_pread)
            .unwrap();
    }

    /// Returns the path of a pre-opened directory.
    pub(super) fn define_fd_prestat_dir_name(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_prestat_dir_name: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  fd: i32,
                  path_buf_offset: i32,
                  path_len: i32|
                  -> i32 {
                ::nvx::log!(
                    "fd_prestat_dir_name(): fd={:?}, path_buf_offset={:?}, path_len={:?}",
                    fd,
                    path_buf_offset,
                    path_len
                );

                let memory: &mut [u8] = Self::get_memory_mut(&mut caller);

                // Convert file descriptor.
                let fd: Fd = fd;

                // Attempt to convert the path buffer offset.
                let path_buf_offset: usize = match path_buf_offset.try_into() {
                    Ok(path_buf_offset) => path_buf_offset,
                    _ => {
                        ::nvx::log!(
                            "fd_prestat_dir_name(): invalid path_buf_offset {:#010x}",
                            path_buf_offset
                        );
                        return Errno::Inval.into();
                    },
                };

                match ctx.fd_prestat_dir_get(fd) {
                    Ok(dirname) => {
                        let dirname_bytes: &[u8] = dirname.as_bytes();

                        // Check if memory is large enough to store the directory name.
                        if memory.len() < path_buf_offset + dirname_bytes.len() {
                            ::nvx::log!(
                                "fd_prestat_dir_name(): buffer too small (size={:?}, \
                                 required={:?})",
                                memory.len(),
                                path_buf_offset + dirname_bytes.len()
                            );
                            return Errno::Inval.into();
                        }

                        dirname_bytes.write_le_bytes(&mut memory[path_buf_offset..]);

                        Errno::Success.into()
                    },
                    Err(e) => e.into(),
                }
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_prestat_dir_name", fd_prestat_dir_name)
            .unwrap();
    }

    /// Returns a description of the given pre-opened file descriptor.
    pub(super) fn define_fd_prestat_get(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_prestat_get: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, HostState>, fd: i32, prestat_offset: i32| -> i32 {
                ::nvx::log!("fd_prestat_get(): fd={:?}, prestat_offset={:?}", fd, prestat_offset);

                let memory: &mut [u8] = Self::get_memory_mut(&mut caller);

                // Convert file descriptor.
                let fd: Fd = fd;

                // Attempt to convert pre-stat offset.
                let prestat_offset: usize = match prestat_offset.try_into() {
                    Ok(prestat_offset) => prestat_offset,
                    Err(_) => return Errno::Fault.into(),
                };

                // Check if memory is large enough to store pre-stat.
                if memory.len() < prestat_offset + mem::size_of::<Prestat>() {
                    ::nvx::log!(
                        "fd_prestat_get(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        prestat_offset + mem::size_of::<Prestat>()
                    );
                    return Errno::Inval.into();
                }

                match ctx.fd_prestat_get(fd) {
                    Ok(prestat) => {
                        prestat.write_le_bytes(&mut memory[prestat_offset..]);
                        Errno::Success.into()
                    },
                    Err(e) => e.into(),
                }
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_prestat_get", fd_prestat_get)
            .expect("should be able to add symbol to linker");
    }

    pub(super) fn define_fd_pwrite(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_pwrite: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             iovs_ptr: i32,
             iovs_len: i32,
             offset: i64,
             nwritten_ptr: i32|
             -> i32 {
                ::nvx::log!("fd_pwrite: {fd}, {iovs_ptr}, {iovs_len}, {offset}, {nwritten_ptr}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_pwrite", fd_pwrite)
            .unwrap();
    }

    pub(super) fn define_fd_read(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_read: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, u32>,
                  fd: i32,
                  iovs_buf: i32,
                  iovs_len: i32,
                  nread_ptr: i32|
                  -> i32 {
                ::nvx::log!(
                    "fd_read(): fd={:?}, iovs_buf={:?}, iovs_len={:?}, nread_ptr={:?}",
                    fd,
                    iovs_buf,
                    iovs_len,
                    nread_ptr
                );

                let memory: &mut [u8] = Self::get_memory_mut(&mut caller);

                // Convert file descriptor.
                let fd: Fd = fd;

                // Attempt to convert I/O vector base pointer.
                let iovs_buf: Pointer<IoVec> =
                    match Pointer::<IoVec>::new(Address::new(iovs_buf as u32)) {
                        Ok(iov_buf) => iov_buf,
                        Err(_) => {
                            ::nvx::log!("fd_read(): invalid iov_buf {:#010x}", iovs_buf);
                            return Errno::Inval.into();
                        },
                    };

                // Attempt to convert I/O vector length.
                let iovs_len: Size = match iovs_len.try_into() {
                    Ok(iovs_len) => iovs_len,
                    Err(_) => {
                        ::nvx::log!("fd_read(): invalid iovs_len {:#010x}", iovs_len);
                        return Errno::Inval.into();
                    },
                };

                let iovecs: Vec<IoVec> = {
                    let iovecs: Slice<'_, IoVec> =
                        Slice::<IoVec>::for_raw_parts(memory, iovs_buf, iovs_len);
                    match iovecs.as_ref() {
                        Ok(iovecs) => iovecs.to_vec(),
                        Err(_) => {
                            ::nvx::log!("fd_read(): failed to get slice from memory");
                            return Errno::Inval.into();
                        },
                    }
                };

                // Attempt to convert pointer to number of bytes read.
                let nread_ptr: usize = match nread_ptr.try_into() {
                    Ok(nread_ptr) => nread_ptr,
                    Err(_) => {
                        ::nvx::log!("fd_read(): invalid nread_ptr {:#010x}", nread_ptr);
                        return Errno::Inval.into();
                    },
                };

                // Check if memory is large enough to store the number of bytes read.
                if memory.len() < nread_ptr + mem::size_of::<Size>() {
                    ::nvx::log!(
                        "fd_read(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        nread_ptr as usize + mem::size_of::<Size>()
                    );
                    return Errno::Inval.into();
                }

                match ctx.fd_read(memory, fd, &iovecs) {
                    Ok(nread) => {
                        // Write the number of bytes read to nread_ptr
                        nread.write_le_bytes(&mut memory[nread_ptr..]);
                        Errno::Success.into()
                    },
                    Err(e) => e.into(),
                }
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_read", fd_read)
            .unwrap();
    }

    pub(super) fn define_fd_readdir(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_readdir: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             buf: i32,
             buf_len: i32,
             cookie: i64,
             buf_used: i32|
             -> i32 {
                ::nvx::log!("fd_readdir: {fd}, {buf}, {buf_len}, {cookie}, {buf_used}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_readdir", fd_readdir)
            .unwrap();
    }

    pub(super) fn define_fd_renumber(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let renumber: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, to: i32| -> i32 {
                ::nvx::log!("fd_renumber: {fd}, {to}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_renumber", renumber)
            .unwrap();
    }

    pub(super) fn define_fd_seek(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_seek: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>, fd: i32, offset: i64, whence: i32, newoffset: i32| -> i32 {
                ::nvx::log!("fd_seek: {fd}, {offset}, {whence}, {newoffset}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_seek", fd_seek)
            .unwrap();
    }

    pub(super) fn define_fd_sync(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_sync: Func = Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32| -> i32 {
            ::nvx::log!("fd_sync: {fd}");
            Errno::Nosys.into()
        });
        linker
            .define("wasi_snapshot_preview1", "fd_sync", fd_sync)
            .unwrap();
    }

    pub(super) fn define_fd_tell(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_tell: Func =
            Func::wrap(store, move |mut caller: Caller<'_, u32>, fd: i32, newoffset: i32| -> i32 {
                ::nvx::log!("fd_tell(): fd={:?}, newoffset={:?}", fd, newoffset);

                let memory: &mut [u8] = Self::get_memory_mut(&mut caller);

                // Convert file descriptor.
                let fd: Fd = fd;

                // Attempt to convert pointer to new offset.
                let newoffset: usize = match newoffset.try_into() {
                    Ok(newoffset) => newoffset,
                    Err(_) => return Errno::Inval.into(),
                };

                // Check if memory is large enough to store the new offset.
                if memory.len() < newoffset + mem::size_of::<FileSize>() {
                    ::nvx::log!(
                        "fd_tell(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        newoffset + mem::size_of::<FileSize>()
                    );
                    return Errno::Inval.into();
                }

                match ctx.fd_tell(fd) {
                    Ok(offset) => {
                        // Attempt to convert offset to FileSize.
                        let offset: FileSize = match offset.try_into() {
                            Ok(offset) => offset,
                            Err(_) => {
                                ::nvx::log!("fd_tell(): failed to convert offset to FileSize");
                                return Errno::TooBig.into();
                            },
                        };

                        offset.write_le_bytes(&mut memory[newoffset..]);
                        Errno::Success.into()
                    },
                    Err(e) => e.into(),
                }
            });
        linker
            .define("wasi_snapshot_preview1", "fd_tell", fd_tell)
            .unwrap();
    }

    pub(super) fn define_fd_write(
        ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_write: Func = Func::wrap(
            store,
            move |mut caller: Caller<'_, HostState>,
                  fd: i32,
                  iovs_ptr: i32,
                  iovs_len: i32,
                  nwritten_ptr: i32|
                  -> i32 {
                ::nvx::log!(
                    "fd_write(): fd={:?}, iovs_ptr={:?}, iovs_len={:?}, nwritten_ptr={:?}",
                    fd,
                    iovs_ptr,
                    iovs_len,
                    nwritten_ptr
                );

                let memory: &mut [u8] = Self::get_memory_mut(&mut caller);

                // Convert file descriptor.
                let fd: Fd = fd;

                // Attempt to convert I/O vector base pointer.
                let iovs_ptr: Pointer<IoVec> =
                    match Pointer::<IoVec>::new(Address::new(iovs_ptr as u32)) {
                        Ok(iovs_ptr) => iovs_ptr,
                        Err(_) => {
                            ::nvx::log!("fd_write(): invalid iovs_ptr {:#010x}", iovs_ptr);
                            return Errno::Inval.into();
                        },
                    };

                // Attempt to convert I/O vector length.
                let iovs_len: Size = match iovs_len.try_into() {
                    Ok(iovs_len) => iovs_len,
                    Err(_) => {
                        ::nvx::log!("fd_write(): invalid iovs_len {:#010x}", iovs_len);
                        return Errno::Inval.into();
                    },
                };

                let iovecs: Slice<'_, IoVec> =
                    Slice::<IoVec>::for_raw_parts(memory, iovs_ptr, iovs_len);
                let iovecs = match iovecs.as_ref() {
                    Ok(iovecs) => iovecs,
                    Err(_) => {
                        ::nvx::log!("fd_write(): failed to get slice from memory");
                        return Errno::Inval.into();
                    },
                };

                // Attempt to convert pointer to number of bytes written.
                let nwritten_ptr: usize = match nwritten_ptr.try_into() {
                    Ok(nwritten_ptr) => nwritten_ptr,
                    Err(_) => {
                        ::nvx::log!("fd_write(): invalid nwritten_ptr {:#010x}", nwritten_ptr);
                        return Errno::Inval.into();
                    },
                };

                // Check if memory is large enough to store the number of bytes written.
                if memory.len() < nwritten_ptr + mem::size_of::<Size>() {
                    ::nvx::log!(
                        "fd_write(): buffer too small (size={:?}, required={:?})",
                        memory.len(),
                        nwritten_ptr as usize + mem::size_of::<Size>()
                    );
                    return Errno::Inval.into();
                }

                match ctx.fd_write(memory, fd, iovecs) {
                    Ok(nwritten) => {
                        // Write the number of bytes written to nwritten_ptr
                        nwritten.write_le_bytes(&mut memory[nwritten_ptr..]);
                        Errno::Success.into()
                    },
                    Err(e) => e.into(),
                }
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_write", fd_write)
            .unwrap();
    }

    pub(super) fn define_poll_oneoff(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let poll_oneoff: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             in_: i32,
             out: i32,
             nsubscriptions: i32,
             nevents: i32|
             -> i32 {
                ::nvx::log!("poll_oneoff: {in_}, {out}, {nsubscriptions}, {nevents}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "poll_oneoff", poll_oneoff)
            .unwrap();
    }
}
