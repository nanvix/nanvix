// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::boxed::Box;
use ::sysapi::{
    fcntl::{
        atflags::AT_FDCWD,
        open_flags::{
            O_CREAT,
            O_DIRECTORY,
            O_RDWR,
            O_TRUNC,
        },
    },
    sys_stat::{
        self,
        file_mode::{
            S_IRUSR,
            S_IWUSR,
        },
    },
    unistd::file_seek::{
        SEEK_END,
        SEEK_SET,
    },
};
use ::syscall::{
    dirent::{
        self,
        DirectoryStream,
    },
    fcntl,
    sys,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn test() {
    // Create a file named `foo.tmp`.
    let fd: i32 =
        match fcntl::openat(AT_FDCWD, "foo.tmp", O_CREAT | O_RDWR | O_TRUNC, S_IRUSR | S_IWUSR) {
            Ok(fd) => {
                ::syslog::info!("opened file foo.tmp with fd {}", fd);
                fd
            },
            Err(error) => {
                panic!("failed to open file foo.tmp: {:?}", error);
            },
        };

    // Fill first 128 bytes of file with ones.
    let buffer: [u8; 128] = [1; 128];
    match unistd::write(fd, &buffer) {
        Ok(128) => {
            ::syslog::info!("wrote 128 bytes to file foo.tmp");
        },
        Ok(n) => {
            panic!("failed to write 128 bytes to file foo.tmp: (n={:?})", n);
        },
        Err(error) => {
            panic!("failed to write 128 bytes to file foo.tmp: (error={:?})", error);
        },
    }

    // Fill bytes [128, 192] with ones using partial write.
    let buffer: [u8; 64] = [1; 64];
    match unistd::pwrite(fd, &buffer, 128) {
        Ok(64) => {
            ::syslog::info!("wrote 64 bytes to file foo.tmp");
        },
        Ok(n) => {
            panic!("failed to write 64 bytes to file foo.tmp: (n={:?})", n);
        },
        errno => {
            panic!("failed to write 64 bytes to file foo.tmp: {:?}", errno);
        },
    }

    // Move seek offset start of file.
    match unistd::lseek(fd, 0, SEEK_SET) {
        Ok(0) => {
            ::syslog::info!("seek file foo.tmp to 1024 bytes");
        },
        Ok(offset) => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
        offset => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
    }

    // Check if first 64 bytes are filled with ones using partial reads.
    let mut buffer: [u8; 64] = [0; 64];
    match unistd::pread(fd, &mut buffer, 0) {
        Ok(64) => {
            ::syslog::info!("read 64 bytes from file foo.tmp");
            (0..64).for_each(|i| {
                if buffer[i] != 1 {
                    panic!("file foo.tmp is not filled with ones");
                }
            });
        },
        Ok(n) => {
            panic!("failed to read 64 bytes from file foo.tmp: (n={:?})", n);
        },
        errno => {
            panic!("failed to read 64 bytes from file foo.tmp: {:?}", errno);
        },
    }

    // Check if bytes [64..128] are filled with ones using offset partial reads.
    let mut buffer: [u8; 64] = [0; 64];
    match unistd::pread(fd, &mut buffer, 64) {
        Ok(64) => {
            ::syslog::info!("read 64 bytes from file foo.tmp");
            (0..64).for_each(|i| {
                if buffer[i] != 1 {
                    panic!("file foo.tmp is not filled with ones");
                }
            });
        },
        Ok(n) => {
            panic!("failed to read 64 bytes from file foo.tmp: (n={:?})", n);
        },
        errno => {
            panic!("failed to read 64 bytes from file foo.tmp: {:?}", errno);
        },
    }

    // Advance seek offset as partial reads do not change it.
    match unistd::lseek(fd, 128, SEEK_SET) {
        Ok(128) => {
            ::syslog::info!("seek file foo.tmp to 128 bytes");
        },
        Ok(offset) => {
            panic!("failed to seek file foo.tmp to 128 bytes: {:?}", offset);
        },
        offset => {
            panic!("failed to seek file foo.tmp to 128 bytes: {:?}", offset);
        },
    }

    // Move seek offset to the end of the (empty) file plus 1024 bytes.
    match unistd::lseek(fd, 64, SEEK_END) {
        Ok(256) => {
            ::syslog::info!("seek file foo.tmp to 1024 bytes");
        },
        Ok(offset) => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
        offset => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
    }

    // Attempt to allocate space.
    match fcntl::posix_fallocate(fd, 512, 512) {
        Ok(()) => {
            ::syslog::info!("allocated space for file foo.tmp");
        },
        Err(error) => {
            panic!("failed to allocate space for file foo.tmp: {:?}", error);
        },
    }

    // Synchronize changes to a file.
    match unistd::fsync(fd) {
        Ok(()) => {
            ::syslog::info!("synchronized file foo.tmp with storage device");
        },
        Err(e) => {
            panic!("failed to synchronize file foo.tmp with storage device ({:?})", e);
        },
    }

    // Get status of file.
    let mut st: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::fstat(fd, &mut st) {
        Ok(()) => {
            ::syslog::info!("got status of file foo.tmp");
            ::syslog::info!("file statistics:");
            ::syslog::info!("  st_dev: {}", { st.st_dev });
            ::syslog::info!("  st_ino: {}", { st.st_ino });
            ::syslog::info!("  st_mode: {}", { st.st_mode });
            ::syslog::info!("  st_nlink: {}", { st.st_nlink });
            ::syslog::info!("  st_uid: {}", { st.st_uid });
            ::syslog::info!("  st_gid: {}", { st.st_gid });
            ::syslog::info!("  st_rdev: {}", { st.st_rdev });
            ::syslog::info!("  st_size: {}", { st.st_size });
            ::syslog::info!("  st_blksize: {}", { st.st_blksize });
            ::syslog::info!("  st_blocks: {}", { st.st_blocks });
            ::syslog::info!("  st_atime: {}s {}ns", { st.st_atim.tv_sec }, { st.st_atim.tv_nsec });
            ::syslog::info!("  st_mtime: {}s {}ns", { st.st_mtim.tv_sec }, { st.st_mtim.tv_nsec });
            ::syslog::info!("  st_ctime: {}s {}ns", { st.st_ctim.tv_sec }, { st.st_ctim.tv_nsec });
        },
        Err(error) => {
            panic!("failed to get status of file foo.tmp: {:?}", error);
        },
    }

    // Sanity check file size.
    if st.st_size != 1024 {
        panic!("file size is not 1024 bytes");
    }

    // Close file.
    match unistd::close(fd) {
        Ok(()) => {
            ::syslog::info!("closed file foo.tmp");
        },
        Err(error) => {
            panic!("failed to close file foo.tmp: {:?}", error);
        },
    }

    // Get status of file.
    let path: &str = "foo.tmp";
    let mut foo_tmp: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::stat(path, &mut foo_tmp) {
        Ok(()) => {
            ::syslog::info!("got status of file {}", path);
            ::syslog::info!("file statistics:");
            ::syslog::info!("  st_dev: {}", { foo_tmp.st_dev });
            ::syslog::info!("  st_ino: {}", { foo_tmp.st_ino });
            ::syslog::info!("  st_mode: {}", { foo_tmp.st_mode });
            ::syslog::info!("  st_nlink: {}", { foo_tmp.st_nlink });
            ::syslog::info!("  st_uid: {}", { foo_tmp.st_uid });
            ::syslog::info!("  st_gid: {}", { foo_tmp.st_gid });
            ::syslog::info!("  st_rdev: {}", { foo_tmp.st_rdev });
            ::syslog::info!("  st_size: {}", { foo_tmp.st_size });
            ::syslog::info!("  st_blksize: {}", { foo_tmp.st_blksize });
            ::syslog::info!("  st_blocks: {}", { foo_tmp.st_blocks });
            ::syslog::info!("  st_atime: {}s {}ns", { foo_tmp.st_atim.tv_sec }, {
                foo_tmp.st_atim.tv_nsec
            });
            ::syslog::info!("  st_mtime: {}s {}ns", { foo_tmp.st_mtim.tv_sec }, {
                foo_tmp.st_mtim.tv_nsec
            });
            ::syslog::info!("  st_ctime: {}s {}ns", { foo_tmp.st_ctim.tv_sec }, {
                foo_tmp.st_ctim.tv_nsec
            });
        },
        Err(error) => {
            panic!("failed to get status of file {:?}: {:?}", path, error);
        },
    }

    // Get status of file named `foo.tmp`.
    let mut bar_tmp: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::fstatat(AT_FDCWD, "foo.tmp", &mut bar_tmp, 0) {
        Ok(()) => {
            ::syslog::info!("got status of file foo.tmp");
            ::syslog::info!("file statistics:");
            ::syslog::info!("  st_dev: {}", { bar_tmp.st_dev });
            ::syslog::info!("  st_ino: {}", { bar_tmp.st_ino });
            ::syslog::info!("  st_mode: {}", { bar_tmp.st_mode });
            ::syslog::info!("  st_nlink: {}", { bar_tmp.st_nlink });
            ::syslog::info!("  st_uid: {}", { bar_tmp.st_uid });
            ::syslog::info!("  st_gid: {}", { bar_tmp.st_gid });
            ::syslog::info!("  st_rdev: {}", { bar_tmp.st_rdev });
            ::syslog::info!("  st_size: {}", { bar_tmp.st_size });
            ::syslog::info!("  st_blksize: {}", { bar_tmp.st_blksize });
            ::syslog::info!("  st_blocks: {}", { bar_tmp.st_blocks });
            ::syslog::info!("  st_atime: {}s {}ns", { bar_tmp.st_atim.tv_sec }, {
                bar_tmp.st_atim.tv_nsec
            });
            ::syslog::info!("  st_mtime: {}s {}ns", { bar_tmp.st_mtim.tv_sec }, {
                bar_tmp.st_mtim.tv_nsec
            });
            ::syslog::info!("  st_ctime: {}s {}ns", { bar_tmp.st_ctim.tv_sec }, {
                bar_tmp.st_ctim.tv_nsec
            });
        },
        Err(error) => {
            panic!("failed to get status of file foo.tmp: {:?}", error);
        },
    }

    // Ensure that foo.tmp and foo.tmp are the same file.
    if foo_tmp.st_ino != bar_tmp.st_ino {
        panic!("foo.tmp and foo.tmp are not the same file");
    }

    // Unlink file named `foo.tmp`.
    match fcntl::unlinkat(AT_FDCWD, "foo.tmp", 0) {
        Ok(()) => {
            ::syslog::info!("unlinked file foo.tmp");
        },
        Err(error) => {
            panic!("failed to unlink file foo.tmp (error={:?})", error);
        },
    }

    test_pipe();
}

fn test_pipe() {
    let [read_fd, write_fd]: [i32; 2] = match unistd::pipe() {
        Ok(fds) => {
            ::syslog::info!("created pipe with fds ({}, {})", fds[0], fds[1]);
            fds
        },
        Err(e) => {
            panic!("failed to create pipe: {:?}", e);
        },
    };

    // Write to pipe.
    let write_buffer: [u8; 128] = [1; 128];
    match unistd::write(write_fd, &write_buffer) {
        Ok(128) => {
            ::syslog::info!("wrote 128 bytes to pipe");
        },
        Ok(n) => {
            panic!("failed to write 128 bytes to pipe: (n={:?})", n);
        },
        Err(error) => {
            panic!("failed to write 128 bytes to pipe (error={:?})", error);
        },
    }

    // Read from pipe.
    let mut read_buffer: [u8; 128] = [0; 128];
    match unistd::read(read_fd, &mut read_buffer) {
        Ok(128) => {
            ::syslog::info!("read 128 bytes from pipe");
        },
        Ok(n) => {
            panic!("failed to read 128 bytes from pipe: (n={:?})", n);
        },
        errno => {
            panic!("failed to read 128 bytes from pipe: {:?}", errno);
        },
    }

    // Check if contents of read buffer matches write buffer.
    (0..128).for_each(|i| {
        if read_buffer[i] != write_buffer[i] {
            panic!("read buffer does not match write buffer");
        }
    });

    // Close read end of pipe.
    match unistd::close(read_fd) {
        Ok(()) => {
            ::syslog::info!("closed read end of pipe");
        },
        Err(error) => {
            panic!("failed to close read end of pipe: {:?}", error);
        },
    }

    // Close write end of pipe.
    match unistd::close(write_fd) {
        Ok(()) => {
            ::syslog::info!("closed write end of pipe");
        },
        Err(error) => {
            panic!("failed to close write end of pipe: {:?}", error);
        },
    }

    // Get current working directory.
    match unistd::getcwd() {
        Ok(cwd) => {
            ::syslog::info!("got current working directory: {}", cwd);
        },
        Err(e) => {
            panic!("failed to get current working directory: {:?}", e);
        },
    };

    // Open directory.
    let dir_fd: i32 = match fcntl::openat(AT_FDCWD, ".", O_DIRECTORY, 0) {
        Ok(fd) => {
            ::syslog::info!("opened directory with fd {}", fd);
            fd
        },
        Err(error) => {
            panic!("failed to open directory: {:?}", error);
        },
    };

    loop {
        match dirent::posix_getdents(dir_fd, 1) {
            Ok(buffer) => {
                if buffer.is_empty() {
                    break;
                }
                for d in buffer.iter() {
                    ::syslog::info!("directory entry: {:?}", d);
                }
            },
            Err(error) => {
                panic!("failed to get directory entries: {:?}", error);
            },
        }
    }

    // Close directory.
    match unistd::close(dir_fd) {
        Ok(()) => {
            ::syslog::info!("closed directory");
        },
        Err(error) => {
            panic!("failed to close directory: {:?}", error);
        },
    }

    // Open directory stream.
    let mut dir: Box<DirectoryStream> = match dirent::opendir(".") {
        Ok(dir) => {
            ::syslog::info!("opened directory");
            dir
        },
        Err(error) => {
            panic!("failed to open directory: {:?}", error);
        },
    };

    // Read directory stream.
    loop {
        match dirent::readdir(&mut dir) {
            Ok(Some(dirent)) => {
                ::syslog::info!("directory entry: {:?}", dirent);
            },
            Ok(None) => {
                break;
            },
            Err(error) => {
                panic!("failed to read directory: {:?}", error);
            },
        }
    }

    // Close directory stream.
    match dirent::closedir(&mut dir) {
        Ok(()) => {
            ::syslog::info!("closed directory");
        },
        Err(error) => {
            panic!("failed to close directory: {:?}", error);
        },
    }
}
