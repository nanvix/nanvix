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
    pub(super) fn define_path_create_directory(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_create_directory: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "path_create_directory(): fd={:?}, path_offset={:?}, path_len={:?}",
                    fd,
                    path_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_create_directory", path_create_directory)
            .unwrap();
    }

    pub(super) fn define_path_filestat_get(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_filestat_get: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             flags: i32,
             buf: i32,
             len: i32,
             offset0: i32|
             -> i32 {
                ::nvx::log!("path_filestat_get: {fd}, {flags}, {buf} {len} {offset0}");
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_filestat_get", path_filestat_get)
            .unwrap();
    }

    pub(super) fn define_path_filestat_set_times(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_filestat_set_times: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             flags: i32,
             buf: i32,
             len: i32,
             atim: i64,
             mtim: i64,
             fst_flags: i32|
             -> i32 {
                ::nvx::log!(
                    "path_filestat_set_times: {fd}, {flags}, {buf} {len} {atim} {mtim} {fst_flags}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_filestat_set_times", path_filestat_set_times)
            .unwrap();
    }

    pub(super) fn define_path_link(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let path_link: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             old_fd: i32,
             old_flags: i32,
             old_path_offset: i32,
             old_path_len: i32,
             new_fd: i32,
             new_path_offset: i32,
             new_path_len: i32|
             -> i32 {
                ::nvx::log!(
                    "path_link: {old_fd}, {old_flags}, {old_path_offset}, {old_path_len}, \
                     {new_fd}, {new_path_offset}, {new_path_len}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_link", path_link)
            .unwrap();
    }

    pub(super) fn define_path_open(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_open: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>,
                  fd: i32,
                  dirflags: i32,
                  path_offset: i32,
                  path_length: i32,
                  oflags: i32,
                  fs_rights_base: i64,
                  fdflags: i64,
                  fs_rights_inheriting: i32,
                  fd_offset: i32|
                  -> i32 {
                ::nvx::log!(
                    "path_open(): fd={:?}, dirflags={:?}, path_offset={:?}, path_length={:?}, \
                     oflags={:?}, fs_rights_base={:?}, fdflags={:?}, fs_rights_inheriting={:?}, \
                     fd_offset={:?}",
                    fd,
                    dirflags,
                    path_offset,
                    path_length,
                    oflags,
                    fs_rights_base,
                    fdflags,
                    fs_rights_inheriting,
                    fd_offset
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_open", path_open)
            .unwrap();
    }

    pub(super) fn define_path_readlink(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_readlink: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             fd: i32,
             path_offset: i32,
             path_len: i32,
             buf: i32,
             buf_len: i32,
             offset: i32|
             -> i32 {
                ::nvx::log!(
                    "path_readlink: {fd}, {path_offset}, {path_len}, {buf}, {buf_len} {offset}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_readlink", path_readlink)
            .unwrap();
    }

    pub(super) fn define_path_remove_directory(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_remove_directory: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "path_remove_directory(): fd={:?}, path_offset={:?}, path_len={:?}",
                    fd,
                    path_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_remove_directory", path_remove_directory)
            .unwrap();
    }

    pub(super) fn define_path_rename(linker: &mut Linker<HostState>, store: &mut Store<HostState>) {
        let path_rename: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             old_fd: i32,
             old_path_offset: i32,
             old_path_len: i32,
             new_fd: i32,
             new_path_offset: i32,
             new_path_len: i32|
             -> i32 {
                ::nvx::log!(
                    "path_rename: {old_fd}, {old_path_offset}, {old_path_len}, {new_fd}, \
                     {new_path_offset}, {new_path_len}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_rename", path_rename)
            .unwrap();
    }

    pub(super) fn define_path_symlink(
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_symlink: Func = Func::wrap(
            store,
            |_caller: Caller<'_, u32>,
             old_path_offset: i32,
             old_path_len: i32,
             fd: i32,
             new_path_offset: i32,
             new_path_len: i32|
             -> i32 {
                ::nvx::log!(
                    "path_symlink: {old_path_offset}, {old_path_len}, {fd}, {new_path_offset}, \
                     {new_path_len}"
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_symlink", path_symlink)
            .unwrap();
    }

    pub(super) fn define_path_unlink_file(
        _ctx: Arc<WasiCtx>,
        linker: &mut Linker<HostState>,
        store: &mut Store<HostState>,
    ) {
        let path_unlink_file: Func = Func::wrap(
            store,
            move |_caller: Caller<'_, u32>, fd: i32, path_offset: i32, path_len: i32| -> i32 {
                ::nvx::log!(
                    "path_unlink_file(): fd={:?}, path_offset={:?}, path_len={:?}",
                    fd,
                    path_offset,
                    path_len
                );
                Errno::Nosys.into()
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_unlink_file", path_unlink_file)
            .unwrap();
    }
}
