// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    engine::WasmEngine,
    wasi::{
        types::Errno,
        WasiCtx,
    },
};
use ::alloc::sync::Arc;
use ::posix::unistd;
use ::wasmi::{
    Caller,
    Func,
    Linker,
    Store,
};

use crate::engine::HostState;

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

    pub(super) fn define_fd_close(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_close: Func = Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32| -> i32 {
            ::nvx::log!("fd_close: {fd}");
            Errno::Nosys.into()
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
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_prestat_dir_name: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_buf_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "fd_prestat_dir_name: fd={:?}, path_buf_offset={:?}, path_len={:?}",
                    fd,
                    path_buf_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_prestat_dir_name", fd_prestat_dir_name)
            .unwrap();
    }

    /// Returns a description of a pre-opened capability.
    pub(super) fn define_fd_prestat_get(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let fd_prestat_get: Func =
            Func::wrap(store, move |_caller: Caller<'_, HostState>, fd: i32, offset: i32| -> i32 {
                ::nvx::log!("fd_prestat_get(): fd={:?}, buf={:#010x}", fd, offset);
                Errno::Nosys.into()
            });
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

    pub(super) fn define_fd_read(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_read: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             iov_buf: i32,
             iov_buf_len: i32,
             offset: i32|
             -> i32 {
                ::nvx::log!("fd_read: {fd}, {iov_buf}, {iov_buf_len}, {offset}");
                Errno::Nosys.into()
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

    pub(super) fn define_fd_tell(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_tell: Func =
            Func::wrap(store, |_caller: Caller<'_, u32>, fd: i32, newoffset: i32| -> i32 {
                ::nvx::log!("fd_tell: {fd}, {newoffset}");
                Errno::Nosys.into()
            });
        linker
            .define("wasi_snapshot_preview1", "fd_tell", fd_tell)
            .unwrap();
    }

    pub(super) fn define_fd_write(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let fd_write: Func = Func::wrap(
            store,
            |mut caller: Caller<'_, HostState>,
             fd: i32,
             iovs_ptr: i32,
             iovs_len: i32,
             nwritten_ptr: i32| {
                // Ensure fd is 1 (stdout)
                if fd != unistd::STDOUT_FILENO {
                    return Errno::Badf.into();
                }

                let memory = Self::get_memory_mut(&mut caller);

                // Read the iovec array
                let mut total_written = 0;
                for i in 0..iovs_len {
                    let iovec_base = iovs_ptr as usize + i as usize * 8;
                    let ptr =
                        u32::from_le_bytes(memory[iovec_base..iovec_base + 4].try_into().unwrap())
                            as usize;
                    let len = u32::from_le_bytes(
                        memory[iovec_base + 4..iovec_base + 8].try_into().unwrap(),
                    ) as usize;

                    let msg = core::str::from_utf8(&memory[ptr..ptr + len]).expect("Invalid utf8");
                    ::nvx::log!("{msg}");
                    total_written += len;
                }

                // Write the number of bytes written to nwritten_ptr
                let nwritten_bytes = (total_written as u32).to_le_bytes();
                let nwritten_ptr = nwritten_ptr as usize;
                memory[nwritten_ptr..nwritten_ptr + 4].copy_from_slice(&nwritten_bytes);

                0
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
