// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::boxed::Box;
use ::posix::{
    dirent::{
        self,
        DirectoryStream,
    },
    fcntl,
    fcntl::OpenFlags,
    sys::{
        self,
        stat::stat,
        types::size_t,
    },
    time::timespec,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn test() {
    // Create a file named `foo.tmp`.
    let fd: i32 = match fcntl::openat(
        fcntl::AT_FDCWD,
        "foo.tmp",
        OpenFlags::O_CREAT | OpenFlags::O_RDWR | OpenFlags::O_TRUNC,
        fcntl::S_IRUSR | fcntl::S_IWUSR,
    ) {
        Ok(fd) => {
            ::nvx::info!("opened file foo.tmp with fd {}", fd);
            fd
        },
        Err(error) => {
            panic!("failed to open file foo.tmp: {:?}", error);
        },
    };

    // Advice normal access.
    match fcntl::posix_fadvise(fd, 0, 0, fcntl::POSIX_FADV_NORMAL) {
        0 => {
            ::nvx::info!("advised normal access for file foo.tmp");
        },
        errno => {
            panic!("failed to advise normal access for file foo.tmp: {:?}", errno);
        },
    }

    // Fill first 128 bytes of file with ones.
    let buffer: [u8; 128] = [1; 128];
    match unistd::write(fd, &buffer) {
        Ok(128) => {
            ::nvx::info!("wrote 128 bytes to file foo.tmp");
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
            ::nvx::info!("wrote 64 bytes to file foo.tmp");
        },
        Ok(n) => {
            panic!("failed to write 64 bytes to file foo.tmp: (n={:?})", n);
        },
        errno => {
            panic!("failed to write 64 bytes to file foo.tmp: {:?}", errno);
        },
    }

    // Move seek offset start of file.
    match unistd::lseek(fd, 0, unistd::SEEK_SET) {
        0 => {
            ::nvx::info!("seek file foo.tmp to 1024 bytes");
        },
        offset => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
    }

    // Check if first 64 bytes are filled with ones using partial reads.
    let mut buffer: [u8; 64] = [0; 64];
    match unistd::pread(fd, buffer.as_mut_ptr(), buffer.len() as size_t, 0) {
        64 => {
            ::nvx::info!("read 64 bytes from file foo.tmp");
            (0..64).for_each(|i| {
                if buffer[i] != 1 {
                    panic!("file foo.tmp is not filled with ones");
                }
            });
        },
        errno => {
            panic!("failed to read 64 bytes from file foo.tmp: {:?}", errno);
        },
    }

    // Check if bytes [64..128] are filled with ones using offset partial reads.
    let mut buffer: [u8; 64] = [0; 64];
    match unistd::pread(fd, buffer.as_mut_ptr(), buffer.len() as size_t, 64) {
        64 => {
            ::nvx::info!("read 64 bytes from file foo.tmp");
            (0..64).for_each(|i| {
                if buffer[i] != 1 {
                    panic!("file foo.tmp is not filled with ones");
                }
            });
        },
        errno => {
            panic!("failed to read 64 bytes from file foo.tmp: {:?}", errno);
        },
    }

    // Advance seek offset as partial reads do not change it.
    match unistd::lseek(fd, 128, unistd::SEEK_SET) {
        128 => {
            ::nvx::info!("seek file foo.tmp to 128 bytes");
        },
        offset => {
            panic!("failed to seek file foo.tmp to 128 bytes: {:?}", offset);
        },
    }

    // Move seek offset to the end of the (empty) file plus 1024 bytes.
    match unistd::lseek(fd, 64, unistd::SEEK_END) {
        256 => {
            ::nvx::info!("seek file foo.tmp to 1024 bytes");
        },
        offset => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
    }

    // Attempt to allocate space.
    match fcntl::posix_fallocate(fd, 512, 512) {
        0 => {
            ::nvx::info!("allocated space for file foo.tmp");
        },
        errno => {
            panic!("failed to allocate space for file foo.tmp: {:?}", errno);
        },
    }

    // Synchronize changes to a file.
    match unistd::fsync(fd) {
        Ok(()) => {
            ::nvx::info!("synchronized file foo.tmp with storage device");
        },
        Err(e) => {
            panic!("failed to synchronize file foo.tmp with storage device ({:?})", e);
        },
    }

    // Get status of file.
    let mut st: stat = stat::default();
    match sys::stat::fstat(fd, &mut st) {
        Ok(()) => {
            ::nvx::info!("got status of file foo.tmp");
            ::nvx::info!("file statistics:");
            ::nvx::info!("  st_dev: {}", { st.st_dev });
            ::nvx::info!("  st_ino: {}", { st.st_ino });
            ::nvx::info!("  st_mode: {}", { st.st_mode });
            ::nvx::info!("  st_nlink: {}", { st.st_nlink });
            ::nvx::info!("  st_uid: {}", { st.st_uid });
            ::nvx::info!("  st_gid: {}", { st.st_gid });
            ::nvx::info!("  st_rdev: {}", { st.st_rdev });
            ::nvx::info!("  st_size: {}", { st.st_size });
            ::nvx::info!("  st_blksize: {}", { st.st_blksize });
            ::nvx::info!("  st_blocks: {}", { st.st_blocks });
            ::nvx::info!("  st_atime: {}s {}ns", { st.st_atim.tv_sec }, { st.st_atim.tv_nsec });
            ::nvx::info!("  st_mtime: {}s {}ns", { st.st_mtim.tv_sec }, { st.st_mtim.tv_nsec });
            ::nvx::info!("  st_ctime: {}s {}ns", { st.st_ctim.tv_sec }, { st.st_ctim.tv_nsec });
        },
        Err(error) => {
            panic!("failed to get status of file foo.tmp: {:?}", error);
        },
    }

    // Sanity check file size.
    if st.st_size != 1024 {
        panic!("file size is not 1024 bytes");
    }

    // Update access time of file named `foo.tmp`.
    let times: [timespec; 2] = [
        timespec {
            tv_sec: 1,
            tv_nsec: 1,
        },
        timespec {
            tv_sec: 1,
            tv_nsec: 1,
        },
    ];
    match sys::stat::futimens(fd, times) {
        0 => {
            ::nvx::info!("updated access time of file foo.tmp");
        },
        errno => {
            panic!("failed to update access time of file foo.tmp: {:?}", errno);
        },
    }

    // Get status of file named `foo.tmp`.
    let mut st: stat = stat::default();
    match sys::stat::fstat(fd, &mut st) {
        Ok(()) => {
            ::nvx::info!("got status of file foo.tmp");
            ::nvx::info!("file statistics:");
            ::nvx::info!("  st_dev: {}", { st.st_dev });
            ::nvx::info!("  st_ino: {}", { st.st_ino });
            ::nvx::info!("  st_mode: {}", { st.st_mode });
            ::nvx::info!("  st_nlink: {}", { st.st_nlink });
            ::nvx::info!("  st_uid: {}", { st.st_uid });
            ::nvx::info!("  st_gid: {}", { st.st_gid });
            ::nvx::info!("  st_rdev: {}", { st.st_rdev });
            ::nvx::info!("  st_size: {}", { st.st_size });
            ::nvx::info!("  st_blksize: {}", { st.st_blksize });
            ::nvx::info!("  st_blocks: {}", { st.st_blocks });
            ::nvx::info!("  st_atime: {}s {}ns", { st.st_atim.tv_sec }, { st.st_atim.tv_nsec });
            ::nvx::info!("  st_mtime: {}s {}ns", { st.st_mtim.tv_sec }, { st.st_mtim.tv_nsec });
            ::nvx::info!("  st_ctime: {}s {}ns", { st.st_ctim.tv_sec }, { st.st_ctim.tv_nsec });
        },
        Err(error) => {
            panic!("failed to get status of file foo.tmp: {:?}", error);
        },
    }

    // Ensure time of last access was updated.
    if st.st_atim.tv_sec != 1 {
        panic!("access time of file bar.tmp was not updated");
    }
    if st.st_atim.tv_nsec != 1 {
        panic!("access time of file bar.tmp was not updated");
    }

    // Close file.
    match unistd::close(fd) {
        Ok(()) => {
            ::nvx::info!("closed file foo.tmp");
        },
        Err(error) => {
            panic!("failed to close file foo.tmp: {:?}", error);
        },
    }

    // Get status of file.
    let path: &str = "foo.tmp";
    let mut foo_tmp: stat = stat::default();
    match sys::stat::stat(path, &mut foo_tmp) {
        Ok(()) => {
            ::nvx::info!("got status of file {}", path);
            ::nvx::info!("file statistics:");
            ::nvx::info!("  st_dev: {}", { foo_tmp.st_dev });
            ::nvx::info!("  st_ino: {}", { foo_tmp.st_ino });
            ::nvx::info!("  st_mode: {}", { foo_tmp.st_mode });
            ::nvx::info!("  st_nlink: {}", { foo_tmp.st_nlink });
            ::nvx::info!("  st_uid: {}", { foo_tmp.st_uid });
            ::nvx::info!("  st_gid: {}", { foo_tmp.st_gid });
            ::nvx::info!("  st_rdev: {}", { foo_tmp.st_rdev });
            ::nvx::info!("  st_size: {}", { foo_tmp.st_size });
            ::nvx::info!("  st_blksize: {}", { foo_tmp.st_blksize });
            ::nvx::info!("  st_blocks: {}", { foo_tmp.st_blocks });
            ::nvx::info!("  st_atime: {}s {}ns", { foo_tmp.st_atim.tv_sec }, {
                foo_tmp.st_atim.tv_nsec
            });
            ::nvx::info!("  st_mtime: {}s {}ns", { foo_tmp.st_mtim.tv_sec }, {
                foo_tmp.st_mtim.tv_nsec
            });
            ::nvx::info!("  st_ctime: {}s {}ns", { foo_tmp.st_ctim.tv_sec }, {
                foo_tmp.st_ctim.tv_nsec
            });
        },
        Err(error) => {
            panic!("failed to get status of file {:?}: {:?}", path, error);
        },
    }

    // Get status of file named `foo.tmp`.
    let mut bar_tmp: stat = stat::default();
    match sys::stat::fstatat(fcntl::AT_FDCWD, "foo.tmp", &mut bar_tmp, 0) {
        Ok(()) => {
            ::nvx::info!("got status of file foo.tmp");
            ::nvx::info!("file statistics:");
            ::nvx::info!("  st_dev: {}", { bar_tmp.st_dev });
            ::nvx::info!("  st_ino: {}", { bar_tmp.st_ino });
            ::nvx::info!("  st_mode: {}", { bar_tmp.st_mode });
            ::nvx::info!("  st_nlink: {}", { bar_tmp.st_nlink });
            ::nvx::info!("  st_uid: {}", { bar_tmp.st_uid });
            ::nvx::info!("  st_gid: {}", { bar_tmp.st_gid });
            ::nvx::info!("  st_rdev: {}", { bar_tmp.st_rdev });
            ::nvx::info!("  st_size: {}", { bar_tmp.st_size });
            ::nvx::info!("  st_blksize: {}", { bar_tmp.st_blksize });
            ::nvx::info!("  st_blocks: {}", { bar_tmp.st_blocks });
            ::nvx::info!("  st_atime: {}s {}ns", { bar_tmp.st_atim.tv_sec }, {
                bar_tmp.st_atim.tv_nsec
            });
            ::nvx::info!("  st_mtime: {}s {}ns", { bar_tmp.st_mtim.tv_sec }, {
                bar_tmp.st_mtim.tv_nsec
            });
            ::nvx::info!("  st_ctime: {}s {}ns", { bar_tmp.st_ctim.tv_sec }, {
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
    match fcntl::unlinkat(fcntl::AT_FDCWD, "foo.tmp", 0) {
        Ok(()) => {
            ::nvx::info!("unlinked file foo.tmp");
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
            ::nvx::info!("created pipe with fds ({}, {})", fds[0], fds[1]);
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
            ::nvx::info!("wrote 128 bytes to pipe");
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
    match unistd::read(read_fd, read_buffer.as_mut_ptr(), read_buffer.len() as size_t) {
        128 => {
            ::nvx::info!("read 128 bytes from pipe");
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
            ::nvx::info!("closed read end of pipe");
        },
        Err(error) => {
            panic!("failed to close read end of pipe: {:?}", error);
        },
    }

    // Close write end of pipe.
    match unistd::close(write_fd) {
        Ok(()) => {
            ::nvx::info!("closed write end of pipe");
        },
        Err(error) => {
            panic!("failed to close write end of pipe: {:?}", error);
        },
    }

    // Get current working directory.
    match unistd::getcwd() {
        Ok(cwd) => {
            ::nvx::info!("got current working directory: {}", cwd);
        },
        Err(e) => {
            panic!("failed to get current working directory: {:?}", e);
        },
    };

    // Open directory.
    let dir_fd: i32 = match fcntl::openat(fcntl::AT_FDCWD, ".", OpenFlags::O_DIRECTORY.into(), 0) {
        Ok(fd) => {
            ::nvx::info!("opened directory with fd {}", fd);
            fd
        },
        Err(error) => {
            panic!("failed to open directory: {:?}", error);
        },
    };

    match dirent::posix_getdents(dir_fd, 1) {
        Ok(buffer) => {
            for d in buffer.iter() {
                ::nvx::info!("directory entry: {:?}", d);
            }
        },
        Err(error) => {
            panic!("failed to get directory entries: {:?}", error);
        },
    }

    // Close directory.
    match unistd::close(dir_fd) {
        Ok(()) => {
            ::nvx::info!("closed directory");
        },
        Err(error) => {
            panic!("failed to close directory: {:?}", error);
        },
    }

    // Open directory stream.
    let mut dir: Box<DirectoryStream> = match dirent::opendir(".") {
        Ok(dir) => {
            ::nvx::info!("opened directory");
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
                ::nvx::info!("directory entry: {:?}", dirent);
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
            ::nvx::info!("closed directory");
        },
        Err(error) => {
            panic!("failed to close directory: {:?}", error);
        },
    }
}
