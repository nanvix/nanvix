// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

/// File descriptor rights.
#[derive(Debug, Clone)]
pub struct Rights {
    pub fd_datasync: bool,
    pub fd_read: bool,
    pub fd_seek: bool,
    pub fd_fdstat_set_flags: bool,
    pub fd_sync: bool,
    pub fd_tell: bool,
    pub fd_write: bool,
    pub fd_advise: bool,
    pub fd_allocate: bool,
    pub path_create_directory: bool,
    pub path_create_file: bool,
    pub path_link_source: bool,
    pub path_link_target: bool,
    pub path_open: bool,
    pub fd_readdir: bool,
    pub path_readlink: bool,
    pub path_rename_source: bool,
    pub path_rename_target: bool,
    pub path_filestat_get: bool,
    pub path_filestat_set_size: bool,
    pub path_filestat_set_times: bool,
    pub fd_filestat_get: bool,
    pub fd_filestat_set_size: bool,
    pub fd_filestat_set_times: bool,
    pub path_symlink: bool,
    pub path_remove_directory: bool,
    pub path_unlink_file: bool,
    pub poll_fd_readwrite: bool,
    pub sock_shutdown: bool,
    pub sock_accept: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Rights {
    const BIT_OFFSET_OF_FD_DATASYNC: u8 = 0;
    const BIT_OFFSET_OF_FD_READ: u8 = 1;
    const BIT_OFFSET_OF_FD_SEEK: u8 = 2;
    const BIT_OFFSET_OF_FD_FDSTAT_SET_FLAGS: u8 = 3;
    const BIT_OFFSET_OF_FD_SYNC: u8 = 4;
    const BIT_OFFSET_OF_FD_TELL: u8 = 5;
    const BIT_OFFSET_OF_FD_WRITE: u8 = 6;
    const BIT_OFFSET_OF_FD_ADVISE: u8 = 7;
    const BIT_OFFSET_OF_FD_ALLOCATE: u8 = 8;
    const BIT_OFFSET_OF_PATH_CREATE_DIRECTORY: u8 = 9;
    const BIT_OFFSET_OF_PATH_CREATE_FILE: u8 = 10;
    const BIT_OFFSET_OF_PATH_LINK_SOURCE: u8 = 11;
    const BIT_OFFSET_OF_PATH_LINK_TARGET: u8 = 12;
    const BIT_OFFSET_OF_PATH_OPEN: u8 = 13;
    const BIT_OFFSET_OF_FD_READDIR: u8 = 14;
    const BIT_OFFSET_OF_PATH_READLINK: u8 = 15;
    const BIT_OFFSET_OF_PATH_RENAME_SOURCE: u8 = 16;
    const BIT_OFFSET_OF_PATH_RENAME_TARGET: u8 = 17;
    const BIT_OFFSET_OF_PATH_FILESTAT_GET: u8 = 18;
    const BIT_OFFSET_OF_PATH_FILESTAT_SET_SIZE: u8 = 19;
    const BIT_OFFSET_OF_PATH_FILESTAT_SET_TIMES: u8 = 20;
    const BIT_OFFSET_OF_FD_FILESTAT_GET: u8 = 21;
    const BIT_OFFSET_OF_FD_FILESTAT_SET_SIZE: u8 = 22;
    const BIT_OFFSET_OF_FD_FILESTAT_SET_TIMES: u8 = 23;
    const BIT_OFFSET_OF_PATH_SYMLINK: u8 = 24;
    const BIT_OFFSET_OF_PATH_REMOVE_DIRECTORY: u8 = 25;
    const BIT_OFFSET_OF_PATH_UNLINK_FILE: u8 = 26;
    const BIT_OFFSET_OF_POLL_FD_READWRITE: u8 = 27;
    const BIT_OFFSET_OF_SOCK_SHUTDOWN: u8 = 28;
    const BIT_OFFSET_OF_SOCK_ACCEPT: u8 = 29;

    pub fn base_rights() -> Self {
        Self {
            fd_datasync: true,
            fd_read: true,
            fd_seek: true,
            fd_fdstat_set_flags: true,
            fd_sync: true,
            fd_tell: true,
            fd_write: true,
            fd_advise: true,
            fd_allocate: true,
            path_create_directory: true,
            path_create_file: true,
            path_link_source: true,
            path_link_target: true,
            path_open: true,
            fd_readdir: true,
            path_readlink: true,
            path_rename_source: true,
            path_rename_target: true,
            path_filestat_get: true,
            path_filestat_set_size: true,
            path_filestat_set_times: true,
            fd_filestat_get: true,
            fd_filestat_set_size: true,
            fd_filestat_set_times: true,
            path_symlink: true,
            path_remove_directory: true,
            path_unlink_file: true,
            poll_fd_readwrite: true,
            sock_shutdown: true,
            sock_accept: true,
        }
    }
}

impl From<u64> for Rights {
    fn from(val: u64) -> Self {
        Self {
            fd_datasync: val & (1 << Self::BIT_OFFSET_OF_FD_DATASYNC) != 0,
            fd_read: val & (1 << Self::BIT_OFFSET_OF_FD_READ) != 0,
            fd_seek: val & (1 << Self::BIT_OFFSET_OF_FD_SEEK) != 0,
            fd_fdstat_set_flags: val & (1 << Self::BIT_OFFSET_OF_FD_FDSTAT_SET_FLAGS) != 0,
            fd_sync: val & (1 << Self::BIT_OFFSET_OF_FD_SYNC) != 0,
            fd_tell: val & (1 << Self::BIT_OFFSET_OF_FD_TELL) != 0,
            fd_write: val & (1 << Self::BIT_OFFSET_OF_FD_WRITE) != 0,
            fd_advise: val & (1 << Self::BIT_OFFSET_OF_FD_ADVISE) != 0,
            fd_allocate: val & (1 << Self::BIT_OFFSET_OF_FD_ALLOCATE) != 0,
            path_create_directory: val & (1 << Self::BIT_OFFSET_OF_PATH_CREATE_DIRECTORY) != 0,
            path_create_file: val & (1 << Self::BIT_OFFSET_OF_PATH_CREATE_FILE) != 0,
            path_link_source: val & (1 << Self::BIT_OFFSET_OF_PATH_LINK_SOURCE) != 0,
            path_link_target: val & (1 << Self::BIT_OFFSET_OF_PATH_LINK_TARGET) != 0,
            path_open: val & (1 << Self::BIT_OFFSET_OF_PATH_OPEN) != 0,
            fd_readdir: val & (1 << Self::BIT_OFFSET_OF_FD_READDIR) != 0,
            path_readlink: val & (1 << Self::BIT_OFFSET_OF_PATH_READLINK) != 0,
            path_rename_source: val & (1 << Self::BIT_OFFSET_OF_PATH_RENAME_SOURCE) != 0,
            path_rename_target: val & (1 << Self::BIT_OFFSET_OF_PATH_RENAME_TARGET) != 0,
            path_filestat_get: val & (1 << Self::BIT_OFFSET_OF_PATH_FILESTAT_GET) != 0,
            path_filestat_set_size: val & (1 << Self::BIT_OFFSET_OF_PATH_FILESTAT_SET_SIZE) != 0,
            path_filestat_set_times: val & (1 << Self::BIT_OFFSET_OF_PATH_FILESTAT_SET_TIMES) != 0,
            fd_filestat_get: val & (1 << Self::BIT_OFFSET_OF_FD_FILESTAT_GET) != 0,
            fd_filestat_set_size: val & (1 << Self::BIT_OFFSET_OF_FD_FILESTAT_SET_SIZE) != 0,
            fd_filestat_set_times: val & (1 << Self::BIT_OFFSET_OF_FD_FILESTAT_SET_TIMES) != 0,
            path_symlink: val & (1 << Self::BIT_OFFSET_OF_PATH_SYMLINK) != 0,
            path_remove_directory: val & (1 << Self::BIT_OFFSET_OF_PATH_REMOVE_DIRECTORY) != 0,
            path_unlink_file: val & (1 << Self::BIT_OFFSET_OF_PATH_UNLINK_FILE) != 0,
            poll_fd_readwrite: val & (1 << Self::BIT_OFFSET_OF_POLL_FD_READWRITE) != 0,
            sock_shutdown: val & (1 << Self::BIT_OFFSET_OF_SOCK_SHUTDOWN) != 0,
            sock_accept: val & (1 << Self::BIT_OFFSET_OF_SOCK_ACCEPT) != 0,
        }
    }
}

impl From<i64> for Rights {
    fn from(val: i64) -> Self {
        Self::from(val as u64)
    }
}

impl From<u32> for Rights {
    fn from(val: u32) -> Self {
        Self::from(val as u64)
    }
}

impl From<i32> for Rights {
    fn from(val: i32) -> Self {
        Self::from(val as u64)
    }
}
