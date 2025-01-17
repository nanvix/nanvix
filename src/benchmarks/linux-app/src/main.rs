// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::nvx::sys::error::Error;
use ::posix::{
    fcntl,
    ffi::c_int,
    netinet::in_::{
        in_addr,
        sockaddr_in,
    },
    sys::{
        self,
        socket::{
            sockaddr,
            socklen_t,
        },
        stat::stat,
        times,
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    let env: VirtualEnvironmentIdentifier = venv::join(VirtualEnvironmentIdentifier::NEW)?;
    ::nvx::log!("joined environment {:?}", env);

    match times::times(None) {
        Ok(clock) => {
            ::nvx::log!("times() returned {}", clock);
        },
        Err(e) => {
            panic!("times() failed: {:?}", e);
        },
    }

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

    match time::clock_gettime(CLOCK_MONOTONIC, Some(&mut tp)) {
        Ok(()) => {
            ::nvx::log!("clock time: {}s {}ns", { tp.tv_sec }, { tp.tv_nsec });
        },
        e => {
            panic!("failed to get clock time: {:?}", e);
        },
    }

    // Try to get PID
    match unistd::getpid() {
        Ok(pid) => {
            ::nvx::log!("got PID {:#?}", pid);
        },
        Err(err) => {
            panic!("failed to get PID: {:?}", err);
        },
    };

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

    // Fill bytes [128, 192] with ones using partial write.
    let buffer: [u8; 64] = [1; 64];
    match unistd::pwrite(fd, buffer.as_ptr(), buffer.len() as size_t, 128) {
        64 => {
            ::nvx::log!("wrote 64 bytes to file foo.tmp");
        },
        errno => {
            panic!("failed to write 64 bytes to file foo.tmp: {:?}", errno);
        },
    }

    // Fill bytes [192..256] with ones using offset partial write.
    let buffer: [u8; 64] = [1; 64];
    let iov: [uio::iovec; 2] = [
        uio::iovec {
            iov_base: buffer.as_ptr() as *mut u8,
            iov_len: 32,
        },
        uio::iovec {
            iov_base: unsafe { buffer.as_ptr().add(32) } as *mut u8,
            iov_len: 32,
        },
    ];
    match uio::pwritev(fd, iov.as_ptr(), iov.len() as i32, 192) {
        64 => {
            ::nvx::log!("wrote 64 bytes to file foo.tmp");
        },
        errno => {
            panic!("failed to write 64 bytes to file foo.tmp: {:?}", errno);
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

    // Check if first 64 bytes are filled with ones using partial reads.
    let mut buffer: [u8; 64] = [0; 64];
    match unistd::pread(fd, buffer.as_mut_ptr(), buffer.len() as size_t, 0) {
        64 => {
            ::nvx::log!("read 64 bytes from file foo.tmp");
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
            ::nvx::log!("read 64 bytes from file foo.tmp");
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
            ::nvx::log!("seek file foo.tmp to 128 bytes");
        },
        offset => {
            panic!("failed to seek file foo.tmp to 128 bytes: {:?}", offset);
        },
    }

    // Check if bytes [128..256] are filled with ones using vectored i/o operations.
    let mut buffer: [u8; 128] = [0; 128];
    let iov: [uio::iovec; 2] = [
        uio::iovec {
            iov_base: buffer.as_mut_ptr(),
            iov_len: 64,
        },
        uio::iovec {
            iov_base: unsafe { buffer.as_mut_ptr().add(64) },
            iov_len: 64,
        },
    ];
    match uio::readv(fd, iov.as_ptr(), iov.len() as i32) {
        128 => {
            ::nvx::log!("read 128 bytes from file foo.tmp");
            (0..128).for_each(|i| {
                if buffer[i] != 1 {
                    panic!("file foo.tmp is not filled with ones");
                }
            });
        },
        errno => {
            panic!("failed to read 128 bytes from file foo.tmp: {:?}", errno);
        },
    }

    // Check if [256..512] bytes are filled with ones.
    let mut buffer: [u8; 256] = [0; 256];
    match unistd::read(fd, buffer.as_mut_ptr(), buffer.len() as size_t) {
        256 => {
            ::nvx::log!("read 256 bytes from file foo.tmp");
            (0..256).for_each(|i| {
                if buffer[i] != 1 {
                    panic!("file foo.tmp is not filled with ones");
                }
            });
        },
        errno => {
            panic!("failed to read 256 bytes from file foo.tmp: {:?}", errno);
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
        Ok(()) => {
            ::nvx::log!("truncated file foo.tmp to 512 bytes");
        },
        Err(e) => {
            panic!("failed to truncate file foo.tmp to 512 bytes ({:?})", e);
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
        Ok(()) => {
            ::nvx::log!("synchronized file foo.tmp with storage device");
        },
        Err(e) => {
            panic!("failed to synchronize file foo.tmp with storage device ({:?})", e);
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

    // Change owner of file.
    match unistd::fchown(fd, st.st_uid, st.st_gid) {
        Ok(()) => {
            ::nvx::log!("changed owner of file foo.tmp");
        },
        Err(e) => {
            panic!("failed to change owner of file foo.tmp: {:?}", e);
        },
    };

    // Change file mode.
    match unistd::fchmod(fd, fcntl::S_IRUSR | fcntl::S_IWUSR) {
        Ok(()) => {
            ::nvx::log!("changed file mode of file foo.tmp");
        },
        Err(e) => {
            panic!("failed to change file mode of file foo.tmp: {:?}", e);
        },
    };

    // Get file access mode and the file status flags.
    let flags: i32 = match fcntl::fcntl(fd, fcntl::F_GETFL, 0) {
        flags if flags >= 0 => {
            ::nvx::log!("got file access mode and file status flags {}", flags);
            flags
        },
        errno => {
            panic!("failed to get file access mode and file status flags: {:?}", errno);
        },
    };
    // Check if file is open for reading and writing.
    if (flags & fcntl::O_ACCMODE) != fcntl::O_RDWR {
        panic!("file is not open for reading and writing");
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
            ::nvx::log!("updated access time of file foo.tmp");
        },
        errno => {
            panic!("failed to update access time of file foo.tmp: {:?}", errno);
        },
    }

    // Get status of file named `foo.tmp`.
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

    // Ensure time of last access was updated.
    if st.st_atim.tv_sec != 1 {
        panic!("access time of file bar.tmp was not updated");
    }
    if st.st_atim.tv_nsec != 1 {
        panic!("access time of file bar.tmp was not updated");
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

    // Change owner of file.
    match fcntl::fchownat(fcntl::AT_FDCWD, "foo.tmp", st.st_uid, st.st_gid, 0) {
        Ok(()) => {
            ::nvx::log!("changed owner of file foo.tmp");
        },
        Err(e) => {
            panic!("failed to change owner of file foo.tmp: {:?}", e);
        },
    };

    // Change file mode.
    match fcntl::fchmodat(fcntl::AT_FDCWD, "foo.tmp", fcntl::S_IRUSR | fcntl::S_IWUSR, 0) {
        Ok(()) => {
            ::nvx::log!("changed file mode of file foo.tmp");
        },
        Err(e) => {
            panic!("failed to change file mode of file foo.tmp: {:?}", e);
        },
    };

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

    // Create a symbolic link to `bar.tmp`.
    match fcntl::symlinkat("bar.tmp", fcntl::AT_FDCWD, "baz.tmp") {
        0 => {
            ::nvx::log!("created symbolic link baz.tmp to bar.tmp");
        },
        errno => {
            panic!("failed to create symbolic link baz.tmp to bar.tmp: {:?}", errno);
        },
    }

    // Readlink file named `baz.tmp`.
    let mut buffer: [u8; 512] = [0; 512];
    match fcntl::readlinkat(fcntl::AT_FDCWD, "baz.tmp", &mut buffer) {
        len if len >= 0 => {
            ::nvx::log!("read link baz.tmp");
            // Print.
            let mut i: usize = 0;
            while buffer[i] != 0 {
                ::nvx::log!("{}", buffer[i] as char);
                i += 1;
            }
        },
        errno => {
            panic!("failed to read link baz.tmp: {:?}", errno);
        },
    }

    // Unlink file named `baz.tmp`.
    match fcntl::unlinkat(fcntl::AT_FDCWD, "baz.tmp", 0) {
        0 => {
            ::nvx::log!("unlinked file baz.tmp");
        },
        errno => {
            panic!("failed to unlink file baz.tmp: {:?}", errno);
        },
    }

    // Create a hard link to `bar.tmp`.
    match unistd::linkat(fcntl::AT_FDCWD, "bar.tmp", fcntl::AT_FDCWD, "baz.tmp", 0) {
        0 => {
            ::nvx::log!("created hard link baz.tmp to bar.tmp");
        },
        errno => {
            panic!("failed to create hard link baz.tmp to bar.tmp: {:?}", errno);
        },
    }

    // Unlink file named `baz.tmp`.
    match fcntl::unlinkat(fcntl::AT_FDCWD, "baz.tmp", 0) {
        0 => {
            ::nvx::log!("unlinked file baz.tmp");
        },
        errno => {
            panic!("failed to unlink file baz.tmp: {:?}", errno);
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

    // Update access time of file named `bar.tmp`.
    let times: [timespec; 2] = [
        timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
    ];
    match sys::stat::utimensat(fcntl::AT_FDCWD, "bar.tmp", times, 0) {
        0 => {
            ::nvx::log!("updated access time of file bar.tmp");
        },
        errno => {
            panic!("failed to update access time of file bar.tmp: {:?}", errno);
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

    // Ensure time of last access was updated.
    if bar_tmp.st_atim.tv_sec != 0 {
        panic!("access time of file bar.tmp was not updated");
    }
    if bar_tmp.st_atim.tv_nsec != 0 {
        panic!("access time of file bar.tmp was not updated");
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

    // Create directory named `foo`.
    match fcntl::mkdirat(fcntl::AT_FDCWD, "foo", fcntl::S_IRUSR | fcntl::S_IWUSR | fcntl::S_IXUSR) {
        0 => {
            ::nvx::log!("created directory foo");
        },
        errno => {
            panic!("failed to create directory foo: {:?}", errno);
        },
    }

    // Remove directory named `foo`.
    match fcntl::unlinkat(fcntl::AT_FDCWD, "foo", fcntl::AT_REMOVEDIR) {
        0 => {
            ::nvx::log!("removed directory foo");
        },
        errno => {
            panic!("failed to remove directory foo: {:?}", errno);
        },
    }

    // Create a socket.
    let domain: i32 = sys::socket::AF_INET as i32;
    let typ: i32 = sys::socket::SOCK_STREAM;
    let sockfd: i32 = match sys::socket::socket(domain, typ, 0) {
        sockfd if sockfd >= 0 => {
            ::nvx::log!("created socket with fd {}", sockfd);
            sockfd
        },
        errno => {
            panic!("failed to create socket: {:?}", errno);
        },
    };

    // Bind socket to address to 127.0.0.1:8080.
    let sockaddr_in: sockaddr_in = sockaddr_in {
        sin_family: sys::socket::AF_INET,
        sin_port: u16::to_be(8080),
        sin_addr: in_addr {
            s_addr: u32::from_be_bytes([127, 0, 0, 1]).to_be(),
        },
        sin_zero: [0; 8],
    };

    // TODO: test case for connect().

    // TODO: test case for accept().

    match sys::socket::bind(
        sockfd,
        unsafe {
            mem::transmute::<&posix::netinet::in_::sockaddr_in, &posix::sys::socket::sockaddr>(
                &sockaddr_in,
            )
        },
        core::mem::size_of::<sys::socket::sockaddr>() as socklen_t,
    ) {
        0 => {
            ::nvx::log!("bound socket to address");
        },
        errno => {
            panic!("failed to bind socket to address: {:?}", errno);
        },
    }

    // Listen for connections on socket.
    match sys::socket::listen(sockfd, 0) {
        0 => {
            ::nvx::log!("listening for connections on socket");
        },
        errno => {
            panic!("failed to listen for connections on socket: {:?}", errno);
        },
    }

    // Close socket.
    match unistd::close(sockfd) {
        0 => {
            ::nvx::log!("closed socket");
        },
        errno => {
            panic!("failed to close socket: {:?}", errno);
        },
    }

    // Create a pair of connected sockets.
    let mut socket_fds: [c_int; 2] = [-1; 2];

    match sys::socket::socketpair(
        sys::socket::AF_UNIX as c_int,
        sys::socket::SOCK_STREAM,
        0,
        &mut socket_fds,
    ) {
        Ok(()) => {
            ::nvx::log!(
                "created pair of connected sockets with fds {} and {}",
                socket_fds[0],
                socket_fds[1]
            );
        },
        Err(errno) => {
            panic!("failed to create pair of connected sockets: {:?}", errno);
        },
    }

    // Get name of the local socket.
    let mut sockaddr_self: [sockaddr; 2] = unsafe { mem::zeroed() };
    let mut addrlen_self: [socklen_t; 2] = [0; 2];
    for i in 0..2 {
        match sys::socket::getsockname(socket_fds[i], &mut sockaddr_self[i], &mut addrlen_self[i]) {
            Ok(()) => {
                ::nvx::log!("sockfd {:?} is bound to {:?}", socket_fds[i], sockaddr_self[i]);
            },
            errno => {
                panic!("failed to get local name of connection: {:?}", errno);
            },
        }
    }

    // Get name of the peer socket.
    let mut sockaddr_peer: [sockaddr; 2] = unsafe { mem::zeroed() };
    let mut addrlen_peer: [socklen_t; 2] = [0; 2];
    for i in (0..2).rev() {
        match sys::socket::getpeername(socket_fds[i], &mut sockaddr_peer[i], &mut addrlen_peer[i]) {
            Ok(()) => {
                ::nvx::log!(
                    "sockfd {:?} is connected to peer {:?}",
                    socket_fds[i],
                    sockaddr_peer[i]
                );
            },
            errno => {
                panic!("failed to get peer name of connection: {:?}", errno);
            },
        }
    }

    // Check if local and peer names are the same.
    for i in 0..2 {
        if addrlen_self[i] != addrlen_peer[i] {
            panic!("local and peer names are not the same");
        }
        if sockaddr_self[i] != sockaddr_peer[i] {
            panic!("local and peer names are not the same");
        }
    }

    let mut buffer: [u8; 32] = [1; 32];

    // Send message.
    match sys::socket::send(socket_fds[0], buffer.as_ptr(), buffer.len() as size_t, 0) {
        len if len >= 0 => {
            ::nvx::log!("sent {} bytes to connection", len);
        },
        errno => {
            panic!("failed to send message to connection: {:?}", errno);
        },
    }

    // Receive message from connection.
    match sys::socket::recv(socket_fds[1], buffer.as_mut_ptr(), buffer.len() as size_t, 0) {
        len if len >= 0 => {
            ::nvx::log!("received {} bytes from connection", len);
        },
        errno => {
            panic!("failed to receive message from connection: {:?}", errno);
        },
    }

    // Sanity check message contents.
    (0..32).for_each(|i| {
        if buffer[i] != 1 {
            panic!("message contents are not correct");
        }
    });

    // Disallow send and receive operations.
    for socketfd in &socket_fds {
        match sys::socket::shutdown(*socketfd, sys::socket::SHUT_RDWR) {
            0 => {
                ::nvx::log!("disallowed send and receive operations on connection");
            },
            errno => {
                panic!("failed to disallow send and receive operations on connection: {:?}", errno);
            },
        }
    }

    // Close sockets.
    match unistd::close(socket_fds[0]) {
        0 => {
            ::nvx::log!("closed socket with fd {}", socket_fds[0]);
        },
        errno => {
            panic!("failed to close socket with fd {}: {:?}", socket_fds[0], errno);
        },
    }

    match unistd::close(socket_fds[1]) {
        0 => {
            ::nvx::log!("closed socket with fd {}", socket_fds[1]);
        },
        errno => {
            panic!("failed to close socket with fd {}: {:?}", socket_fds[1], errno);
        },
    }

    venv::leave(env)?;
    ::nvx::log!("left environment {:?}", env);

    Ok(())
}
