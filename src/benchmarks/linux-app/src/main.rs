// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::{
    fcntl,
    sys::{
        self,
        stat::stat,
        types::size_t,
        uio,
    },
    time::{
        self,
        timespec,
        CLOCK_MONOTONIC,
    },
    unistd,
    venv,
    venv::VirtualEnvironmentIdentifier,
};
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    let env: VirtualEnvironmentIdentifier = venv::join(VirtualEnvironmentIdentifier::NEW)?;
    ::nvx::log!("joined environment {:?}", env);

    let mut res: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    match time::clock_getres(CLOCK_MONOTONIC, &mut res) {
        0 => {
            ::nvx::log!("clock resolution: {}s {}ns", { res.tv_sec }, { res.tv_nsec });
        },
        errno => {
            panic!("failed to get clock resolution: {:?}", errno);
        },
    }

    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    match time::clock_gettime(CLOCK_MONOTONIC, &mut tp) {
        0 => {
            ::nvx::log!("clock time: {}s {}ns", { tp.tv_sec }, { tp.tv_nsec });
        },
        errno => {
            panic!("failed to get clock time: {:?}", errno);
        },
    }

    // Create a file named `foo.tmp`.
    let fd: i32 = match fcntl::openat(
        fcntl::AT_FDCWD,
        "foo.tmp",
        fcntl::O_CREAT | fcntl::O_RDWR | fcntl::O_TRUNC,
        fcntl::S_IRUSR | fcntl::S_IWUSR,
    ) {
        fd if fd >= 0 => {
            ::nvx::log!("opened file foo.tmp with fd {}", fd);
            fd
        },
        errno => {
            panic!("failed to open file foo.tmp: {:?}", errno);
        },
    };

    // Advice normal access.
    match fcntl::posix_fadvise(fd, 0, 0, fcntl::POSIX_FADV_NORMAL) {
        0 => {
            ::nvx::log!("advised normal access for file foo.tmp");
        },
        errno => {
            panic!("failed to advise normal access for file foo.tmp: {:?}", errno);
        },
    }

    // Fill first 128 bytes of file with ones.
    let buffer: [u8; 128] = [1; 128];
    match unistd::write(fd, buffer.as_ptr(), buffer.len() as size_t) {
        128 => {
            ::nvx::log!("wrote 128 bytes to file foo.tmp");
        },
        errno => {
            panic!("failed to write 128 bytes to file foo.tmp: {:?}", errno);
        },
    }

    // Fill bytes [128, 256] with ones using partial write.
    let buffer: [u8; 128] = [1; 128];
    match unistd::pwrite(fd, buffer.as_ptr(), buffer.len() as size_t, 128) {
        128 => {
            ::nvx::log!("wrote 128 bytes to file foo.tmp");
        },
        errno => {
            panic!("failed to write 128 bytes to file foo.tmp: {:?}", errno);
        },
    }

    // Advance seek offset as partial writes do not change it.
    match unistd::lseek(fd, 256, unistd::SEEK_SET) {
        256 => {
            ::nvx::log!("seek file foo.tmp to 256 bytes");
        },
        offset => {
            panic!("failed to seek file foo.tmp to 256 bytes: {:?}", offset);
        },
    }

    // Fill bytes [256..512] with ones using vectored i/o operations.
    let buffer: [u8; 256] = [1; 256];
    let iov: [uio::iovec; 2] = [
        uio::iovec {
            iov_base: buffer.as_ptr() as *mut u8,
            iov_len: 128,
        },
        uio::iovec {
            iov_base: unsafe { buffer.as_ptr().add(128) } as *mut u8,
            iov_len: 128,
        },
    ];
    match uio::writev(fd, iov.as_ptr(), iov.len() as i32) {
        256 => {
            ::nvx::log!("wrote 256 bytes to file foo.tmp");
        },
        errno => {
            panic!("failed to write 256 bytes to file foo.tmp: {:?}", errno);
        },
    }

    // Move seek offset start of file.
    match unistd::lseek(fd, 0, unistd::SEEK_SET) {
        0 => {
            ::nvx::log!("seek file foo.tmp to 1024 bytes");
        },
        offset => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
    }

    // Check if first 512 bytes are filled with ones.
    let mut buffer: [u8; 512] = [0; 512];
    match unistd::read(fd, buffer.as_mut_ptr(), buffer.len() as size_t) {
        512 => {
            ::nvx::log!("read 512 bytes from file foo.tmp");
            (0..512).for_each(|i| {
                if buffer[i] != 1 {
                    panic!("file foo.tmp is not filled with ones");
                }
            });
        },
        errno => {
            panic!("failed to read 512 bytes from file foo.tmp: {:?}", errno);
        },
    }

    // Move seek offset to the end of the (empty) file plus 1024 bytes.
    match unistd::lseek(fd, 512, unistd::SEEK_END) {
        1024 => {
            ::nvx::log!("seek file foo.tmp to 1024 bytes");
        },
        offset => {
            panic!("failed to seek file foo.tmp to 1024 bytes: {:?}", offset);
        },
    }

    // Truncate file to 512 bytes.
    match unistd::ftruncate(fd, 512) {
        0 => {
            ::nvx::log!("truncated file foo.tmp to 512 bytes");
        },
        errno => {
            panic!("failed to truncate file foo.tmp to 512 bytes: {:?}", errno);
        },
    }

    // Attempt to allocate space.
    match fcntl::posix_fallocate(fd, 512, 512) {
        0 => {
            ::nvx::log!("allocated space for file foo.tmp");
        },
        errno => {
            panic!("failed to allocate space for file foo.tmp: {:?}", errno);
        },
    }

    // Synchronize data changes to file.
    match unistd::fdatasync(fd) {
        0 => {
            ::nvx::log!("synchronized file foo.tmp with storage device");
        },
        errno => {
            panic!("failed to synchronize file foo.tmp with storage device: {:?}", errno);
        },
    }

    // Synchronize changes to a file.
    match unistd::fsync(fd) {
        0 => {
            ::nvx::log!("synchronized file foo.tmp with storage device");
        },
        errno => {
            panic!("failed to synchronize file foo.tmp with storage device: {:?}", errno);
        },
    }

    // Get status of file.
    let mut st: stat = stat::default();
    match sys::stat::fstat(fd, &mut st) {
        0 => {
            ::nvx::log!("got status of file foo.tmp");
            ::nvx::log!("file statistics:");
            ::nvx::log!("  st_dev: {}", { st.st_dev });
            ::nvx::log!("  st_ino: {}", { st.st_ino });
            ::nvx::log!("  st_mode: {}", { st.st_mode });
            ::nvx::log!("  st_nlink: {}", { st.st_nlink });
            ::nvx::log!("  st_uid: {}", { st.st_uid });
            ::nvx::log!("  st_gid: {}", { st.st_gid });
            ::nvx::log!("  st_rdev: {}", { st.st_rdev });
            ::nvx::log!("  st_size: {}", { st.st_size });
            ::nvx::log!("  st_blksize: {}", { st.st_blksize });
            ::nvx::log!("  st_blocks: {}", { st.st_blocks });
            ::nvx::log!("  st_atime: {}s {}ns", { st.st_atim.tv_sec }, { st.st_atim.tv_nsec });
            ::nvx::log!("  st_mtime: {}s {}ns", { st.st_mtim.tv_sec }, { st.st_mtim.tv_nsec });
            ::nvx::log!("  st_ctime: {}s {}ns", { st.st_ctim.tv_sec }, { st.st_ctim.tv_nsec });
        },
        errno => {
            panic!("failed to get status of file foo.tmp: {:?}", errno);
        },
    }

    // Sanity check file size.
    if st.st_size != 1024 {
        panic!("file size is not 1024 bytes");
    }

    // Close file.
    match unistd::close(fd) {
        0 => {
            ::nvx::log!("closed file foo.tmp");
        },
        errno => {
            panic!("failed to close file foo.tmp: {:?}", errno);
        },
    }

    // Get status of file.
    let path: &str = "foo.tmp";
    let mut foo_tmp: stat = stat::default();
    match sys::stat::stat(path, &mut foo_tmp) {
        0 => {
            ::nvx::log!("got status of file {}", path);
            ::nvx::log!("file statistics:");
            ::nvx::log!("  st_dev: {}", { foo_tmp.st_dev });
            ::nvx::log!("  st_ino: {}", { foo_tmp.st_ino });
            ::nvx::log!("  st_mode: {}", { foo_tmp.st_mode });
            ::nvx::log!("  st_nlink: {}", { foo_tmp.st_nlink });
            ::nvx::log!("  st_uid: {}", { foo_tmp.st_uid });
            ::nvx::log!("  st_gid: {}", { foo_tmp.st_gid });
            ::nvx::log!("  st_rdev: {}", { foo_tmp.st_rdev });
            ::nvx::log!("  st_size: {}", { foo_tmp.st_size });
            ::nvx::log!("  st_blksize: {}", { foo_tmp.st_blksize });
            ::nvx::log!("  st_blocks: {}", { foo_tmp.st_blocks });
            ::nvx::log!("  st_atime: {}s {}ns", { foo_tmp.st_atim.tv_sec }, {
                foo_tmp.st_atim.tv_nsec
            });
            ::nvx::log!("  st_mtime: {}s {}ns", { foo_tmp.st_mtim.tv_sec }, {
                foo_tmp.st_mtim.tv_nsec
            });
            ::nvx::log!("  st_ctime: {}s {}ns", { foo_tmp.st_ctim.tv_sec }, {
                foo_tmp.st_ctim.tv_nsec
            });
        },
        errno => {
            panic!("failed to get status of file {:?}: {:?}", path, errno);
        },
    }

    // Rename `foo.tmp` to `bar.tmp`.
    match fcntl::renameat(fcntl::AT_FDCWD, "foo.tmp", fcntl::AT_FDCWD, "bar.tmp") {
        0 => {
            ::nvx::log!("renamed file foo.tmp to bar.tmp");
        },
        errno => {
            panic!("failed to rename file foo.tmp to bar.tmp: {:?}", errno);
        },
    }

    // Get status of file named `bar.tmp`.
    let mut bar_tmp: stat = stat::default();
    match sys::stat::fstatat(fcntl::AT_FDCWD, "bar.tmp", &mut bar_tmp, 0) {
        0 => {
            ::nvx::log!("got status of file bar.tmp");
            ::nvx::log!("file statistics:");
            ::nvx::log!("  st_dev: {}", { bar_tmp.st_dev });
            ::nvx::log!("  st_ino: {}", { bar_tmp.st_ino });
            ::nvx::log!("  st_mode: {}", { bar_tmp.st_mode });
            ::nvx::log!("  st_nlink: {}", { bar_tmp.st_nlink });
            ::nvx::log!("  st_uid: {}", { bar_tmp.st_uid });
            ::nvx::log!("  st_gid: {}", { bar_tmp.st_gid });
            ::nvx::log!("  st_rdev: {}", { bar_tmp.st_rdev });
            ::nvx::log!("  st_size: {}", { bar_tmp.st_size });
            ::nvx::log!("  st_blksize: {}", { bar_tmp.st_blksize });
            ::nvx::log!("  st_blocks: {}", { bar_tmp.st_blocks });
            ::nvx::log!("  st_atime: {}s {}ns", { bar_tmp.st_atim.tv_sec }, {
                bar_tmp.st_atim.tv_nsec
            });
            ::nvx::log!("  st_mtime: {}s {}ns", { bar_tmp.st_mtim.tv_sec }, {
                bar_tmp.st_mtim.tv_nsec
            });
            ::nvx::log!("  st_ctime: {}s {}ns", { bar_tmp.st_ctim.tv_sec }, {
                bar_tmp.st_ctim.tv_nsec
            });
        },
        errno => {
            panic!("failed to get status of file bar.tmp: {:?}", errno);
        },
    }

    // Ensure that foo.tmp and bar.tmp are the same file.
    if foo_tmp.st_ino != bar_tmp.st_ino {
        panic!("foo.tmp and bar.tmp are not the same file");
    }

    // Unlink file named `foo.tmp`.
    match fcntl::unlinkat(fcntl::AT_FDCWD, "bar.tmp", 0) {
        0 => {
            ::nvx::log!("unlinked file foo.tmp");
        },
        errno => {
            panic!("failed to unlink file foo.tmp: {:?}", errno);
        },
    }

    venv::leave(env)?;
    ::nvx::log!("left environment {:?}", env);

    Ok(())
}
