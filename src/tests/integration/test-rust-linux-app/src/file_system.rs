// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::{
        atflags::AT_FDCWD,
        file_access_mode::O_RDWR,
        file_creation_flags::{
            O_CREAT,
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

/// Exercises unnamed pipes routed through vfsd.
///
/// Covers process-local pipe behavior: creation, `fstat` reporting
/// `S_IFIFO`, `lseek` returning `ESPIPE`, `fcntl` round-tripping `O_NONBLOCK`, non-blocking
/// `EAGAIN` on an empty read and on a full write, a byte-exact round trip, end-of-file after the
/// write end closes, and `EPIPE` after the read end closes. Cross-process blocking is covered by
/// the dedicated `pipe-test` binary.
fn test_pipe() {
    use ::sys::error::ErrorCode;
    use ::sysapi::{
        fcntl::{
            file_control_request::{
                F_GETFL,
                F_SETFL,
            },
            file_status_flags::O_NONBLOCK,
        },
        sys_stat::{
            self,
            file_type::S_ISFIFO,
        },
        unistd::file_seek::SEEK_SET,
    };

    /// Pipe buffer capacity in vfsd; filling this many bytes makes the pipe full.
    const PIPE_CAPACITY: usize = 64 * 1024;

    ::syslog::info!("testing unnamed pipes");

    let [read_fd, write_fd]: [i32; 2] = match unistd::pipe() {
        Ok(fds) => {
            ::syslog::info!("created pipe with fds ({}, {})", fds[0], fds[1]);
            fds
        },
        Err(e) => panic!("pipe() failed: {:?}", e),
    };

    // fstat() reports a FIFO with zero size.
    let mut st: sys_stat::stat = sys_stat::stat::default();
    match sys::stat::fstat(read_fd, &mut st) {
        Ok(()) => {
            if !S_ISFIFO(st.st_mode) {
                panic!("pipe fstat st_mode is not S_IFIFO (mode={:#o})", { st.st_mode });
            }
            if st.st_size != 0 {
                panic!("pipe fstat st_size should be 0 (size={})", { st.st_size });
            }
        },
        Err(e) => panic!("fstat on pipe failed: {:?}", e),
    }

    // lseek() on a pipe fails with ESPIPE.
    match unistd::lseek(read_fd, 0, SEEK_SET) {
        Ok(_) => panic!("lseek on a pipe should fail with ESPIPE"),
        Err(e) if e.code == ErrorCode::IllegalSeek => {},
        Err(e) => panic!("lseek on a pipe returned unexpected error: {:?}", e),
    }

    // fcntl(F_GETFL/F_SETFL) round-trips O_NONBLOCK.
    match fcntl::fcntl(read_fd, F_GETFL, None) {
        Ok(0) => {},
        Ok(fl) => panic!("initial F_GETFL should be 0 (got {})", fl),
        Err(e) => panic!("F_GETFL failed: {:?}", e),
    }
    if let Err(e) = fcntl::fcntl(read_fd, F_SETFL, Some(O_NONBLOCK)) {
        panic!("F_SETFL O_NONBLOCK failed: {:?}", e);
    }
    match fcntl::fcntl(read_fd, F_GETFL, None) {
        Ok(fl) if fl & O_NONBLOCK != 0 => {},
        Ok(fl) => panic!("F_GETFL should report O_NONBLOCK (got {:#x})", fl),
        Err(e) => panic!("F_GETFL failed: {:?}", e),
    }

    // A non-blocking read on an empty pipe returns EAGAIN.
    let mut one: [u8; 1] = [0u8; 1];
    match unistd::read(read_fd, &mut one) {
        Err(e) if e.code == ErrorCode::TryAgain => {},
        Ok(n) => panic!("nonblocking read on empty pipe should fail, got {} bytes", n),
        Err(e) => panic!("nonblocking read returned unexpected error: {:?}", e),
    }

    // Restore blocking mode for the round trip.
    if let Err(e) = fcntl::fcntl(read_fd, F_SETFL, Some(0)) {
        panic!("F_SETFL clear failed: {:?}", e);
    }

    // A write-then-read round trip preserves bytes in order.
    let mut write_buf: [u8; 128] = [0u8; 128];
    for (i, b) in write_buf.iter_mut().enumerate() {
        *b = i as u8;
    }
    match unistd::write(write_fd, &write_buf) {
        Ok(128) => {},
        Ok(n) => panic!("short pipe write (n={})", n),
        Err(e) => panic!("pipe write failed: {:?}", e),
    }
    let mut read_buf: [u8; 128] = [0u8; 128];
    match unistd::read(read_fd, &mut read_buf) {
        Ok(128) => {},
        Ok(n) => panic!("short pipe read (n={})", n),
        Err(e) => panic!("pipe read failed: {:?}", e),
    }
    if read_buf != write_buf {
        panic!("pipe round-trip data mismatch");
    }
    ::syslog::info!("pipe round trip preserved 128 bytes");

    // A non-blocking write to a full pipe returns EAGAIN. Fill the buffer with page-sized writes,
    // then expect the next write to fail.
    if let Err(e) = fcntl::fcntl(write_fd, F_SETFL, Some(O_NONBLOCK)) {
        panic!("F_SETFL O_NONBLOCK on write end failed: {:?}", e);
    }
    let page: [u8; 4096] = [0xABu8; 4096];
    let mut filled: usize = 0;
    while filled < PIPE_CAPACITY {
        match unistd::write(write_fd, &page) {
            Ok(0) => break,
            Ok(n) => filled += n as usize,
            Err(e) if e.code == ErrorCode::TryAgain => break,
            Err(e) => panic!("fill write failed: {:?}", e),
        }
    }
    match unistd::write(write_fd, &page[..1]) {
        Err(e) if e.code == ErrorCode::TryAgain => {},
        Ok(n) => panic!("nonblocking write to full pipe should fail, wrote {}", n),
        Err(e) => panic!("nonblocking full-pipe write returned unexpected error: {:?}", e),
    }
    ::syslog::info!("nonblocking write to a full pipe returned EAGAIN (filled {} bytes)", filled);

    // Drain everything so the EOF check below sees an empty pipe.
    let mut sink: [u8; 4096] = [0u8; 4096];
    let mut drained: usize = 0;
    while drained < filled {
        match unistd::read(read_fd, &mut sink) {
            Ok(0) => break,
            Ok(n) => drained += n as usize,
            Err(e) => panic!("drain read failed: {:?}", e),
        }
    }
    if let Err(e) = fcntl::fcntl(write_fd, F_SETFL, Some(0)) {
        panic!("F_SETFL clear on write end failed: {:?}", e);
    }

    // Closing the write end makes a blocking read return end-of-file.
    if let Err(e) = unistd::close(write_fd) {
        panic!("close write end failed: {:?}", e);
    }
    match unistd::read(read_fd, &mut one) {
        Ok(0) => {},
        Ok(n) => panic!("read after writer close should be EOF, got {} bytes", n),
        Err(e) => panic!("EOF read failed: {:?}", e),
    }
    if let Err(e) = unistd::close(read_fd) {
        panic!("close read end failed: {:?}", e);
    }
    ::syslog::info!("read returned EOF after the write end closed");

    // Writing to a pipe whose read end is closed fails with EPIPE.
    let [read_fd2, write_fd2]: [i32; 2] = match unistd::pipe() {
        Ok(fds) => fds,
        Err(e) => panic!("second pipe() failed: {:?}", e),
    };
    if let Err(e) = unistd::close(read_fd2) {
        panic!("close read end (2) failed: {:?}", e);
    }
    match unistd::write(write_fd2, &write_buf[..1]) {
        Err(e) if e.code == ErrorCode::BrokenPipe => {},
        Ok(n) => panic!("write with no readers should fail with EPIPE, wrote {}", n),
        Err(e) => panic!("EPIPE write returned unexpected error: {:?}", e),
    }
    if let Err(e) = unistd::close(write_fd2) {
        panic!("close write end (2) failed: {:?}", e);
    }
    ::syslog::info!("write returned EPIPE after the read end closed");

    // Cross-process blocking transfer (suspend/revive) via fork().
    test_pipe_blocking_fork();

    // Peer-exit wakeups: a parked reader observes EOF when the last writer process exits, and a
    // parked writer observes EPIPE when the last reader process exits.
    test_pipe_eof_on_writer_exit();
    test_pipe_epipe_on_reader_exit();

    ::syslog::info!("pipe test passed");
}

/// Streams more than one pipe buffer's worth of data from a parent to a forked child, forcing the
/// writer to block until the reader drains and exercising vfsd's suspend/revive path end-to-end.
///
/// The parent writes a deterministic ramp larger than the pipe capacity; the child drains and
/// verifies it, then reports a pass/fail verdict over IPC. The child exits without returning to the
/// shared flow, so only the parent continues.
fn test_pipe_blocking_fork() {
    use ::sys::{
        ipc::{
            Message,
            MessageReceiver,
            MessageSender,
            MessageType,
        },
        kcall::{
            fork,
            ipc,
            pm,
        },
        pm::{
            ProcessIdentifier,
            ThreadIdentifier,
        },
    };

    /// Total bytes streamed; exceeds the 64 KiB pipe capacity to force the writer to block.
    const TOTAL: usize = 96 * 1024;
    /// Bytes transferred per read/write iteration.
    const CHUNK: usize = 4096;
    /// Bound on first-read retries absorbing the asynchronous fork-clone descriptor duplication.
    const MAX_FIRST_READ_ATTEMPTS: u32 = 4096;
    /// Child exit status when the transfer verifies successfully.
    const CHILD_EXIT_OK: i32 = 0;
    /// Child exit status when the transfer fails.
    const CHILD_EXIT_FAIL: i32 = 1;

    ::syslog::info!("testing cross-process blocking pipe transfer via fork()");

    let parent_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(e) => panic!("getpid failed: {:?}", e),
    };

    let [read_fd, write_fd]: [i32; 2] = match unistd::pipe() {
        Ok(fds) => fds,
        Err(e) => panic!("pipe() for fork test failed: {:?}", e),
    };

    let child_pid: ProcessIdentifier = match fork::__kcall_fork() {
        Ok(pid) => pid,
        Err(e) => panic!("fork() failed: {:?}", e),
    };

    // Child path: drain and verify the stream, report the verdict, and exit.
    if child_pid == ProcessIdentifier::from(0) {
        let my_pid: ProcessIdentifier = match pm::getpid_uncached() {
            Ok(pid) => pid,
            Err(_) => {
                let _ = pm::__kcall_exit(CHILD_EXIT_FAIL);
                loop {
                    ::core::hint::spin_loop();
                }
            },
        };
        let passed: bool = pipe_child_drain(read_fd, TOTAL, CHUNK, MAX_FIRST_READ_ATTEMPTS);

        // Report the verdict to the parent.
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        payload[0] = u8::from(passed);
        let reply: Message = Message::new(
            MessageSender::new(my_pid, ThreadIdentifier::NONE),
            MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
            MessageType::Ipc,
            None,
            payload,
        );
        let _ = ipc::__kcall_send(&reply);

        let status: i32 = if passed {
            CHILD_EXIT_OK
        } else {
            CHILD_EXIT_FAIL
        };
        let _ = pm::__kcall_exit(status);
        loop {
            ::core::hint::spin_loop();
        }
    }

    // Parent path: write the ramp, blocking whenever the pipe fills, then collect the verdict.
    let mut page: [u8; CHUNK] = [0u8; CHUNK];
    let mut offset: usize = 0;
    while offset < TOTAL {
        let len: usize = core::cmp::min(CHUNK, TOTAL - offset);
        for (i, b) in page[..len].iter_mut().enumerate() {
            *b = ((offset + i) & 0xFF) as u8;
        }
        match unistd::write(write_fd, &page[..len]) {
            Ok(0) => panic!("parent pipe write made no progress at offset {}", offset),
            Ok(n) => offset += n as usize,
            Err(e) => panic!("parent pipe write failed at offset {}: {:?}", offset, e),
        }
    }

    // Receive the child's verdict (happens-after the child has drained the whole stream).
    let reply: Message = match ipc::__kcall_recv() {
        Ok(m) => m,
        Err(e) => panic!("parent failed to receive child verdict: {:?}", e),
    };
    if reply.payload[0] != 1 {
        panic!("child reported a blocking pipe transfer failure");
    }

    let _ = unistd::close(read_fd);
    let _ = unistd::close(write_fd);
    ::syslog::info!("cross-process blocking pipe transfer of {} bytes succeeded", TOTAL);
}

/// Drains exactly `total` bytes from `read_fd` and verifies they form the expected ramp.
///
/// The first read is retried up to `max_first_attempts` times to absorb the window in which a
/// freshly forked child's descriptors have not yet been duplicated into vfsd. Returns whether the
/// full stream was received and validated.
fn pipe_child_drain(read_fd: i32, total: usize, chunk: usize, max_first_attempts: u32) -> bool {
    let mut buf: [u8; 4096] = [0u8; 4096];
    let mut offset: usize = 0;
    let mut first_read_done: bool = false;
    let mut attempts: u32 = 0;

    while offset < total {
        let len: usize = core::cmp::min(chunk, total - offset);
        match unistd::read(read_fd, &mut buf[..len]) {
            // Premature EOF while the writer is still open: a failure.
            Ok(0) => return false,
            Ok(n) => {
                first_read_done = true;
                let n: usize = n as usize;
                for (i, b) in buf[..n].iter().enumerate() {
                    if *b != ((offset + i) & 0xFF) as u8 {
                        return false;
                    }
                }
                offset += n;
            },
            // Absorb the fork-clone duplication window on the very first read only.
            Err(_) if !first_read_done && attempts < max_first_attempts => {
                attempts += 1;
            },
            Err(_) => return false,
        }
    }

    offset == total
}

/// Verifies that a reader parked on an empty pipe observes EOF when the last writer *process*
/// exits, exercising vfsd's process-exit reclamation path (`wake_all_readers_eof`).
///
/// The parent keeps the read end and survives; the forked child keeps the write end and exits
/// without writing. The parent closes its own write end first, so the child holds the sole
/// remaining writer reference at exit. Whether the parent parks before the child exits or reads
/// afterward, the observable result is the same: a clean EOF.
fn test_pipe_eof_on_writer_exit() {
    use ::sys::{
        ipc::{
            Message,
            MessageReceiver,
            MessageSender,
            MessageType,
        },
        kcall::{
            fork,
            ipc,
            pm,
        },
        pm::{
            ProcessIdentifier,
            ThreadIdentifier,
        },
    };

    /// Bound on close retries absorbing the asynchronous fork-clone descriptor duplication.
    const MAX_SETUP_ATTEMPTS: u32 = 4096;
    /// Payload byte signalling that the child has finished setting up its descriptors.
    const READY: u8 = 1;
    /// Child exit status when setup succeeds.
    const CHILD_EXIT_OK: i32 = 0;
    /// Child exit status when setup fails.
    const CHILD_EXIT_FAIL: i32 = 1;

    ::syslog::info!("testing pipe EOF wakeup when the last writer process exits");

    let parent_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(e) => panic!("getpid failed: {:?}", e),
    };

    let [read_fd, write_fd]: [i32; 2] = match unistd::pipe() {
        Ok(fds) => fds,
        Err(e) => panic!("pipe() for EOF-on-exit test failed: {:?}", e),
    };

    let child_pid: ProcessIdentifier = match fork::__kcall_fork() {
        Ok(pid) => pid,
        Err(e) => panic!("fork() failed: {:?}", e),
    };

    // Child path: drop the unused read end, signal readiness, and exit holding the write end.
    if child_pid == ProcessIdentifier::from(0) {
        let my_pid: ProcessIdentifier = match pm::getpid_uncached() {
            Ok(pid) => pid,
            Err(_) => {
                let _ = pm::__kcall_exit(CHILD_EXIT_FAIL);
                loop {
                    ::core::hint::spin_loop();
                }
            },
        };

        // A successful close proves the fork-clone landed and drops the child's reader reference.
        let status: i32 = if pipe_close_after_fork_clone(read_fd, MAX_SETUP_ATTEMPTS) {
            let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
            payload[0] = READY;
            let reply: Message = Message::new(
                MessageSender::new(my_pid, ThreadIdentifier::NONE),
                MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
                MessageType::Ipc,
                None,
                payload,
            );
            let _ = ipc::__kcall_send(&reply);
            CHILD_EXIT_OK
        } else {
            CHILD_EXIT_FAIL
        };

        // Exit while holding the write end: this drops the final writer reference.
        let _ = pm::__kcall_exit(status);
        loop {
            ::core::hint::spin_loop();
        }
    }

    // Parent path: wait for the child to be ready, then become the sole reader and read EOF.
    let ready: Message = match ipc::__kcall_recv() {
        Ok(m) => m,
        Err(e) => panic!("parent failed to receive child readiness: {:?}", e),
    };
    if ready.payload[0] != READY {
        panic!("child failed to set up the writer end for the EOF test");
    }

    // Drop the parent's own write reference so the child holds the only remaining writer.
    if let Err(e) = unistd::close(write_fd) {
        panic!("parent failed to close its write end: {:?}", e);
    }

    // The child exits holding the last writer reference, so this read must observe a clean EOF,
    // whether it parks first and is revived or runs after the child has already exited.
    let mut buf: [u8; 64] = [0u8; 64];
    match unistd::read(read_fd, &mut buf) {
        Ok(0) => {},
        Ok(n) => panic!("reader expected EOF but received {} bytes", n),
        Err(e) => panic!("reader expected EOF but read failed: {:?}", e),
    }

    let _ = unistd::close(read_fd);
    ::syslog::info!("reader observed EOF after the last writer process exited");
}

/// Verifies that a writer parked on a full pipe observes EPIPE when the last reader *process*
/// exits, exercising vfsd's process-exit reclamation path (`fail_all_writers_epipe`).
///
/// The parent keeps the write end and survives; the forked child keeps the read end and exits
/// without draining. The parent closes its own read end first, so the child holds the sole
/// remaining reader reference at exit. The parent fills the pipe until it blocks; the child's
/// exit then fails the parked writer with EPIPE.
fn test_pipe_epipe_on_reader_exit() {
    use ::sys::{
        error::ErrorCode,
        ipc::{
            Message,
            MessageReceiver,
            MessageSender,
            MessageType,
        },
        kcall::{
            fork,
            ipc,
            pm,
        },
        pm::{
            ProcessIdentifier,
            ThreadIdentifier,
        },
    };

    /// Bound on close retries absorbing the asynchronous fork-clone descriptor duplication.
    const MAX_SETUP_ATTEMPTS: u32 = 4096;
    /// Payload byte signalling that the child has finished setting up its descriptors.
    const READY: u8 = 1;
    /// Bytes written per iteration while filling the pipe.
    const CHUNK: usize = 4096;
    /// Upper bound on bytes written before the writer is expected to have failed; comfortably
    /// exceeds the 64 KiB pipe capacity so a healthy pipe parks well before this limit.
    const MAX_WRITE: usize = 256 * 1024;
    /// Child exit status when setup succeeds.
    const CHILD_EXIT_OK: i32 = 0;
    /// Child exit status when setup fails.
    const CHILD_EXIT_FAIL: i32 = 1;

    ::syslog::info!("testing pipe EPIPE wakeup when the last reader process exits");

    let parent_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(e) => panic!("getpid failed: {:?}", e),
    };

    let [read_fd, write_fd]: [i32; 2] = match unistd::pipe() {
        Ok(fds) => fds,
        Err(e) => panic!("pipe() for EPIPE-on-exit test failed: {:?}", e),
    };

    let child_pid: ProcessIdentifier = match fork::__kcall_fork() {
        Ok(pid) => pid,
        Err(e) => panic!("fork() failed: {:?}", e),
    };

    // Child path: drop the unused write end, signal readiness, and exit holding the read end.
    if child_pid == ProcessIdentifier::from(0) {
        let my_pid: ProcessIdentifier = match pm::getpid_uncached() {
            Ok(pid) => pid,
            Err(_) => {
                let _ = pm::__kcall_exit(CHILD_EXIT_FAIL);
                loop {
                    ::core::hint::spin_loop();
                }
            },
        };

        // A successful close proves the fork-clone landed and drops the child's writer reference.
        let status: i32 = if pipe_close_after_fork_clone(write_fd, MAX_SETUP_ATTEMPTS) {
            let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
            payload[0] = READY;
            let reply: Message = Message::new(
                MessageSender::new(my_pid, ThreadIdentifier::NONE),
                MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
                MessageType::Ipc,
                None,
                payload,
            );
            let _ = ipc::__kcall_send(&reply);
            CHILD_EXIT_OK
        } else {
            CHILD_EXIT_FAIL
        };

        // Exit while holding the read end without draining: drops the final reader reference.
        let _ = pm::__kcall_exit(status);
        loop {
            ::core::hint::spin_loop();
        }
    }

    // Parent path: wait for the child to be ready, then become the sole writer and fill the pipe.
    let ready: Message = match ipc::__kcall_recv() {
        Ok(m) => m,
        Err(e) => panic!("parent failed to receive child readiness: {:?}", e),
    };
    if ready.payload[0] != READY {
        panic!("child failed to set up the reader end for the EPIPE test");
    }

    // Drop the parent's own read reference so the child holds the only remaining reader.
    if let Err(e) = unistd::close(read_fd) {
        panic!("parent failed to close its read end: {:?}", e);
    }

    // Fill the pipe until the writer blocks; the child's exit then fails the parked writer with
    // EPIPE. If the child exits before the pipe fills, the write fails with EPIPE immediately.
    let page: [u8; CHUNK] = [0xABu8; CHUNK];
    let mut written: usize = 0;
    let broken: bool = loop {
        match unistd::write(write_fd, &page) {
            Ok(0) => panic!("writer made no progress after writing {} bytes", written),
            Ok(n) => {
                written += n as usize;
                if written >= MAX_WRITE {
                    break false;
                }
            },
            Err(e) if e.code == ErrorCode::BrokenPipe => break true,
            Err(e) => panic!("writer failed with an unexpected error: {:?}", e),
        }
    };

    if !broken {
        panic!("writer wrote {} bytes without observing EPIPE after the reader exited", written);
    }

    let _ = unistd::close(write_fd);
    ::syslog::info!("writer observed EPIPE after the last reader process exited");
}

/// Closes `fd` from a freshly forked child, retrying to absorb the window during which the child's
/// descriptors have not yet been duplicated into vfsd.
///
/// A successful close proves the fork-clone has landed (the descriptor now resolves) and drops the
/// child's reference to the unused pipe end. Returns whether the close eventually succeeded.
fn pipe_close_after_fork_clone(fd: i32, max_attempts: u32) -> bool {
    let mut attempts: u32 = 0;
    loop {
        match unistd::close(fd) {
            Ok(()) => return true,
            Err(_) if attempts < max_attempts => attempts += 1,
            Err(_) => return false,
        }
    }
}
