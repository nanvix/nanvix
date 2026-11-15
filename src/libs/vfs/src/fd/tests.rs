// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::process::CURRENT_PID;
use ::sysapi::fcntl::{
    file_access_mode,
    file_descriptor_flags::{
        FD_CLOEXEC,
        FD_CLOFORK,
    },
};

// -- Device namespace tests -------------------------------------------------

/// Tests pathname metadata and enumeration for the synthetic `/dev` directory.
#[test]
fn devfs_directory() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7402);
    set_current_process(pid);

    let mut directory: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    let mut missing: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_stat("/dev", &mut directory).expect("stat /dev");
    assert_eq!(directory.st_mode & file_type::S_IFMT, file_type::S_IFDIR);
    assert_eq!(vfs_stat("/dev/missing", &mut missing), Err(Fat32Error::NotFound));
    assert_eq!(vfs_rmdir("/dev"), Err(Fat32Error::PermissionDenied));
    assert_eq!(vfs_unlink("/dev/missing"), Err(Fat32Error::NotFound));
    assert_eq!(vfs_mkdir("/dev"), Err(Fat32Error::AlreadyExists));
    assert_eq!(
        vfs_open(
            "/dev",
            file_access_mode::O_RDONLY | file_creation_flags::O_CREAT | file_creation_flags::O_EXCL,
        ),
        Err(Fat32Error::AlreadyExists)
    );
    assert_eq!(vfs_open("/dev", file_access_mode::O_WRONLY), Err(Fat32Error::NotAFile));
    assert_eq!(
        vfs_open("/dev", file_access_mode::O_WRONLY | file_creation_flags::O_TRUNC,),
        Err(Fat32Error::NotAFile)
    );
    assert_eq!(vfs_mkdir("/dev/hidden"), Err(Fat32Error::PermissionDenied));
    assert_eq!(vfs_mkdir("/dev/hidden/child"), Err(Fat32Error::NotFound));
    assert_eq!(
        vfs_open("/dev/hidden", file_access_mode::O_WRONLY | file_creation_flags::O_CREAT,),
        Err(Fat32Error::PermissionDenied)
    );
    assert_eq!(
        vfs_open("/dev/hidden/child", file_access_mode::O_WRONLY | file_creation_flags::O_CREAT,),
        Err(Fat32Error::NotFound)
    );
    assert!(crate::file::file_raw_region("/dev/hidden").is_none());
    let dev_path = filesystem::VfsResolvedPath::new(
        AnchoredPath::new(AT_FDCWD, String::from("/dev")).expect("anchor /dev"),
    )
    .expect("resolve /dev");
    assert_eq!(
        filesystem::set_times(&dev_path, Some(FAT_EPOCH_SECS), None),
        Err(Fat32Error::PermissionDenied)
    );
    let times: [timespec; 2] = [
        timespec {
            tv_sec: FAT_EPOCH_SECS,
            tv_nsec: 0,
        },
        timespec {
            tv_sec: FAT_EPOCH_SECS,
            tv_nsec: 0,
        },
    ];
    assert_eq!(
        vfs_utimensat(::sysapi::fcntl::atflags::AT_FDCWD, "/dev", &times, 0),
        Err(Fat32Error::PermissionDenied)
    );
    assert_eq!(vfs_rename("/dev", "/outside-dev"), Err(Fat32Error::PermissionDenied));

    let root: Vec<filesystem::DirEntry> = vfs_readdir("/").expect("read root");
    let dev: Vec<filesystem::DirEntry> = vfs_readdir("/dev").expect("read /dev");
    assert_eq!(root.iter().map(|entry| entry.name()).collect::<Vec<_>>(), ["dev"]);
    assert_eq!(root[0].inode(), directory.st_ino);
    assert_eq!(
        dev.iter().map(|entry| entry.name()).collect::<Vec<_>>(),
        ["null", "tty", "console", "stdin", "stdout", "stderr"]
    );
    assert!(dev[..3].iter().all(|entry| entry.is_character_device()));
    assert!(dev[3..].iter().all(|entry| entry.is_descriptor_alias()));

    let fd: c_int = vfs_open("/dev", file_access_mode::O_RDONLY).expect("open /dev");
    let mut opened: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_fstat(fd, &mut opened).expect("fstat /dev");
    assert_eq!(opened.st_dev, directory.st_dev);
    assert_eq!(opened.st_ino, directory.st_ino);
    let dents: Vec<::sysapi::dirent::posix_dent> = vfs_getdents(fd, 1).expect("getdents /dev");
    assert_eq!(dents.len(), 1);
    assert_eq!(dents[0].d_type, ::sysapi::dirent::dirent_file_type::DT_CHR);
    assert_eq!(dents[0].d_ino, dev[0].inode());
    let rest: Vec<::sysapi::dirent::posix_dent> =
        vfs_getdents(fd, dev.len()).expect("getdents rest of /dev");
    assert_eq!(rest.len(), dev.len() - 1);
    assert!(rest[2..]
        .iter()
        .all(|dent| dent.d_type == ::sysapi::dirent::dirent_file_type::DT_UNKNOWN));
    vfs_close(fd).expect("close /dev");
    forget_processes(&[pid]);
}

/// Tests null-device reads, writes, positioned I/O, and seeks.
#[test]
fn null_device_io() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7403);
    set_current_process(pid);

    let fd: c_int = vfs_open("/dev/null", file_access_mode::O_RDWR).expect("open null");
    assert_eq!(vfs_fcntl(fd, file_control_request::F_GETFL, 0), Ok(file_access_mode::O_RDWR));
    assert_eq!(vfs_write(fd, b"discarded"), Ok(9));
    assert_eq!(vfs_write(fd, b"again"), Ok(5));
    assert_eq!(vfs_pwrite(fd, b"positionless", 42), Ok(12));
    let mut data: [u8; 8] = [0; 8];
    assert_eq!(vfs_read(fd, &mut data), Ok(0));
    assert_eq!(vfs_pread(fd, &mut data, 42), Ok(0));
    assert_eq!(vfs_pread(fd, &mut data, -1), Err(Fat32Error::InvalidArgument));
    assert_eq!(vfs_pwrite(fd, b"rejected", -1), Err(Fat32Error::InvalidArgument));
    for whence in [
        file_seek::SEEK_SET,
        file_seek::SEEK_CUR,
        file_seek::SEEK_END,
    ] {
        assert_eq!(vfs_lseek(fd, 17, whence), Ok(0));
    }
    assert_eq!(vfs_lseek(fd, 0, -1), Err(Fat32Error::InvalidArgument));

    vfs_close(fd).expect("close null");
    forget_processes(&[pid]);
}

/// Tests null-device descriptor metadata against pathname metadata.
#[test]
fn null_device_fstat() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7404);
    set_current_process(pid);

    let fd: c_int = vfs_open("/dev/null", file_access_mode::O_RDONLY).expect("open null");
    let mut by_path: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    let mut by_fd: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_stat("/dev/null", &mut by_path).expect("stat null");
    vfs_fstat(fd, &mut by_fd).expect("fstat null");

    assert_eq!(by_fd.st_mode & file_type::S_IFMT, file_type::S_IFCHR);
    assert_eq!(by_fd.st_dev, by_path.st_dev);
    assert_eq!(by_fd.st_ino, by_path.st_ino);
    assert_eq!(by_fd.st_rdev, by_path.st_rdev);
    assert_eq!(by_fd.st_size, 0);
    assert_eq!(by_fd.st_blocks, 0);

    vfs_close(fd).expect("close null");
    forget_processes(&[pid]);
}

/// Tests null-device creation flags and access modes.
#[test]
fn null_device_open_modes() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7405);
    set_current_process(pid);
    let mut data: [u8; 8] = [0; 8];

    let flags: c_int =
        file_access_mode::O_WRONLY | file_creation_flags::O_CREAT | file_creation_flags::O_TRUNC;
    let write_fd: c_int = vfs_open("/dev/null", flags).expect("open null for writing");
    assert_eq!(
        vfs_fcntl(write_fd, file_control_request::F_GETFL, 0),
        Ok(file_access_mode::O_WRONLY)
    );
    assert_eq!(vfs_write(write_fd, b"discarded"), Ok(9));
    assert_eq!(vfs_read(write_fd, &mut data), Err(Fat32Error::InvalidFd));
    vfs_close(write_fd).expect("close write-only null");

    let read_fd: c_int = vfs_open("/dev/null", file_access_mode::O_RDONLY).expect("open null");
    assert_eq!(
        vfs_fcntl(read_fd, file_control_request::F_GETFL, 0),
        Ok(file_access_mode::O_RDONLY)
    );
    assert_eq!(vfs_write(read_fd, b"rejected"), Err(Fat32Error::InvalidFd));
    vfs_close(read_fd).expect("close read-only null");

    let exclusive_flags: c_int =
        file_access_mode::O_WRONLY | file_creation_flags::O_CREAT | file_creation_flags::O_EXCL;
    assert_eq!(vfs_open("/dev/null", exclusive_flags), Err(Fat32Error::AlreadyExists));
    assert_eq!(vfs_open("/dev/null", file_access_mode::O_EXEC), Err(Fat32Error::PermissionDenied));
    forget_processes(&[pid]);
}

/// Tests null-device polling and synchronization.
#[test]
fn null_device_poll_and_sync() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7406);
    set_current_process(pid);

    for mode in [file_access_mode::O_RDONLY, file_access_mode::O_WRONLY] {
        let fd: c_int = vfs_open("/dev/null", mode).expect("open null");
        let events: c_short = POLLIN | POLLOUT | POLLRDNORM | POLLWRNORM;
        assert_eq!(vfs_poll(fd, events), Ok(events));
        assert_eq!(vfs_fsync(fd), Err(Fat32Error::InvalidArgument));
        assert_eq!(vfs_ftruncate(fd, 0), Err(Fat32Error::InvalidArgument));
        vfs_close(fd).expect("close null");
    }
    forget_processes(&[pid]);
}

/// Tests directory requirements when opening the null device.
#[test]
fn null_device_path_errors() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7407);
    set_current_process(pid);

    let directory_flags: c_int = file_access_mode::O_RDONLY | file_creation_flags::O_DIRECTORY;
    assert_eq!(vfs_open("/dev/null", directory_flags), Err(Fat32Error::NotADirectory));
    assert_eq!(vfs_open("/dev/null/", file_access_mode::O_RDONLY), Err(Fat32Error::NotADirectory));
    forget_processes(&[pid]);
}

/// Tests typed FAT consumers through open, stat, cache, and mount-lifetime behavior.
#[test]
fn typed_fat_consumers_preserve_open_semantics() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x740a);
    set_current_process(pid);
    let mount: &str = "/typed-consumers";
    let cache_flush_mount: &str = "/typed-cache-flush";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");
    vfs_mkdir("/typed-consumers/dir").expect("create directory");
    let create_flags: c_int = file_access_mode::O_RDWR | file_creation_flags::O_CREAT;
    let file: c_int = vfs_open("/typed-consumers/file", create_flags).expect("create file");
    vfs_close(file).expect("close created file");

    let directory_flags: c_int = file_access_mode::O_RDONLY | file_creation_flags::O_DIRECTORY;
    assert_eq!(
        vfs_open("/typed-consumers/file/", file_access_mode::O_RDONLY),
        Err(Fat32Error::NotADirectory)
    );
    assert_eq!(vfs_open("/typed-consumers/file", directory_flags), Err(Fat32Error::NotADirectory));
    for path in ["/typed-consumers/dir/", "/typed-consumers/dir", mount] {
        let fd: c_int = vfs_open(path, file_access_mode::O_RDONLY).expect("open directory");
        vfs_close(fd).expect("close directory");
    }

    let mut root_stat: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_stat(mount, &mut root_stat).expect("stat mount root");
    assert_eq!(root_stat.st_mode & file_type::S_IFMT, file_type::S_IFDIR);

    crate::state::create_mount(cache_flush_mount, crate::state::MIN_FAT_SIZE)
        .expect("create cache-flush mount");
    let first = crate::file::stat("/typed-consumers/file").expect("stat cache miss");
    let after_miss: usize =
        crate::state::with_vfs(|vfs| Ok(vfs.resolve_cache_len())).expect("read cache size");
    let second = crate::file::stat("/typed-consumers//./file").expect("stat cache hit");
    let after_hit: usize =
        crate::state::with_vfs(|vfs| Ok(vfs.resolve_cache_len())).expect("read cache size");
    assert_eq!(first, second, "cache hit must preserve metadata");
    assert_eq!(after_miss, 1, "first resolution should populate one cache entry");
    assert_eq!(after_hit, after_miss, "equivalent path should reuse the cache entry");

    let file: c_int =
        vfs_open("/typed-consumers/file", file_access_mode::O_RDWR).expect("open tracked FAT file");
    assert_eq!(crate::state::unmount(mount), Err(Fat32Error::FileLocked));
    vfs_close(file).expect("close tracked FAT file");
    crate::state::unmount(mount).expect("unmount closed filesystem");
    crate::state::unmount(cache_flush_mount).expect("unmount cache-flush filesystem");
    forget_processes(&[pid]);
}

/// Tests that unmounting a modified guest mount flushes its FAT filesystem before freeing it.
#[test]
fn unmount_flushes_fat_before_freeing_image() {
    // Host allocators unmap allocations this large when freeing them, so any write to a freed
    // image faults. The size exceeds both the cap on glibc's dynamic mmap threshold (32 MiB)
    // and the Windows heap's threshold for allocating from virtual memory (about 1 MiB).
    const UNMAPPED_IMAGE_SIZE: usize = 64 * 1024 * 1024;

    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x740d);
    set_current_process(pid);
    let mount: &str = "/unmount-flush";
    crate::state::create_mount(mount, UNMAPPED_IMAGE_SIZE).expect("create mount");

    // Modifying the filesystem marks it dirty, so unmounting it writes back to the image.
    vfs_mkdir("/unmount-flush/dir").expect("create directory");
    crate::state::unmount(mount).expect("unmount modified filesystem");
    forget_processes(&[pid]);
}

/// Hostfs cwd bookkeeping preserves spelling; local chdir retains its normalization policy.
#[test]
fn hostfs_cwd_preserves_path_spelling() {
    use crate::path::vfs_resolve_path;

    const HOST_CWD: &str = "/mnt/link//.././";

    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid = ProcessIdentifier::from(0x740c);
    set_current_process(pid);
    let path = vfs_resolve_path(-99, HOST_CWD).expect("absolute hostfs path");
    vfs_set_cwd(path);
    assert_eq!(vfs_getcwd().expect("hostfs cwd"), HOST_CWD);
    assert_eq!(
        vfs_resolve_path(AT_FDCWD, "marker")
            .expect("cwd-relative path")
            .as_str(),
        "/mnt/link//.././marker",
    );

    vfs_chdir("/dev//./").expect("change to local directory");
    assert_eq!(vfs_getcwd().expect("local cwd"), "/dev");
    forget_processes(&[pid]);
}

/// Tests local `*at` resolution and its required error cases.
#[test]
fn at_paths_resolve_directory_descriptors() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7408);
    set_current_process(pid);
    let mount: &str = "/typed-at";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");
    vfs_mkdir("/typed-at/base").expect("create base directory");

    let directory_flags: c_int = file_access_mode::O_RDONLY | file_creation_flags::O_DIRECTORY;
    let directory: c_int = vfs_open("/typed-at/base", directory_flags).expect("open base");
    let bad_dirfd: c_int = -99;
    assert_eq!(vfs_accessat(bad_dirfd, "missing"), Err(Fat32Error::InvalidFd));
    vfs_mkdirat(bad_dirfd, "/typed-at/absolute").expect("absolute path ignores bad dirfd");
    assert_eq!(vfs_mkdirat(directory, ""), Err(Fat32Error::NotFound));
    vfs_mkdirat(directory, "child").expect("create relative directory");

    let create_flags: c_int = file_access_mode::O_RDWR | file_creation_flags::O_CREAT;
    let file: c_int = vfs_openat(directory, "file", create_flags).expect("create relative file");
    assert_eq!(
        vfs_openat(file, "nested", file_access_mode::O_RDONLY),
        Err(Fat32Error::NotADirectory)
    );

    let mut metadata: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_fstatat(directory, "file", &mut metadata).expect("stat relative file");
    assert_eq!(metadata.st_mode & file_type::S_IFMT, file_type::S_IFREG);
    vfs_accessat(directory, "file").expect("access relative file");
    vfs_fchmodat(directory, "file", 0, 0).expect("validate chmod target");
    vfs_fchownat(directory, "file", 0, 0, 0).expect("validate chown target");
    let times: [timespec; 2] = [
        timespec {
            tv_sec: FAT_EPOCH_SECS,
            tv_nsec: 0,
        },
        timespec {
            tv_sec: FAT_EPOCH_SECS,
            tv_nsec: UTIME_OMIT,
        },
    ];
    vfs_utimensat(directory, "file", &times, 0).expect("set relative timestamps");

    vfs_close(file).expect("close relative file");
    vfs_unlinkat(directory, "file", 0).expect("unlink relative file");
    vfs_unlinkat(directory, "child", ::sysapi::fcntl::atflags::AT_REMOVEDIR)
        .expect("remove relative directory");
    vfs_rmdir("/typed-at/absolute").expect("remove absolute directory");
    vfs_close(directory).expect("close base directory");
    vfs_rmdir("/typed-at/base").expect("remove base directory");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that `renameat()` resolves its two directory descriptors independently.
#[test]
fn renameat_uses_separate_anchors() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7409);
    set_current_process(pid);
    let mount: &str = "/typed-renameat";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");
    vfs_mkdir("/typed-renameat/source").expect("create source directory");
    vfs_mkdir("/typed-renameat/destination").expect("create destination directory");

    let directory_flags: c_int = file_access_mode::O_RDONLY | file_creation_flags::O_DIRECTORY;
    let source: c_int =
        vfs_open("/typed-renameat/source", directory_flags).expect("open source directory");
    let destination: c_int = vfs_open("/typed-renameat/destination", directory_flags)
        .expect("open destination directory");
    let create_flags: c_int = file_access_mode::O_RDWR | file_creation_flags::O_CREAT;
    let file: c_int = vfs_openat(source, "file", create_flags).expect("create source file");
    vfs_close(file).expect("close source file");

    vfs_renameat(source, "file", destination, "renamed").expect("rename between anchors");
    assert_eq!(vfs_accessat(source, "file"), Err(Fat32Error::NotFound));
    vfs_accessat(destination, "renamed").expect("renamed file exists");

    vfs_unlinkat(destination, "renamed", 0).expect("unlink renamed file");
    vfs_close(source).expect("close source directory");
    vfs_close(destination).expect("close destination directory");
    vfs_rmdir("/typed-renameat/source").expect("remove source directory");
    vfs_rmdir("/typed-renameat/destination").expect("remove destination directory");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Writes all of `data` to a descriptor, looping on short writes.
fn write_all(fd: c_int, data: &[u8]) {
    let mut written: usize = 0;
    while written < data.len() {
        let n: c_size_t = vfs_write(fd, &data[written..]).expect("write should succeed");
        assert!(n > 0, "write should make progress");
        written += n as usize;
    }
}

/// Reads a descriptor in small chunks until the end of the file.
fn read_to_end(fd: c_int) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    let mut buf: [u8; 300] = [0u8; 300];
    loop {
        let n: c_size_t = vfs_read(fd, &mut buf).expect("read should succeed");
        if n == 0 {
            return data;
        }
        data.extend_from_slice(&buf[..n as usize]);
    }
}

/// Tests reading a fragmented FAT file through a read-only descriptor, which reads the file
/// through its cluster chain instead of in place.
#[test]
fn fat_descriptor_reads_fragmented_file() {
    // Ends at a cluster boundary, so that the next write to the file allocates a new cluster.
    const HEAD_SIZE: usize = 2048;
    const TAIL_SIZE: usize = 2048 + 7;

    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x740e);
    set_current_process(pid);
    let mount: &str = "/fragmented-read";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    // Interleave the clusters of two files, so that the first one is not contiguous.
    let data: Vec<u8> = (0..HEAD_SIZE + TAIL_SIZE).map(|i| (i % 251) as u8).collect();
    let create_flags: c_int = file_access_mode::O_WRONLY | file_creation_flags::O_CREAT;
    let file: c_int = vfs_open("/fragmented-read/file", create_flags).expect("create file");
    let other: c_int =
        vfs_open("/fragmented-read/other", create_flags).expect("create other file");
    write_all(file, &data[..HEAD_SIZE]);
    write_all(other, b"interleaved");
    write_all(file, &data[HEAD_SIZE..]);
    vfs_close(other).expect("close other file");
    vfs_close(file).expect("close file");
    assert!(
        crate::file::file_raw_region("/fragmented-read/file").is_none(),
        "file should not be contiguous"
    );

    let fd: c_int =
        vfs_open("/fragmented-read/file", file_access_mode::O_RDONLY).expect("open file");
    assert_eq!(read_to_end(fd), data, "reads should return the file contents in order");
    let mut buf: [u8; 16] = [0u8; 16];
    assert_eq!(vfs_read(fd, &mut buf), Ok(0), "read at the end should return 0");
    assert_eq!(vfs_lseek(fd, 0, file_seek::SEEK_END), Ok(data.len() as off_t));
    assert_eq!(vfs_pread(fd, &mut buf, HEAD_SIZE as off_t), Ok(16));
    assert_eq!(buf[..], data[HEAD_SIZE..HEAD_SIZE + 16], "pread should read at the offset");
    let mut metadata: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_fstat(fd, &mut metadata).expect("fstat file");
    assert_eq!(metadata.st_size, data.len() as off_t, "fstat should report the file size");
    vfs_close(fd).expect("close file");

    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that the size of a FAT file seen through a descriptor follows writes and truncations.
#[test]
fn fat_descriptor_size_follows_writes() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x740f);
    set_current_process(pid);
    let mount: &str = "/size-follows-writes";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    let create_flags: c_int = file_access_mode::O_RDWR | file_creation_flags::O_CREAT;
    let fd: c_int = vfs_open("/size-follows-writes/file", create_flags).expect("create file");
    let mut buf: [u8; 16] = [0u8; 16];
    assert_eq!(vfs_read(fd, &mut buf), Ok(0), "new file should be empty");
    assert_eq!(vfs_write(fd, b"12345"), Ok(5));
    assert_eq!(vfs_lseek(fd, 0, file_seek::SEEK_END), Ok(5), "write should grow the file");
    assert_eq!(vfs_pwrite(fd, b"*", 9), Ok(1));
    let mut metadata: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_fstat(fd, &mut metadata).expect("fstat file");
    assert_eq!(metadata.st_size, 10, "pwrite past the end should grow the file");
    assert_eq!(vfs_pread(fd, &mut buf, 0), Ok(10));
    assert_eq!(&buf[..10], b"12345\0\0\0\0*", "pwrite should fill the gap with zeros");

    vfs_ftruncate(fd, 3).expect("shrink file");
    assert_eq!(vfs_lseek(fd, 0, file_seek::SEEK_END), Ok(3), "ftruncate should shrink the file");
    assert_eq!(vfs_pread(fd, &mut buf, 3), Ok(0), "read at the end should return 0");
    vfs_ftruncate(fd, 6).expect("extend file");
    assert_eq!(vfs_lseek(fd, 0, file_seek::SEEK_SET), Ok(0));
    assert_eq!(vfs_read(fd, &mut buf), Ok(6), "ftruncate should extend the file");
    assert_eq!(&buf[..6], b"123\0\0\0", "ftruncate should extend the file with zeros");
    assert_eq!(vfs_read(fd, &mut buf), Ok(0), "read at the end should return 0");

    vfs_close(fd).expect("close file");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that data written through a FAT descriptor is visible to `stat()` and to descriptors
/// opened afterwards, while the writing descriptor is still open.
#[test]
fn fat_descriptor_writes_visible_to_new_descriptors() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7410);
    set_current_process(pid);
    let mount: &str = "/write-visibility";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    // Create the file like a temporary file is created, and keep it open.
    let path: &str = "/write-visibility/file.env";
    let create_flags: c_int =
        file_access_mode::O_RDWR | file_creation_flags::O_CREAT | file_creation_flags::O_EXCL;
    let writer: c_int = vfs_open(path, create_flags).expect("create file");
    let mut expected: Vec<u8> = Vec::new();
    for data in [&b"KEY=value\n"[..], &b"OTHER=value\n"[..]] {
        write_all(writer, data);
        expected.extend_from_slice(data);

        // A read-only descriptor reads a contiguous file in place, and a read-write descriptor
        // reads it through its cluster chain.
        for flags in [file_access_mode::O_RDONLY, file_access_mode::O_RDWR] {
            let reader: c_int = vfs_open(path, flags).expect("open file for reading");
            assert_eq!(read_to_end(reader), expected, "reader should see the writes");
            vfs_close(reader).expect("close reader");
        }
        let mut metadata: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
        vfs_stat(path, &mut metadata).expect("stat file");
        assert_eq!(metadata.st_size, expected.len() as off_t, "stat should see the writes");
    }

    vfs_close(writer).expect("close writer");
    vfs_unlink(path).expect("unlink file");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that truncating a FAT file through a descriptor is visible to `stat()` and to descriptors
/// opened afterwards, while the truncating descriptor is still open.
#[test]
fn fat_descriptor_truncation_visible_to_new_descriptors() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7412);
    set_current_process(pid);
    let mount: &str = "/truncation-visibility";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    let path: &str = "/truncation-visibility/file";
    let create_flags: c_int = file_access_mode::O_WRONLY | file_creation_flags::O_CREAT;
    let fd: c_int = vfs_open(path, create_flags).expect("create file");
    write_all(fd, b"initial content");
    vfs_close(fd).expect("close file");

    // Truncate the file when opening it, and keep it open.
    let writer: c_int = vfs_open(path, create_flags | file_creation_flags::O_TRUNC)
        .expect("open file with O_TRUNC");
    let reader: c_int = vfs_open(path, file_access_mode::O_RDONLY).expect("open file for reading");
    assert_eq!(read_to_end(reader), b"", "reader should see the truncation on open");
    vfs_close(reader).expect("close reader");
    let mut metadata: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_stat(path, &mut metadata).expect("stat file");
    assert_eq!(metadata.st_size, 0, "stat should see the truncation on open");

    write_all(writer, b"hello world");
    vfs_ftruncate(writer, 5).expect("shrink file");
    let reader: c_int = vfs_open(path, file_access_mode::O_RDONLY).expect("open file for reading");
    assert_eq!(read_to_end(reader), b"hello", "reader should see the truncation");
    vfs_close(reader).expect("close reader");
    vfs_stat(path, &mut metadata).expect("stat file");
    assert_eq!(metadata.st_size, 5, "stat should see the truncation");

    vfs_close(writer).expect("close writer");
    vfs_unlink(path).expect("unlink file");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that `stat()` follows the creation and the removal of paths, and the writes to files,
/// although a mount answers repeated queries, including for paths that do not exist, from its cache.
#[test]
fn stat_follows_creation_and_removal() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7416);
    set_current_process(pid);
    let mount: &str = "/stat-creation";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    let file: &str = "/stat-creation/file";
    let dir: &str = "/stat-creation/dir";
    for path in [file, dir] {
        for _ in 0..2 {
            assert_eq!(crate::file::stat(path), Err(Fat32Error::NotFound), "path should be missing");
        }
    }

    // Create the file and keep it open, so that its data is written through its descriptor.
    let create_flags: c_int = file_access_mode::O_RDWR | file_creation_flags::O_CREAT;
    let fd: c_int = vfs_open(file, create_flags).expect("create file");
    let created: filesystem::Stat = crate::file::stat(file).expect("stat created file");
    assert_eq!(created.size(), 0, "created file should be empty");
    assert!(!created.is_dir(), "created file should not be a directory");
    write_all(fd, b"hello");
    assert_eq!(
        crate::file::stat(file).expect("stat written file").size(),
        5,
        "stat should see the write"
    );
    vfs_close(fd).expect("close file");

    vfs_mkdir(dir).expect("create directory");
    assert!(
        crate::file::stat(dir).expect("stat created directory").is_dir(),
        "created directory should be found"
    );

    vfs_unlink(file).expect("unlink file");
    vfs_rmdir(dir).expect("remove directory");
    for path in [file, dir] {
        assert_eq!(crate::file::stat(path), Err(Fat32Error::NotFound), "path should be gone");
    }

    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that `stat()` follows renames, including the move of the paths below a renamed directory,
/// and the change of the times of a file, although a mount answers repeated queries from its cache.
#[test]
fn stat_follows_rename_and_times() {
    // A timestamp within the FAT range (2024-06-15T12:30:00Z).
    const TIME: i64 = 1_718_454_600;

    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7417);
    set_current_process(pid);
    let mount: &str = "/stat-rename";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    vfs_mkdir("/stat-rename/dir").expect("create directory");
    let create_flags: c_int = file_access_mode::O_WRONLY | file_creation_flags::O_CREAT;
    let fd: c_int = vfs_open("/stat-rename/dir/file", create_flags).expect("create file");
    write_all(fd, b"hello");
    vfs_close(fd).expect("close file");

    // Query every path involved, so that the mount remembers its metadata, or that it is missing.
    let old_file: filesystem::Stat = crate::file::stat("/stat-rename/dir/file").expect("stat file");
    assert_eq!(old_file.size(), 5, "file should have been written");
    for path in ["/stat-rename/moved", "/stat-rename/moved/file"] {
        assert_eq!(crate::file::stat(path), Err(Fat32Error::NotFound), "path should be missing");
    }
    // A trailing separator must still be rejected on a file whose metadata is cached.
    assert_eq!(crate::file::stat("/stat-rename/dir/file/"), Err(Fat32Error::NotADirectory));

    vfs_rename("/stat-rename/dir", "/stat-rename/moved").expect("rename directory");
    assert_eq!(crate::file::stat("/stat-rename/dir"), Err(Fat32Error::NotFound));
    assert_eq!(crate::file::stat("/stat-rename/dir/file"), Err(Fat32Error::NotFound));
    assert!(crate::file::stat("/stat-rename/moved").expect("stat directory").is_dir());
    assert_eq!(
        crate::file::stat("/stat-rename/moved/file").expect("stat moved file").size(),
        5,
        "file should have moved with its directory"
    );

    vfs_rename("/stat-rename/moved/file", "/stat-rename/file").expect("rename file");
    assert_eq!(crate::file::stat("/stat-rename/moved/file"), Err(Fat32Error::NotFound));
    let renamed: filesystem::Stat = crate::file::stat("/stat-rename/file").expect("stat file");
    assert_eq!(renamed.size(), 5, "renamed file should keep its data");
    assert_ne!(renamed.mtime(), TIME, "file should not already have the time to set");

    let times: [timespec; 2] = [
        timespec {
            tv_sec: TIME,
            tv_nsec: 0,
        },
        timespec {
            tv_sec: TIME,
            tv_nsec: 0,
        },
    ];
    vfs_utimensat(::sysapi::fcntl::atflags::AT_FDCWD, "/stat-rename/file", &times, 0)
        .expect("set timestamps");
    assert_eq!(
        crate::file::stat("/stat-rename/file").expect("stat file").mtime(),
        TIME,
        "stat should see the new modification time"
    );

    vfs_unlink("/stat-rename/file").expect("unlink file");
    vfs_rmdir("/stat-rename/moved").expect("remove directory");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that files that are read in place follow renames, removals and the creation of another
/// file at the path of a removed one, although a mount answers repeated queries from its cache.
#[test]
fn direct_reads_follow_renames_and_removal() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7418);
    set_current_process(pid);
    let mount: &str = "/direct-follow";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    let first: &str = "/direct-follow/first";
    let moved: &str = "/direct-follow/moved";
    let create_flags: c_int = file_access_mode::O_WRONLY | file_creation_flags::O_CREAT;
    let fd: c_int = vfs_open(first, create_flags).expect("create file");
    write_all(fd, b"first contents");
    vfs_close(fd).expect("close file");

    // Query every path involved, so that the mount remembers where the data is, or that there is
    // none, and read the file in place.
    let region: (*const u8, usize) =
        crate::file::file_raw_region(first).expect("file should be read in place");
    assert_eq!(region.1, 14, "region should span the file");
    assert_eq!(crate::file::file_raw_region(first), Some(region), "queries should agree");
    assert_eq!(crate::file::file_raw_region(moved), None, "missing path should have no data");
    let fd: c_int = vfs_open(first, file_access_mode::O_RDONLY).expect("open file for reading");
    assert_eq!(read_to_end(fd), b"first contents");
    vfs_close(fd).expect("close file");

    vfs_rename(first, moved).expect("rename file");
    assert_eq!(crate::file::file_raw_region(first), None, "old name should have no data");
    assert_eq!(vfs_open(first, file_access_mode::O_RDONLY), Err(Fat32Error::NotFound));
    assert_eq!(
        crate::file::file_raw_region(moved),
        Some(region),
        "new name should have the data of the file, which did not move"
    );
    let fd: c_int = vfs_open(moved, file_access_mode::O_RDONLY).expect("open renamed file");
    assert_eq!(read_to_end(fd), b"first contents");
    vfs_close(fd).expect("close file");

    vfs_unlink(moved).expect("unlink file");
    assert_eq!(crate::file::file_raw_region(moved), None, "removed file should have no data");
    assert_eq!(vfs_open(moved, file_access_mode::O_RDONLY), Err(Fat32Error::NotFound));

    // A new file takes the clusters that the removed one freed, so its region can start where the
    // region of the removed file did, and then only its size tells the two apart.
    let fd: c_int = vfs_open(moved, create_flags).expect("create file");
    write_all(fd, b"second");
    vfs_close(fd).expect("close file");
    assert_eq!(
        crate::file::file_raw_region(moved).map(|(_, len)| len),
        Some(6),
        "new file should have its own data"
    );
    let fd: c_int = vfs_open(moved, file_access_mode::O_RDONLY).expect("open new file");
    assert_eq!(read_to_end(fd), b"second");
    vfs_close(fd).expect("close file");

    vfs_unlink(moved).expect("unlink file");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that opening a path below a file fails until the file is replaced by a directory, and
/// fails again once the directory is replaced by a file, although a mount answers repeated lookups
/// of the ancestors of a path from its cache.
#[test]
fn open_follows_ancestor_changes() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7419);
    set_current_process(pid);
    let mount: &str = "/ancestor-follow";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    let node: &str = "/ancestor-follow/node";
    let child: &str = "/ancestor-follow/node/child";
    let create_flags: c_int = file_access_mode::O_RDWR | file_creation_flags::O_CREAT;

    let fd: c_int = vfs_open(node, create_flags).expect("create file");
    vfs_close(fd).expect("close file");
    for _ in 0..2 {
        assert_eq!(
            vfs_open(child, create_flags),
            Err(Fat32Error::NotADirectory),
            "a file should not hold paths"
        );
    }

    // The file turns into a directory.
    vfs_unlink(node).expect("unlink file");
    vfs_mkdir(node).expect("create directory");
    let fd: c_int = vfs_open(child, create_flags).expect("create file in directory");
    vfs_close(fd).expect("close file");

    // The directory turns into a file again.
    vfs_unlink(child).expect("unlink file in directory");
    vfs_rmdir(node).expect("remove directory");
    let fd: c_int = vfs_open(node, create_flags).expect("create file");
    vfs_close(fd).expect("close file");
    assert_eq!(
        vfs_open(child, create_flags),
        Err(Fat32Error::NotADirectory),
        "a file should not hold paths"
    );

    vfs_unlink(node).expect("unlink file");
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests opened terminal devices independently of the standard descriptor slots.
#[test]
fn terminal_device_open_modes() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = root_process_identifier();
    vfs_seed_root_console(pid);
    set_current_process(pid);

    let redirected: c_int = vfs_alloc_hostfs(10, false, None, file_access_mode::O_RDONLY)
        .expect("open redirected file");
    for fd in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
        vfs_dup2(redirected, fd).expect("redirect standard descriptor");
    }
    vfs_close(redirected).expect("close redirected file");
    for fd in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
        assert_eq!(vfs_resolve(fd), Some((VfsRoute::Vfs, fd)));
    }

    assert_eq!(vfs_open("/dev/tty", file_access_mode::O_EXEC), Err(Fat32Error::PermissionDenied));
    assert_eq!(
        vfs_open("/dev/console", file_access_mode::O_EXEC),
        Err(Fat32Error::PermissionDenied)
    );
    let console_write_flags: c_int =
        file_access_mode::O_WRONLY | file_creation_flags::O_CREAT | file_creation_flags::O_TRUNC;
    let directory: c_int = vfs_open("/dev/", file_access_mode::O_RDONLY).expect("open /dev/");
    for flags in [file_access_mode::O_RDONLY, console_write_flags] {
        assert_eq!(vfs_open("/dev/console/", flags), Err(Fat32Error::NotADirectory));
        assert_eq!(vfs_openat(directory, "console/", flags), Err(Fat32Error::NotADirectory));
    }
    vfs_close(directory).expect("close /dev/");

    let tty_read: c_int = vfs_open("/dev/tty", file_access_mode::O_RDONLY).expect("open tty");
    let console_write: c_int = vfs_open("/dev/console", console_write_flags).expect("open console");
    let tty_readwrite: c_int =
        vfs_open("/dev/tty", file_access_mode::O_RDWR).expect("open read-write tty");

    for fd in [tty_read, console_write, tty_readwrite] {
        assert_eq!(vfs_resolve(fd), Some((VfsRoute::Terminal, fd)));
    }
    assert_eq!(vfs_terminal_access(tty_read), Ok((true, false)));
    assert_eq!(vfs_terminal_access(console_write), Ok((false, true)));
    assert_eq!(vfs_terminal_access(tty_readwrite), Ok((true, true)));

    let resized: Winsize = Winsize {
        ws_row: 40,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    vfs_tty_set_winsize(tty_read, resized).expect("set tty winsize");
    assert_eq!(vfs_tty_get_winsize(console_write), Ok(resized));

    let child: ProcessIdentifier = ProcessIdentifier::from(0x7408);
    vfs_fork_clone(pid, child).expect("fork terminal descriptors");
    set_current_process(child);
    assert_eq!(vfs_terminal_access(tty_readwrite), Ok((true, true)));
    assert_eq!(vfs_tty_get_winsize(console_write), Ok(resized));
    vfs_detach_controlling_terminal(child);
    let exclusive_tty: c_int =
        file_access_mode::O_RDONLY | file_creation_flags::O_CREAT | file_creation_flags::O_EXCL;
    assert_eq!(vfs_open("/dev/tty", exclusive_tty), Err(Fat32Error::AlreadyExists));
    assert_eq!(vfs_open("/dev/tty", file_access_mode::O_RDONLY), Err(Fat32Error::NoDevice));
    let detached_console: c_int =
        vfs_open("/dev/console", file_access_mode::O_RDONLY).expect("open system console");
    vfs_close(detached_console).expect("close system console");
    set_current_process(pid);

    let mut tty_path: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    let mut tty_fd: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    let mut console_path: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    let mut console_fd: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_stat("/dev/tty", &mut tty_path).expect("stat tty");
    vfs_fstat(tty_read, &mut tty_fd).expect("fstat tty");
    vfs_stat("/dev/console", &mut console_path).expect("stat console");
    vfs_fstat(console_write, &mut console_fd).expect("fstat console");
    assert_eq!(tty_fd.st_ino, tty_path.st_ino);
    assert_eq!(console_fd.st_ino, console_path.st_ino);
    assert_ne!(tty_fd.st_ino, console_fd.st_ino);

    let exclusive_flags: c_int =
        file_access_mode::O_WRONLY | file_creation_flags::O_CREAT | file_creation_flags::O_EXCL;
    assert_eq!(vfs_open("/dev/tty", exclusive_flags), Err(Fat32Error::AlreadyExists));

    for fd in [tty_read, console_write, tty_readwrite] {
        vfs_close(fd).expect("close terminal");
    }
    forget_processes(&[pid, child]);
}

// -- DirectReadHandle tests --------------------------------------------------

/// Tests reading from a direct read handle.
#[test]
fn direct_read_basic() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
    let mut buf: [u8; 3] = [0; 3];

    let n: usize = handle.read(&mut buf);
    assert_eq!(n, 3, "should read 3 bytes");
    assert_eq!(buf, [1, 2, 3], "first 3 bytes");
}

/// Tests reading until EOF.
#[test]
fn direct_read_to_eof() {
    let data: [u8; 3] = [10, 20, 30];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
    let mut buf: [u8; 10] = [0; 10];

    let n: usize = handle.read(&mut buf);
    assert_eq!(n, 3, "should read all 3 bytes");
    assert_eq!(&buf[..3], &[10, 20, 30]);

    let n2: usize = handle.read(&mut buf);
    assert_eq!(n2, 0, "should return 0 at EOF");
}

/// Tests reading with empty buffer.
#[test]
fn direct_read_empty_buffer() {
    let data: [u8; 3] = [1, 2, 3];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
    let mut buf: [u8; 0] = [];

    let n: usize = handle.read(&mut buf);
    assert_eq!(n, 0, "empty buffer should read 0 bytes");
}

/// Tests SEEK_SET on a direct read handle.
#[test]
fn direct_seek_set() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

    // Read 2 bytes to advance position.
    let mut buf: [u8; 2] = [0; 2];
    handle.read(&mut buf);

    // Seek back to start.
    let pos: off_t = handle
        .seek(0, file_seek::SEEK_SET)
        .expect("SEEK_SET should succeed");
    assert_eq!(pos, 0, "position should be 0");

    // Read again.
    let n: usize = handle.read(&mut buf);
    assert_eq!(n, 2, "should read 2 bytes after seek");
    assert_eq!(buf, [1, 2], "should re-read first 2 bytes");
}

/// Tests SEEK_CUR on a direct read handle.
#[test]
fn direct_seek_cur() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

    // Read 2 bytes to advance position.
    let mut buf: [u8; 2] = [0; 2];
    handle.read(&mut buf);

    // Seek forward by 1 (relative).
    let pos: off_t = handle
        .seek(1, file_seek::SEEK_CUR)
        .expect("SEEK_CUR should succeed");
    assert_eq!(pos, 3, "position should be 3");

    // Read next byte.
    let mut one: [u8; 1] = [0];
    let n: usize = handle.read(&mut one);
    assert_eq!(n, 1);
    assert_eq!(one[0], 4, "should be the 4th byte");
}

/// Tests SEEK_END on a direct read handle.
#[test]
fn direct_seek_end() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

    let pos: off_t = handle
        .seek(0, file_seek::SEEK_END)
        .expect("SEEK_END should succeed");
    assert_eq!(pos, 5, "position should be at end");

    let pos2: off_t = handle
        .seek(-2, file_seek::SEEK_END)
        .expect("SEEK_END(-2) should succeed");
    assert_eq!(pos2, 3, "position should be 3");
}

/// Tests that seeking to a negative position fails.
#[test]
fn direct_seek_negative_fails() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

    let result = handle.seek(-1, file_seek::SEEK_SET);
    assert!(result.is_err(), "negative SEEK_SET should fail");
}

/// Tests that seeking past end fails.
#[test]
fn direct_seek_past_end_fails() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

    let result = handle.seek(6, file_seek::SEEK_SET);
    assert!(result.is_err(), "seeking past end should fail");
}

/// Tests that an invalid whence value fails.
#[test]
fn direct_seek_invalid_whence() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

    let result = handle.seek(0, 99);
    assert!(result.is_err(), "invalid whence should fail");
}

/// Tests the size accessor.
#[test]
fn direct_read_size() {
    let data: [u8; 42] = [0; 42];
    let handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
    assert_eq!(handle.size(), 42, "size should match data length");
}

// -- VfsFileHandle::DirectRead dispatch tests --------------------------------

/// Tests VfsFileHandle::DirectRead read dispatch.
#[test]
fn vfs_handle_direct_read() {
    let data: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
    let mut handle: VfsFileHandle =
        VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

    let mut buf: [u8; 4] = [0; 4];
    let n: usize = handle.read(&mut buf).expect("read should succeed");
    assert_eq!(n, 4);
    assert_eq!(buf, data);
}

/// Tests VfsFileHandle::DirectRead write fails (read-only).
#[test]
fn vfs_handle_direct_write_fails() {
    let data: [u8; 4] = [0; 4];
    let mut handle: VfsFileHandle =
        VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

    let result = handle.write(&[1, 2, 3]);
    assert!(result.is_err(), "writing to DirectRead should fail");
}

/// Tests VfsFileHandle::DirectRead seek dispatch.
#[test]
fn vfs_handle_direct_seek() {
    let data: [u8; 10] = [0; 10];
    let mut handle: VfsFileHandle =
        VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

    let pos: off_t = handle
        .seek(5, file_seek::SEEK_SET)
        .expect("seek should succeed");
    assert_eq!(pos, 5);
}

/// Tests VfsFileHandle::DirectRead size dispatch.
#[test]
fn vfs_handle_direct_size() {
    let data: [u8; 100] = [0; 100];
    let mut handle: VfsFileHandle =
        VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

    let size: u64 = handle.size().expect("size should succeed");
    assert_eq!(size, 100);
}

// -- flat allocation & resolution tests --------------------------------------

/// Tests that allocation is flat and lowest-free: descriptors are handed out from `0` upward and
/// a freed number is reused before any higher one. Every object draws from one flat range, so an
/// allocated descriptor is simply the smallest free number.
#[test]
fn alloc_is_lowest_free() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7401);
    set_current_process(pid);

    // A fresh process with no console seeded allocates from 0 upward.
    let a: c_int = vfs_alloc_hostfs(10, false, None, file_access_mode::O_RDONLY).expect("alloc a");
    let b: c_int = vfs_alloc_hostfs(11, false, None, file_access_mode::O_RDONLY).expect("alloc b");
    let c: c_int = vfs_alloc_hostfs(12, false, None, file_access_mode::O_RDONLY).expect("alloc c");
    assert_eq!((a, b, c), (0, 1, 2), "flat allocation hands out the lowest free numbers");
    assert!(
        [a, b, c].iter().all(|fd| (0..MAX_OPEN_FDS).contains(fd)),
        "an allocated descriptor must fall within the flat namespace"
    );

    // Freeing the middle descriptor makes it the lowest free, so the next alloc reuses it.
    vfs_close(b).expect("close b");
    let d: c_int = vfs_alloc_hostfs(13, false, None, file_access_mode::O_RDONLY).expect("alloc d");
    assert_eq!(d, b, "a freed number is reused before any higher one");

    forget_processes(&[pid]);
}

/// Tests that the first `open` in a process whose console slots are seeded returns `3`, because
/// `0`/`1`/`2` are occupied by the standard streams — the POSIX lowest-free expectation.
#[test]
fn first_open_after_console_seed_returns_three() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();

    vfs_seed_root_console(root);
    set_current_process(root);
    let fd: c_int =
        vfs_alloc_hostfs(20, false, None, file_access_mode::O_RDONLY).expect("alloc after seed");
    assert_eq!(fd, 3, "console occupies 0/1/2, so the first open returns 3");

    // Closing stdin frees slot 0, which the lowest-free allocator then reuses ahead of 4.
    vfs_close(STDIN_FILENO).expect("close stdin");
    let reused: c_int = vfs_alloc_hostfs(21, false, None, file_access_mode::O_RDONLY)
        .expect("alloc after closing stdin");
    assert_eq!(reused, STDIN_FILENO, "closing stdin makes 0 the lowest free number");

    forget_processes(&[root]);
}

/// Tests that [`vfs_resolve`] answers a descriptor's backend from its slot handle: a console
/// token reports its stream number, a socket token its networkd descriptor, and every
/// vfsd-served handle the raw descriptor. An absent descriptor resolves to `None`.
#[test]
fn vfs_resolve_reports_backend_by_handle() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7411);
    set_current_process(pid);

    let console_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc");
    let socket_fd: c_int =
        alloc_fd(VfsFileHandle::Socket(SocketHandle::new(77))).expect("socket alloc");
    let file_fd: c_int =
        vfs_alloc_hostfs(30, false, None, file_access_mode::O_RDONLY).expect("hostfs alloc");

    // The console token's backend is its stream number (stdout = 1), not its slot number.
    assert_eq!(
        vfs_resolve(console_fd),
        Some((VfsRoute::Console, STDOUT_FILENO)),
        "a console token resolves to its stream number"
    );
    assert_eq!(
        vfs_resolve(socket_fd),
        Some((VfsRoute::Socket, 77)),
        "a socket token resolves to its networkd descriptor"
    );
    assert_eq!(
        vfs_resolve(file_fd),
        Some((VfsRoute::Vfs, file_fd)),
        "a vfsd-served handle resolves to its raw descriptor"
    );
    assert_eq!(vfs_resolve(4096), None, "an absent descriptor resolves to None");

    forget_processes(&[pid]);
}

/// Tests that a host-backed descriptor records the access mode and `O_NONBLOCK` it was opened
/// with, while creation flags are not retained as status flags.
#[test]
fn hostfs_descriptor_records_open_status_flags() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7413);
    set_current_process(pid);

    let flags: c_int = file_access_mode::O_WRONLY
        | file_creation_flags::O_CREAT
        | file_creation_flags::O_TRUNC
        | file_status_flags::O_NONBLOCK;
    let fd: c_int = vfs_alloc_hostfs(90, false, None, flags).expect("hostfs alloc");
    let duplicate: c_int = vfs_dup(fd).expect("dup hostfs descriptor");
    let expected: c_int = file_access_mode::O_WRONLY | file_status_flags::O_NONBLOCK;
    assert_eq!(vfs_fcntl(fd, file_control_request::F_GETFL, 0), Ok(expected));
    assert_eq!(vfs_fcntl(duplicate, file_control_request::F_GETFL, 0), Ok(expected));

    forget_processes(&[pid]);
}

// -- Standard descriptor alias tests ----------------------------------------

/// Tests that opening a standard-descriptor alias duplicates the descriptor it names: the new
/// descriptor shares the open file description, ignores the flags that would create a new one,
/// and takes `O_CLOEXEC` as its own descriptor flag.
#[test]
fn descriptor_alias_open_shares_description() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7414);
    set_current_process(pid);
    let mount: &str = "/stdio-alias";
    crate::state::create_mount(mount, crate::state::MIN_FAT_SIZE).expect("create mount");

    // A fresh process receives the lowest free descriptor.
    let create_flags: c_int = file_access_mode::O_RDWR | file_creation_flags::O_CREAT;
    let file: c_int = vfs_open("/stdio-alias/file", create_flags).expect("create file");
    assert_eq!(file, STDIN_FILENO);
    assert_eq!(vfs_write(file, b"hello"), Ok(5));
    assert_eq!(vfs_dup2(file, STDOUT_FILENO), Ok(STDOUT_FILENO));

    let flags: c_int = file_access_mode::O_WRONLY
        | file_creation_flags::O_CREAT
        | file_creation_flags::O_TRUNC
        | file_status_flags::O_APPEND;
    let alias: c_int = vfs_open("/dev/stdout", flags).expect("open /dev/stdout");
    assert_eq!(vfs_fcntl(alias, file_control_request::F_GETFL, 0), Ok(file_access_mode::O_RDWR));
    assert_eq!(vfs_fcntl(alias, file_control_request::F_GETFD, 0), Ok(0));
    assert_eq!(vfs_lseek(alias, 0, file_seek::SEEK_CUR), Ok(5), "the offset is shared");
    assert_eq!(vfs_write(alias, b"!"), Ok(1));
    assert_eq!(vfs_lseek(file, 0, file_seek::SEEK_CUR), Ok(6), "the offset is shared");

    let cloexec_flags: c_int = file_access_mode::O_RDONLY | file_creation_flags::O_CLOEXEC;
    let cloexec: c_int = vfs_open("/dev/stdin", cloexec_flags).expect("open /dev/stdin");
    assert_eq!(vfs_fcntl(cloexec, file_control_request::F_GETFD, 0), Ok(FD_CLOEXEC));

    let mut by_alias: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    let mut by_fd: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_stat("/dev/stdout", &mut by_alias).expect("stat /dev/stdout");
    vfs_fstat(file, &mut by_fd).expect("fstat file");
    assert_eq!(by_alias.st_mode & file_type::S_IFMT, file_type::S_IFREG);
    assert_eq!((by_alias.st_dev, by_alias.st_ino), (by_fd.st_dev, by_fd.st_ino));
    assert_eq!(by_alias.st_size, 6, "O_TRUNC must not truncate the named file");
    assert_eq!(vfs_access("/dev/stdout"), Ok(()));

    let exclusive_flags: c_int =
        file_access_mode::O_WRONLY | file_creation_flags::O_CREAT | file_creation_flags::O_EXCL;
    let directory_flags: c_int = file_access_mode::O_RDONLY | file_creation_flags::O_DIRECTORY;
    assert_eq!(vfs_open("/dev/stdout", exclusive_flags), Err(Fat32Error::AlreadyExists));
    assert_eq!(
        vfs_open("/dev/stdout", file_access_mode::O_EXEC),
        Err(Fat32Error::PermissionDenied)
    );
    assert_eq!(vfs_open("/dev/stdout", directory_flags), Err(Fat32Error::NotADirectory));
    assert_eq!(
        vfs_open("/dev/stdout/", file_access_mode::O_RDONLY),
        Err(Fat32Error::NotADirectory)
    );
    assert_eq!(
        vfs_open("/dev/stdout/child", file_access_mode::O_RDONLY),
        Err(Fat32Error::NotADirectory)
    );
    assert_eq!(vfs_stat("/dev/stdout/", &mut by_alias), Err(Fat32Error::NotADirectory));
    assert_eq!(vfs_access("/dev/stdout/"), Err(Fat32Error::NotADirectory));
    assert_eq!(vfs_chdir("/dev/stdout"), Err(Fat32Error::NotADirectory));
    assert_eq!(vfs_unlink("/dev/stdout"), Err(Fat32Error::PermissionDenied));

    for fd in [cloexec, alias, STDOUT_FILENO, file] {
        vfs_close(fd).expect("close descriptor");
    }
    crate::state::unmount(mount).expect("unmount filesystem");
    forget_processes(&[pid]);
}

/// Tests that a standard-descriptor alias grants only the access its descriptor permits, follows
/// the descriptor's backend, and names nothing once the descriptor is closed.
#[test]
fn descriptor_alias_follows_descriptor_access() {
    let _guard = FORK_TEST_GUARD.lock();
    if !crate::state::is_initialized() {
        crate::state::init().expect("initialize VFS");
    }
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7415);
    set_current_process(pid);

    let (read_end, write_end): (c_int, c_int) = vfs_pipe().expect("create pipe");
    assert_eq!((read_end, write_end), (STDIN_FILENO, STDOUT_FILENO));
    let console: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stderr)))
            .expect("console alloc");
    assert_eq!(console, STDERR_FILENO);

    assert_eq!(
        vfs_open("/dev/stdin", file_access_mode::O_WRONLY),
        Err(Fat32Error::PermissionDenied)
    );
    assert_eq!(vfs_open("/dev/stdin", file_access_mode::O_RDWR), Err(Fat32Error::PermissionDenied));
    assert_eq!(
        vfs_open("/dev/stdout", file_access_mode::O_RDONLY),
        Err(Fat32Error::PermissionDenied)
    );
    assert_eq!(
        vfs_open("/dev/stderr", file_access_mode::O_RDONLY),
        Err(Fat32Error::PermissionDenied)
    );
    assert_eq!(
        vfs_open("/dev/stdin", file_access_mode::O_ACCMODE),
        Err(Fat32Error::InvalidArgument)
    );

    let reader: c_int =
        vfs_open("/dev/stdin", file_access_mode::O_RDONLY).expect("open /dev/stdin");
    let writer: c_int =
        vfs_open("/dev/stdout", file_access_mode::O_WRONLY).expect("open /dev/stdout");
    let error: c_int =
        vfs_open("/dev/stderr", file_access_mode::O_WRONLY).expect("open /dev/stderr");
    assert_eq!(vfs_pipe_id(reader), vfs_pipe_id(read_end));
    assert_eq!(vfs_pipe_id(writer), vfs_pipe_id(write_end));
    assert_eq!(vfs_resolve(error), Some((VfsRoute::Console, STDERR_FILENO)));

    let mut metadata: ::sysapi::sys_stat::stat = ::sysapi::sys_stat::stat::default();
    vfs_stat("/dev/stdin", &mut metadata).expect("stat /dev/stdin");
    assert_eq!(metadata.st_mode & file_type::S_IFMT, file_type::S_IFIFO);
    vfs_stat("/dev/stderr", &mut metadata).expect("stat /dev/stderr");
    assert_eq!(metadata.st_mode & file_type::S_IFMT, file_type::S_IFCHR);

    // A closed standard descriptor leaves its alias naming nothing.
    vfs_close(STDERR_FILENO).expect("close stderr");
    let stderr_flags: c_int = file_access_mode::O_WRONLY | file_creation_flags::O_CREAT;
    assert_eq!(vfs_open("/dev/stderr", stderr_flags), Err(Fat32Error::NotFound));
    assert_eq!(vfs_stat("/dev/stderr", &mut metadata), Err(Fat32Error::NotFound));
    assert_eq!(vfs_access("/dev/stderr"), Err(Fat32Error::NotFound));

    forget_processes(&[pid]);
}

// -- fork() per-process descriptor table tests -------------------------------

/// Serializes the fork tests below. They mutate the process-global `CURRENT_PID` and the shared
/// `PROCESSES` registry, so they must not run concurrently with one another.
static FORK_TEST_GUARD: Mutex<()> = Mutex::new(());

/// Returns the recorded working directory of a process, or `None` if it has no state.
fn registry_cwd(pid: ProcessIdentifier) -> Option<String> {
    PROCESSES.lock().get(&pid).map(|state| state.cwd.clone())
}

/// Returns the number of open descriptor slots held by a process.
fn registry_open_fd_count(pid: ProcessIdentifier) -> usize {
    PROCESSES
        .lock()
        .get(&pid)
        .map(|state| state.slots.len())
        .unwrap_or(0)
}

/// Returns the descriptor-table generation recorded for a process, or `None` if it has no state.
fn registry_generation(pid: ProcessIdentifier) -> Option<u64> {
    PROCESSES.lock().get(&pid).map(|state| state.generation)
}

/// Reads the virtual position of a process's descriptor through its shared open file
/// description.
fn fd_virtual_pos(pid: ProcessIdentifier, fd: c_int) -> Option<off_t> {
    let procs = PROCESSES.lock();
    let state = procs.get(&pid)?;
    let entry: OpenFile = state.slots.get(&fd)?.file.clone();
    let pos: off_t = entry.lock().virtual_pos;
    Some(pos)
}

/// Writes the virtual position of a process's descriptor through its shared open file
/// description.
fn set_fd_virtual_pos(pid: ProcessIdentifier, fd: c_int, pos: off_t) {
    let procs = PROCESSES.lock();
    if let Some(state) = procs.get(&pid) {
        if let Some(slot) = state.slots.get(&fd) {
            slot.file.lock().virtual_pos = pos;
        }
    }
}

/// Removes all recorded state for the given processes (test cleanup).
fn forget_processes(pids: &[ProcessIdentifier]) {
    let mut procs = PROCESSES.lock();
    for pid in pids {
        procs.remove(pid);
    }
    CURRENT_PID.store(0, Ordering::Relaxed);
    // Clear the one-shot root-seeding latch so a later test starts from a pristine state.
    ROOT_CONSOLE_SEEDED.store(false, Ordering::Relaxed);
    *CONSOLE_TERMINAL.lock() = None;
}

/// Tests file creation mask normalization, fork inheritance, and exec preservation.
#[test]
fn umask_follows_process_lifecycle() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7103), ProcessIdentifier::from(0x7104));

    set_current_process(parent);
    assert_eq!(vfs_umask(0o1277), 0, "a new process starts with an empty mask");
    assert_eq!(vfs_apply_umask(0o7777), 0o7500, "only the low nine permission bits are masked");

    vfs_fork_clone(parent, child).expect("fork clone should succeed");
    set_current_process(child);
    assert_eq!(vfs_apply_umask(0o7777), 0o7500, "a child inherits its parent's mask");

    drop(vfs_exec_cloexec(child));
    assert_eq!(vfs_apply_umask(0o7777), 0o7500, "exec must preserve the process mask");
    assert_eq!(vfs_umask(0), 0o277, "umask returns the normalized previous mask");

    set_current_process(parent);
    assert_eq!(vfs_apply_umask(0o7777), 0o7500, "the child's mask change is isolated");

    forget_processes(&[parent, child]);
}

/// Tests that the descriptor-table generation advances on every table mutation (open, flag
/// change, close) and is inherited by a forked child.
#[test]
fn generation_advances_on_table_mutations() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7101), ProcessIdentifier::from(0x7102));

    set_current_process(parent);
    let start: u64 = vfs_current_generation();

    // Allocating a descriptor (open) advances the generation.
    let fd: c_int = vfs_alloc_hostfs(50, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");
    let after_open: u64 = vfs_current_generation();
    assert!(after_open > start, "open must advance the generation");

    // A per-descriptor flag change advances the generation.
    let mut flags: FdFlags = FdFlags::default();
    flags.set_close_on_exec(true);
    vfs_set_fd_flags(fd, flags).expect("set fd flags should succeed");
    let after_flags: u64 = vfs_current_generation();
    assert!(after_flags > after_open, "a flag change must advance the generation");

    // A forked child inherits the parent's current generation.
    vfs_fork_clone(parent, child).expect("fork clone should succeed");
    assert_eq!(
        registry_generation(child),
        Some(after_flags),
        "the child must inherit the parent's generation"
    );

    // Closing a descriptor advances the generation. Closing through the child (the parent still
    // holds the shared open file description) exercises the bump without last-reference
    // teardown.
    set_current_process(child);
    vfs_close(fd).expect("close should succeed");
    assert!(vfs_current_generation() > after_flags, "close must advance the generation");

    forget_processes(&[parent, child]);
}

/// Tests that `fork()` gives the child a copy of the parent's open descriptors.
#[test]
fn fork_inherits_open_descriptors() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7001), ProcessIdentifier::from(0x7002));

    // Parent opens a host-backed descriptor.
    set_current_process(parent);
    let fd: c_int = vfs_alloc_hostfs(42, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");
    assert_eq!(vfs_hostfs_remote_fd(fd), Some(42), "parent should see the remote fd");

    // Fork: the child inherits a copy of the descriptor table.
    vfs_fork_clone(parent, child).expect("fork clone should succeed");

    // The child observes the same descriptor referring to the same remote fd.
    set_current_process(child);
    assert_eq!(vfs_hostfs_remote_fd(fd), Some(42), "child should inherit the parent's descriptor");

    forget_processes(&[parent, child]);
}

/// Tests that a forked parent and child share the same open file description (and offset).
#[test]
fn fork_shares_open_file_description() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7011), ProcessIdentifier::from(0x7012));

    // Parent opens a descriptor; it is the sole reference.
    set_current_process(parent);
    let fd: c_int = vfs_alloc_hostfs(43, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");
    assert!(vfs_hostfs_is_last_ref(fd), "the only descriptor should be the last reference");

    // Fork: parent and child now share the open file description.
    vfs_fork_clone(parent, child).expect("fork clone should succeed");
    set_current_process(parent);
    assert!(!vfs_hostfs_is_last_ref(fd), "parent must not be the last reference after fork");
    set_current_process(child);
    assert!(!vfs_hostfs_is_last_ref(fd), "child must not be the last reference after fork");

    // A position advanced through the parent's descriptor is visible through the child's,
    // because both refer to the same open file description (POSIX offset sharing).
    set_fd_virtual_pos(parent, fd, 128);
    assert_eq!(
        fd_virtual_pos(child, fd),
        Some(128),
        "child should observe the parent's file offset"
    );

    forget_processes(&[parent, child]);
}

/// Tests that closing a descriptor in the child leaves the parent's descriptor intact, and that
/// the parent then becomes the last reference (so a host close would be forwarded).
#[test]
fn fork_descriptor_close_is_isolated() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7021), ProcessIdentifier::from(0x7022));

    set_current_process(parent);
    let fd: c_int = vfs_alloc_hostfs(44, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");
    vfs_fork_clone(parent, child).expect("fork clone should succeed");

    // The child closes its descriptor.
    set_current_process(child);
    vfs_close(fd).expect("child close should succeed");
    assert_eq!(vfs_hostfs_remote_fd(fd), None, "child's descriptor should be gone");

    // The parent's descriptor is untouched and is now the sole reference.
    set_current_process(parent);
    assert_eq!(vfs_hostfs_remote_fd(fd), Some(44), "parent's descriptor should remain open");
    assert!(
        vfs_hostfs_is_last_ref(fd),
        "parent should be the last reference once the child has closed"
    );

    forget_processes(&[parent, child]);
}

/// Tests that `fork()` deep-copies the parent's working directory into the child.
#[test]
fn fork_clones_working_directory() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7031), ProcessIdentifier::from(0x7032));

    // Parent sets a working directory, then forks.
    set_current_process(parent);
    set_current_cwd(String::from("/data"));
    vfs_fork_clone(parent, child).expect("fork clone should succeed");

    // The child inherits a copy of the parent's working directory.
    assert_eq!(registry_cwd(child), Some(String::from("/data")), "child should inherit cwd");

    // The copy is independent: changing the child's cwd does not affect the parent.
    set_current_process(child);
    set_current_cwd(String::from("/other"));
    assert_eq!(registry_cwd(parent), Some(String::from("/data")), "parent cwd must be unchanged");
    assert_eq!(registry_cwd(child), Some(String::from("/other")), "child cwd should be updated");

    forget_processes(&[parent, child]);
}

/// Tests that forking from a parent with no recorded state is rejected. `procd` registers every
/// process before it can fork, so an unregistered parent is a contract violation.
#[test]
fn fork_without_parent_state_is_rejected() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7041), ProcessIdentifier::from(0x7042));

    // The parent has never been seen by the VFS.
    assert!(registry_cwd(parent).is_none(), "parent should have no state");

    let err: Fat32Error =
        vfs_fork_clone(parent, child).expect_err("fork from unregistered parent should fail");
    assert_eq!(err, Fat32Error::NotFound, "missing parent state must be rejected");

    // The child must not have been registered as a side effect of the failed fork.
    assert!(registry_cwd(child).is_none(), "child must not be registered on failure");
    assert_eq!(registry_open_fd_count(child), 0, "child should have no open descriptors");

    forget_processes(&[parent, child]);
}

/// Tests that forking into a child that already has recorded state is rejected, so its open
/// descriptors are never silently dropped.
#[test]
fn fork_into_existing_child_is_rejected() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7051), ProcessIdentifier::from(0x7052));

    // The child already holds an open descriptor (e.g., PID reuse or a logic bug).
    set_current_process(child);
    let fd: c_int = vfs_alloc_hostfs(45, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");

    // Forking onto it must fail rather than clobber the existing state.
    let err: Fat32Error =
        vfs_fork_clone(parent, child).expect_err("fork into existing child should fail");
    assert_eq!(err, Fat32Error::AlreadyExists, "existing child state must be rejected");

    // The child's descriptor is left intact.
    set_current_process(child);
    assert_eq!(vfs_hostfs_remote_fd(fd), Some(45), "child's existing descriptor must be preserved");

    forget_processes(&[parent, child]);
}

/// Tests that forking onto a child whose only state is the empty placeholder inserted by
/// [`set_current_process`] succeeds. The child's first request can race ahead of procd's
/// fork-clone notification and lazily create that placeholder; because it holds no descriptors,
/// the clone must overwrite it rather than fail, otherwise the child never inherits the
/// parent's descriptors.
#[test]
fn fork_overwrites_empty_placeholder_child() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7061), ProcessIdentifier::from(0x7062));

    // Parent opens a host-backed descriptor.
    set_current_process(parent);
    let fd: c_int = vfs_alloc_hostfs(46, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");

    // The child's first request reaches the VFS before the fork-clone notification, lazily
    // inserting an empty placeholder state for it.
    set_current_process(child);
    assert_eq!(registry_open_fd_count(child), 0, "placeholder must hold no descriptors");

    // Fork-clone must overwrite the empty placeholder and install the inherited descriptors.
    vfs_fork_clone(parent, child).expect("fork clone over empty placeholder should succeed");

    set_current_process(child);
    assert_eq!(
        vfs_hostfs_remote_fd(fd),
        Some(46),
        "child should inherit the parent's descriptor after the placeholder is overwritten"
    );

    forget_processes(&[parent, child]);
}

/// Tests that forking onto an empty placeholder preserves a working directory the child set
/// before the fork-clone notification arrived. The child's first request can race ahead of
/// procd's notification and `chdir` within the lazily created placeholder; that explicit update
/// must survive the clone rather than reverting to the parent's directory.
#[test]
fn fork_preserves_placeholder_cwd() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7071), ProcessIdentifier::from(0x7072));

    // Parent establishes its own working directory.
    set_current_process(parent);
    set_current_cwd(String::from("/parent"));

    // The child's first request races ahead of the fork-clone notification, lazily creating a
    // placeholder, then the child changes its working directory before the notification lands.
    set_current_process(child);
    set_current_cwd(String::from("/child"));
    assert_eq!(registry_open_fd_count(child), 0, "placeholder must hold no descriptors");

    // Fork-clone overwrites the placeholder but must keep the child's own working directory.
    vfs_fork_clone(parent, child).expect("fork clone over empty placeholder should succeed");
    assert_eq!(
        registry_cwd(child),
        Some(String::from("/child")),
        "child's pre-existing working directory must be preserved"
    );

    forget_processes(&[parent, child]);
}

/// Tests that forking onto a pristine placeholder — one the child created but never `chdir`ed —
/// lets the child inherit the parent's working directory. Only a directory the child actually
/// changed is preserved; an untouched placeholder must not pin the child to the default root.
#[test]
fn fork_pristine_placeholder_inherits_parent_cwd() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7081), ProcessIdentifier::from(0x7082));

    // Parent establishes a non-default working directory.
    set_current_process(parent);
    set_current_cwd(String::from("/parent"));

    // The child's first request creates a placeholder but never changes its directory, leaving
    // it at the default root.
    set_current_process(child);
    assert_eq!(registry_cwd(child), Some(String::from("/")), "placeholder defaults to root");

    // Fork-clone must give the child a copy of the parent's directory, not the default root.
    vfs_fork_clone(parent, child).expect("fork clone over empty placeholder should succeed");
    assert_eq!(
        registry_cwd(child),
        Some(String::from("/parent")),
        "pristine placeholder must inherit the parent's working directory"
    );

    forget_processes(&[parent, child]);
}

// -- pipe descriptor tests ---------------------------------------------------

/// Tests that `vfs_pipe` allocates two descriptors with correct directions and shared identity.
#[test]
fn pipe_allocates_read_and_write_ends() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7101);
    set_current_process(pid);

    let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
    let (rid, r_is_write): (u64, bool) = vfs_pipe_id(read_fd).expect("read fd should be a pipe");
    let (wid, w_is_write): (u64, bool) = vfs_pipe_id(write_fd).expect("write fd should be a pipe");
    assert_eq!(rid, wid, "both ends share one pipe identity");
    assert!(!r_is_write, "read_fd must be the read end");
    assert!(w_is_write, "write_fd must be the write end");

    forget_processes(&[pid]);
}

/// Tests the non-blocking read/write outcome matrix and direction enforcement on a pipe.
#[test]
fn pipe_read_write_outcomes() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7111);
    set_current_process(pid);

    let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");

    // Empty pipe with an open writer: a read would block.
    let mut buf: [u8; 8] = [0u8; 8];
    assert!(
        matches!(vfs_pipe_read(read_fd, &mut buf), Ok(PipeReadOutcome::WouldBlock)),
        "an empty pipe with a writer should block reads"
    );

    // Write then read back.
    assert!(
        matches!(vfs_pipe_write(write_fd, &[1, 2, 3]), Ok(PipeWriteOutcome::Wrote(3))),
        "a write into a pipe with space should succeed"
    );
    match vfs_pipe_read(read_fd, &mut buf) {
        Ok(PipeReadOutcome::Read(n)) => {
            assert_eq!(n, 3, "should read the 3 written bytes");
            assert_eq!(&buf[..3], &[1, 2, 3], "read bytes should match");
        },
        _ => panic!("expected a successful read"),
    }

    // Direction enforcement.
    assert!(vfs_pipe_read(write_fd, &mut buf).is_err(), "reading the write end is rejected");
    assert!(vfs_pipe_write(read_fd, &[0]).is_err(), "writing the read end is rejected");

    forget_processes(&[pid]);
}

/// Tests EOF after the write end closes and a broken pipe after the read end closes.
#[test]
fn pipe_eof_and_broken_pipe() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7121);
    set_current_process(pid);

    // EOF: close the write end, the read end then reports EOF.
    let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
    assert!(vfs_pipe_is_last_ref(write_fd), "the sole write descriptor is the last reference");
    vfs_close(write_fd).expect("closing the write end should succeed");
    let mut buf: [u8; 4] = [0u8; 4];
    assert!(
        matches!(vfs_pipe_read(read_fd, &mut buf), Ok(PipeReadOutcome::Eof)),
        "after the writer closes, the empty pipe reports EOF"
    );
    vfs_close(read_fd).expect("closing the read end should succeed");

    // Broken pipe: close the read end, a write then reports a broken pipe.
    let (read_fd2, write_fd2): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
    vfs_close(read_fd2).expect("closing the read end should succeed");
    assert!(
        matches!(vfs_pipe_write(write_fd2, &[9]), Ok(PipeWriteOutcome::BrokenPipe)),
        "writing with no readers reports a broken pipe"
    );
    vfs_close(write_fd2).expect("closing the write end should succeed");

    forget_processes(&[pid]);
}

/// Tests that `F_SETFL` stores `O_NONBLOCK` and `F_GETFL`/`vfs_get_status_flags` read it back.
#[test]
fn pipe_nonblock_status_flags_round_trip() {
    use ::sysapi::fcntl::{
        file_control_request,
        file_status_flags,
    };
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7131);
    set_current_process(pid);

    let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
    assert_eq!(vfs_get_status_flags(read_fd), 0, "flags default to zero");

    vfs_fcntl(read_fd, file_control_request::F_SETFL, file_status_flags::O_NONBLOCK)
        .expect("F_SETFL should succeed");
    assert_eq!(
        vfs_get_status_flags(read_fd),
        file_status_flags::O_NONBLOCK,
        "O_NONBLOCK should be stored"
    );
    assert_eq!(
        vfs_fcntl(read_fd, file_control_request::F_GETFL, 0).expect("F_GETFL should succeed"),
        file_status_flags::O_NONBLOCK,
        "F_GETFL returns the stored flags"
    );
    // Status flags are per open file description: the write end is unaffected.
    assert_eq!(vfs_get_status_flags(write_fd), 0, "the write end is unaffected");

    forget_processes(&[pid]);
}

/// Tests that process exit surfaces pipe count-to-zero transitions for the process's own ends.
#[test]
fn pipe_process_exit_reports_closures() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7141);
    set_current_process(pid);

    let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
    let (pipe_id, _): (u64, bool) = vfs_pipe_id(read_fd).expect("read fd should be a pipe");
    // Both ends stay open in this single process, so it holds the only reference to each.
    let _ = write_fd;

    let reclaim: ProcessExitReclaim = vfs_process_exit(pid);
    assert!(reclaim.orphaned_hostfs_fds.is_empty(), "no hostfs fds were opened");
    assert_eq!(reclaim.pipe_closures.len(), 2, "both ends are last references at exit");
    assert!(
        reclaim.pipe_closures.iter().all(|c| c.pipe_id == pipe_id),
        "closures must target our pipe"
    );
    assert!(reclaim.pipe_closures.iter().any(|c| c.was_write), "the write end closure is reported");
    assert!(reclaim.pipe_closures.iter().any(|c| !c.was_write), "the read end closure is reported");

    forget_processes(&[pid]);
}

// -- console/socket token and per-descriptor flag tests ----------------------

/// Tests that [`FdFlags`] defaults to empty and that each flag can be set and cleared
/// independently of the other.
#[test]
fn fd_flags_set_and_clear() {
    // A default descriptor carries no flags.
    let mut flags: FdFlags = FdFlags::default();
    assert!(!flags.close_on_exec(), "close-on-exec defaults to off");
    assert!(!flags.close_on_fork(), "close-on-fork defaults to off");

    // Setting one flag must not disturb the other.
    flags.set_close_on_exec(true);
    assert!(flags.close_on_exec(), "close-on-exec should be set");
    assert!(!flags.close_on_fork(), "close-on-fork must remain off");

    flags.set_close_on_fork(true);
    assert!(flags.close_on_exec(), "close-on-exec must remain set");
    assert!(flags.close_on_fork(), "close-on-fork should be set");

    // Clearing one flag must not disturb the other.
    flags.set_close_on_exec(false);
    assert!(!flags.close_on_exec(), "close-on-exec should be cleared");
    assert!(flags.close_on_fork(), "close-on-fork must remain set");
}

/// Tests that [`FdFlags::try_from_bits`] accepts the recognized descriptor-flag bits and rejects
/// any bit outside `FD_CLOEXEC`/`FD_CLOFORK` with [`Fat32Error::InvalidArgument`], matching POSIX
/// `fcntl(F_SETFD)` `EINVAL` semantics rather than silently masking unknown bits.
#[test]
fn fd_flags_try_from_bits_validates() {
    use ::sysapi::fcntl::file_descriptor_flags::{
        FD_CLOEXEC,
        FD_CLOFORK,
    };

    // Recognized bits — alone and combined — are accepted and round-trip exactly.
    assert_eq!(FdFlags::try_from_bits(0).map(FdFlags::bits), Ok(0));
    assert_eq!(FdFlags::try_from_bits(FD_CLOEXEC).map(FdFlags::bits), Ok(FD_CLOEXEC));
    assert_eq!(FdFlags::try_from_bits(FD_CLOFORK).map(FdFlags::bits), Ok(FD_CLOFORK));
    assert_eq!(
        FdFlags::try_from_bits(FD_CLOEXEC | FD_CLOFORK).map(FdFlags::bits),
        Ok(FD_CLOEXEC | FD_CLOFORK),
    );

    // Any bit outside the recognized set is rejected, even alongside a recognized bit.
    assert_eq!(FdFlags::try_from_bits(0x4), Err(Fat32Error::InvalidArgument));
    assert_eq!(FdFlags::try_from_bits(FD_CLOEXEC | 0x4), Err(Fat32Error::InvalidArgument));
}

/// Tests that a [`ConsoleHandle`] reports the standard stream it was created for.
#[test]
fn console_handle_tracks_stream() {
    for stream in [
        ConsoleStream::Stdin,
        ConsoleStream::Stdout,
        ConsoleStream::Stderr,
    ] {
        let handle: ConsoleHandle = ConsoleHandle::new(stream);
        assert_eq!(handle.stream(), stream, "console handle should report its stream");
    }
}

/// Tests that a console descriptor serves the default terminal attributes (canonical mode with
/// echo) and that a `tcsetattr`-style update round-trips through a subsequent `tcgetattr`.
#[test]
fn tty_termios_round_trips_via_console_descriptor() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    vfs_seed_root_console(root);
    set_current_process(root);

    // The default attributes enable canonical line editing with echo.
    let default: Termios = vfs_tty_get_termios(STDIN_FILENO).expect("console get termios");
    assert_ne!(
        default.c_lflag & ::sysapi::termios::ICANON,
        0,
        "the default attributes enable canonical mode"
    );
    assert_ne!(default.c_lflag & ::sysapi::termios::ECHO, 0, "the default attributes enable echo");

    // A modified copy is stored and read back unchanged.
    let mut modified: Termios = default;
    modified.c_lflag &= !(::sysapi::termios::ICANON | ::sysapi::termios::ECHO);
    modified.c_cc[::sysapi::termios::VMIN] = 4;
    vfs_tty_set_termios(STDIN_FILENO, modified).expect("console set termios");
    assert_eq!(
        vfs_tty_get_termios(STDIN_FILENO),
        Ok(modified),
        "the modified attributes must round-trip"
    );

    forget_processes(&[root]);
}

/// Tests that the terminal attributes are shared across the standard streams: a change made
/// through one console descriptor is visible through the others, because they reference one
/// shared terminal object.
#[test]
fn tty_attributes_shared_across_standard_streams() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    vfs_seed_root_console(root);
    set_current_process(root);

    let mut attrs: Termios = vfs_tty_get_termios(STDOUT_FILENO).expect("get via stdout");
    attrs.c_lflag &= !::sysapi::termios::ECHO;
    vfs_tty_set_termios(STDOUT_FILENO, attrs).expect("set via stdout");

    // `tcsetattr(1, ...)` is observable via `tcgetattr(0, ...)` and `tcgetattr(2, ...)`.
    assert_eq!(
        vfs_tty_get_termios(STDIN_FILENO),
        Ok(attrs),
        "stdin observes a change made through stdout"
    );
    assert_eq!(
        vfs_tty_get_termios(STDERR_FILENO),
        Ok(attrs),
        "stderr observes a change made through stdout"
    );

    forget_processes(&[root]);
}

/// Tests that the shared console line discipline is reachable through console descriptors.
#[test]
fn console_line_discipline_is_shared_across_standard_streams() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    vfs_seed_root_console(root);
    set_current_process(root);

    let echo: Vec<u8> = vfs_console_push_input(STDIN_FILENO, b"ab\n").expect("push console input");
    assert_eq!(echo, b"ab\r\n", "canonical input is echoed through the discipline");

    let mut buf: [u8; 8] = [0u8; 8];
    assert_eq!(
        vfs_console_read(STDERR_FILENO, &mut buf),
        Ok(ConsoleReadOutcome::Read(3)),
        "all standard streams share one console input queue"
    );
    assert_eq!(&buf[..3], b"ab\n");

    forget_processes(&[root]);
}

/// Tests that the console device remains usable after init closes its stdin descriptor.
#[test]
fn console_device_outlives_init_stdin() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    let child: ProcessIdentifier = ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 2);
    vfs_seed_root_console(root);
    vfs_fork_clone(root, child).expect("fork clone should succeed");

    set_current_process(root);
    vfs_close(STDIN_FILENO).expect("init stdin close should succeed");
    assert_eq!(vfs_console_device_push_input(b"child\n"), b"child\r\n");

    set_current_process(child);
    let mut buf: [u8; 8] = [0; 8];
    assert_eq!(vfs_console_read(STDIN_FILENO, &mut buf), Ok(ConsoleReadOutcome::Read(6)));
    assert_eq!(&buf[..6], b"child\n");

    forget_processes(&[root, child]);
}

/// Tests that a forked child observes the parent's terminal attributes, because the shared
/// terminal object flows down the fork with the console descriptor slots.
#[test]
fn tty_attributes_inherited_across_fork() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    let child: ProcessIdentifier = ProcessIdentifier::from(0x7444);
    vfs_seed_root_console(root);
    set_current_process(root);
    vfs_fork_clone(root, child).expect("fork clone should succeed");

    // The parent changes the terminal attributes after the fork.
    let mut attrs: Termios = vfs_tty_get_termios(STDOUT_FILENO).expect("parent get");
    attrs.c_cc[::sysapi::termios::VMIN] = 7;
    vfs_tty_set_termios(STDOUT_FILENO, attrs).expect("parent set");

    // The child observes the parent's change through its inherited console descriptors.
    set_current_process(child);
    assert_eq!(
        vfs_tty_get_termios(STDIN_FILENO),
        Ok(attrs),
        "the child inherits the parent's shared terminal"
    );

    forget_processes(&[root, child]);
}

/// Tests that the window size round-trips and is shared across console descriptors, starting
/// from the console default.
#[test]
fn tty_winsize_round_trips_and_is_shared() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    vfs_seed_root_console(root);
    set_current_process(root);

    assert_eq!(
        vfs_tty_get_winsize(STDIN_FILENO),
        Ok(Winsize::console_default()),
        "a console reports the default window size"
    );

    let resized: Winsize = Winsize {
        ws_row: 40,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    vfs_tty_set_winsize(STDOUT_FILENO, resized).expect("set winsize via stdout");
    assert_eq!(
        vfs_tty_get_winsize(STDIN_FILENO),
        Ok(resized),
        "the window size is shared across the standard streams"
    );

    forget_processes(&[root]);
}

/// Tests that the route classification `isatty` answers from and the terminal-control queries
/// are authoritative against the slot table: a console descriptor (and a duplicate of one)
/// routes to the console, while a regular file and a socket route to their own backends and an
/// absent descriptor has no route, and the queries map a non-terminal to `ENOTTY` and an absent
/// descriptor to `EBADF`.
#[test]
fn isatty_and_tty_control_are_authoritative() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    vfs_seed_root_console(root);
    set_current_process(root);

    let file_fd: c_int =
        vfs_alloc_hostfs(50, false, None, file_access_mode::O_RDONLY).expect("hostfs alloc");
    let socket_fd: c_int = vfs_register_socket(2060).expect("socket register");

    assert_eq!(
        vfs_resolve(STDIN_FILENO),
        Some((VfsRoute::Console, STDIN_FILENO)),
        "a console descriptor is a terminal"
    );
    assert_eq!(
        vfs_resolve(file_fd),
        Some((VfsRoute::Vfs, file_fd)),
        "a regular file is not a terminal"
    );
    assert_eq!(
        vfs_resolve(socket_fd),
        Some((VfsRoute::Socket, 2060)),
        "a socket is not a terminal"
    );
    assert_eq!(vfs_resolve(4096), None, "an absent descriptor has no route");

    // A duplicate of a console descriptor remains a terminal.
    let dup_fd: c_int = vfs_dup(STDOUT_FILENO).expect("dup console");
    assert_eq!(
        vfs_resolve(dup_fd),
        Some((VfsRoute::Console, STDOUT_FILENO)),
        "a duplicate of a console descriptor is a terminal"
    );

    // Terminal-control queries distinguish a valid non-terminal from an absent descriptor.
    assert_eq!(
        vfs_tty_get_termios(file_fd),
        Err(TtyError::NotTty),
        "a terminal query on a regular file is ENOTTY"
    );
    assert_eq!(
        vfs_tty_get_termios(socket_fd),
        Err(TtyError::NotTty),
        "a terminal query on a socket is ENOTTY"
    );
    assert_eq!(
        vfs_tty_get_termios(4096),
        Err(TtyError::BadFd),
        "a terminal query on an absent descriptor is EBADF"
    );

    forget_processes(&[root]);
}

/// Tests that a [`SocketHandle`] reports the `networkd` descriptor it was created for.
#[test]
fn socket_handle_tracks_remote_fd() {
    let handle: SocketHandle = SocketHandle::new(7);
    assert_eq!(handle.remote_fd(), 7, "socket handle should report its networkd descriptor");
}

/// Tests that every [`SocketHandle`] receives a nonzero identity of its own, even when `networkd`
/// assigns a later socket the descriptor of an earlier one.
#[test]
fn socket_handle_identity_is_unique() {
    let first: SocketHandle = SocketHandle::new(7);
    let reused: SocketHandle = SocketHandle::new(7);
    assert_ne!(first.socket_id(), 0, "a socket identity is never zero");
    assert_ne!(reused.socket_id(), 0, "a socket identity is never zero");
    assert_ne!(
        first.socket_id(),
        reused.socket_id(),
        "sockets backed by the same networkd descriptor must have distinct identities"
    );
}

/// Tests that [`vfs_register_socket`] allocates a flat slot bound to a socket token, so the
/// descriptor resolves to the `networkd` descriptor it routes to, is reported as a socket, and
/// is the last reference when held alone.
#[test]
fn register_socket_allocates_flat_slot() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7220);
    set_current_process(pid);

    let fd: c_int = vfs_register_socket(2050).expect("register socket should succeed");
    // The first allocation in a fresh process is the lowest free flat number.
    assert_eq!(fd, 0, "a socket is allocated the lowest free flat descriptor");
    assert_eq!(
        vfs_resolve(fd),
        Some((VfsRoute::Socket, 2050)),
        "a socket slot resolves to its networkd descriptor"
    );
    assert_eq!(vfs_socket_remote_fd(fd), Some(2050), "the socket reports its remote fd");
    assert!(vfs_socket_is_last_ref(fd), "the sole reference is the last reference");
    // A non-socket descriptor is not reported as a socket.
    let file_fd: c_int =
        vfs_alloc_hostfs(70, false, None, file_access_mode::O_RDONLY).expect("alloc file");
    assert_eq!(vfs_socket_remote_fd(file_fd), None, "a hostfs descriptor is not a socket");
    assert!(!vfs_socket_is_last_ref(file_fd), "a non-socket is never a socket last reference");

    forget_processes(&[pid]);
}

/// Tests that [`vfs_register_socket_with_flags`] creates the socket slot with the requested
/// per-descriptor flags and open-file status flags, while [`vfs_register_socket`] leaves both
/// cleared.
#[test]
fn register_socket_with_flags_sets_initial_flags() {
    use ::sysapi::fcntl::{
        file_control_request::{
            F_GETFD,
            F_GETFL,
        },
        file_status_flags::O_NONBLOCK,
    };
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7223);
    set_current_process(pid);

    let mut fd_flags: FdFlags = FdFlags::default();
    fd_flags.set_close_on_exec(true);
    fd_flags.set_close_on_fork(true);
    let fd: c_int = vfs_register_socket_with_flags(2090, fd_flags, O_NONBLOCK)
        .expect("register socket with flags should succeed");
    assert_eq!(
        vfs_resolve(fd),
        Some((VfsRoute::Socket, 2090)),
        "a socket slot created with flags resolves to its networkd descriptor"
    );
    assert_eq!(
        vfs_fcntl(fd, F_GETFD, 0),
        Ok(FD_CLOEXEC | FD_CLOFORK),
        "the socket descriptor must be created with the requested descriptor flags"
    );
    assert_eq!(
        vfs_fcntl(fd, F_GETFL, 0),
        Ok(O_NONBLOCK),
        "the socket description must be created with the requested status flags"
    );

    let plain_fd: c_int = vfs_register_socket(2091).expect("register socket should succeed");
    assert_eq!(vfs_fcntl(plain_fd, F_GETFD, 0), Ok(0), "descriptor flags default to clear");
    assert_eq!(vfs_fcntl(plain_fd, F_GETFL, 0), Ok(0), "status flags default to clear");

    forget_processes(&[pid]);
}

/// Tests that [`vfs_register_socket_with_flags`] rejects a status flag other than `O_NONBLOCK`
/// without allocating a descriptor.
#[test]
fn register_socket_with_flags_rejects_unsupported_status_flags() {
    use ::sysapi::fcntl::file_status_flags::{
        O_APPEND,
        O_NONBLOCK,
    };
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7224);
    set_current_process(pid);

    assert_eq!(
        vfs_register_socket_with_flags(2092, FdFlags::default(), O_NONBLOCK | O_APPEND),
        Err(Fat32Error::InvalidArgument),
        "a status flag other than O_NONBLOCK must be rejected"
    );
    assert_eq!(registry_open_fd_count(pid), 0, "a rejected registration must not allocate a slot");

    forget_processes(&[pid]);
}

/// Tests that a close-on-exec socket is released at exec: its `networkd` descriptor is surfaced
/// in [`ProcessExitReclaim::orphaned_socket_fds`] so the daemon closes the endpoint, while a
/// non-close-on-exec socket survives at its original number.
#[test]
fn exec_cloexec_reclaims_socket() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7221);
    set_current_process(pid);

    // A close-on-exec socket and a surviving socket.
    let cloexec_fd: c_int = vfs_register_socket(2060).expect("register cloexec socket");
    let mut flags: FdFlags = FdFlags::default();
    flags.set_close_on_exec(true);
    vfs_set_fd_flags(cloexec_fd, flags).expect("set close-on-exec");
    let survivor_fd: c_int = vfs_register_socket(2061).expect("register surviving socket");

    let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
    assert_eq!(reclaim.orphaned_socket_fds.len(), 1, "only the cloexec socket is reclaimed");
    assert_eq!(
        reclaim.orphaned_socket_fds[0], 2060,
        "the cloexec socket's networkd descriptor is surfaced for closing"
    );
    // The surviving socket is still resolvable at its original number.
    assert_eq!(
        vfs_resolve(survivor_fd),
        Some((VfsRoute::Socket, 2061)),
        "a non-cloexec socket survives exec"
    );

    forget_processes(&[pid]);
}

/// Tests that close-on-exec reports a duplicated socket endpoint only once when every alias is
/// flagged close-on-exec. The daemon sends one close per reported `networkd` descriptor, so a
/// shared socket open-file description must not surface duplicate reclaim records.
#[test]
fn exec_cloexec_reclaims_duplicated_socket_once() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7222);
    set_current_process(pid);

    let fd: c_int = vfs_register_socket(2062).expect("register socket");
    let dup_fd: c_int = vfs_dup(fd).expect("dup socket");
    let mut cloexec: FdFlags = FdFlags::default();
    cloexec.set_close_on_exec(true);
    vfs_set_fd_flags(fd, cloexec).expect("set close-on-exec on original");
    vfs_set_fd_flags(dup_fd, cloexec).expect("set close-on-exec on duplicate");

    let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
    assert_eq!(
        reclaim.orphaned_socket_fds,
        [2062],
        "a duplicated socket endpoint must be reclaimed exactly once"
    );
    assert_eq!(vfs_get_fd_flags(fd), None, "the original alias is dropped");
    assert_eq!(vfs_get_fd_flags(dup_fd), None, "the duplicate alias is dropped");

    forget_processes(&[pid]);
}

/// Tests that `fork()` copies a slot's per-descriptor flags into the child and that the copies
/// are independent, so changing the child's flags does not affect the parent's. This is the
/// per-descriptor semantics the later close-on-exec/close-on-fork plans depend on.
///
/// Close-on-exec is used here rather than close-on-fork precisely because a close-on-fork slot
/// is now *dropped* by the clone (see [`fork_drops_close_on_fork_slots`]), so it would not
/// survive to have its flags inspected in the child.
#[test]
fn fork_copies_independent_descriptor_flags() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7201), ProcessIdentifier::from(0x7202));

    // Parent allocates a console-backed descriptor and marks it close-on-exec.
    set_current_process(parent);
    let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
        .expect("console alloc should succeed");
    assert_eq!(vfs_get_fd_flags(fd), Some(FdFlags::default()), "flags default to empty");
    let mut flags: FdFlags = FdFlags::default();
    flags.set_close_on_exec(true);
    vfs_set_fd_flags(fd, flags).expect("setting flags should succeed");

    // Fork: the child inherits a copy of the descriptor's flags.
    vfs_fork_clone(parent, child).expect("fork clone should succeed");
    set_current_process(child);
    let child_flags: FdFlags = vfs_get_fd_flags(fd).expect("child should inherit the slot");
    assert!(child_flags.close_on_exec(), "child inherits close-on-exec");
    assert!(!child_flags.close_on_fork(), "child must not gain close-on-fork");

    // The flags are per descriptor: clearing the child's must leave the parent's untouched.
    vfs_set_fd_flags(fd, FdFlags::default()).expect("clearing child flags should succeed");
    assert_eq!(vfs_get_fd_flags(fd), Some(FdFlags::default()), "child flags should be cleared");
    set_current_process(parent);
    let parent_flags: FdFlags = vfs_get_fd_flags(fd).expect("parent slot should remain");
    assert!(parent_flags.close_on_exec(), "parent close-on-exec must be unchanged");

    forget_processes(&[parent, child]);
}

/// Tests that `fork()` drops descriptors flagged close-on-fork (`FD_CLOFORK`) from the child
/// while cloning every other slot. This is the close-on-fork semantics this plan wires in: a
/// flagged slot is not inherited, but an unflagged one — including the standard streams — is.
#[test]
fn fork_drops_close_on_fork_slots() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7241), ProcessIdentifier::from(0x7242));

    set_current_process(parent);
    // One descriptor flagged close-on-fork, one left unflagged.
    let clofork_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc should succeed");
    let kept_fd: c_int = vfs_alloc_hostfs(70, false, None, file_access_mode::O_RDONLY)
        .expect("hostfs alloc should succeed");
    let mut clofork: FdFlags = FdFlags::default();
    clofork.set_close_on_fork(true);
    vfs_set_fd_flags(clofork_fd, clofork).expect("setting close-on-fork should succeed");
    let parent_generation: u64 =
        registry_generation(parent).expect("parent generation should be recorded");

    // Fork: the flagged descriptor is dropped in the child; the unflagged one is inherited.
    vfs_fork_clone(parent, child).expect("fork clone should succeed");
    set_current_process(child);
    assert_eq!(vfs_get_fd_flags(clofork_fd), None, "close-on-fork descriptor must be dropped");
    assert!(
        registry_generation(child).expect("child generation should be recorded")
            > parent_generation,
        "dropping a close-on-fork descriptor must advance the child's generation"
    );
    assert_eq!(
        vfs_hostfs_remote_fd(kept_fd),
        Some(70),
        "an unflagged descriptor must still be inherited"
    );

    // The parent keeps both descriptors: close-on-fork affects only the child.
    set_current_process(parent);
    assert!(vfs_get_fd_flags(clofork_fd).is_some(), "parent retains its close-on-fork descriptor");

    forget_processes(&[parent, child]);
}

/// Tests that applying close-on-exec drops only the descriptors flagged `FD_CLOEXEC`, leaves
/// every other descriptor at its number, and advances the table generation so the new image's
/// cache is rebuilt against the post-close-on-exec table.
#[test]
fn exec_cloexec_drops_flagged_and_keeps_others() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7261);
    set_current_process(pid);

    // One descriptor flagged close-on-exec, one left unflagged.
    let cloexec_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc should succeed");
    let kept_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stderr)))
            .expect("console alloc should succeed");
    let mut cloexec: FdFlags = FdFlags::default();
    cloexec.set_close_on_exec(true);
    vfs_set_fd_flags(cloexec_fd, cloexec).expect("setting close-on-exec should succeed");
    let generation_before: u64 = registry_generation(pid).expect("generation should be recorded");

    // A console token holds no external resource, so nothing is reclaimed.
    let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
    assert!(reclaim.orphaned_hostfs_fds.is_empty(), "console drop orphans no hostfs fd");
    assert!(reclaim.pipe_closures.is_empty(), "console drop closes no pipe end");

    // The flagged descriptor is gone; the unflagged one survives at its number.
    assert_eq!(vfs_get_fd_flags(cloexec_fd), None, "close-on-exec descriptor must be dropped");
    assert!(vfs_resolve(cloexec_fd).is_none(), "the dropped descriptor must not resolve");
    assert!(vfs_get_fd_flags(kept_fd).is_some(), "an unflagged descriptor must survive");
    assert!(vfs_resolve(kept_fd).is_some(), "the surviving descriptor must still resolve");
    assert!(
        registry_generation(pid).expect("generation should be recorded") > generation_before,
        "dropping a close-on-exec descriptor must advance the generation"
    );

    forget_processes(&[pid]);
}

/// Tests that applying close-on-exec runs the same last-reference accounting as `close`: a
/// host-backed descriptor for which the process held the final reference is surfaced for
/// closing on hostfsd, and a pipe end whose count reaches zero is surfaced so the daemon can
/// wake its counterpart.
#[test]
fn exec_cloexec_reports_pipe_and_hostfs_reclaim() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7262);
    set_current_process(pid);

    // A pipe whose write end is flagged close-on-exec; the read end stays unflagged.
    let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
    let (pipe_id, _): (u64, bool) = vfs_pipe_id(write_fd).expect("write fd should be a pipe");
    let mut cloexec: FdFlags = FdFlags::default();
    cloexec.set_close_on_exec(true);
    vfs_set_fd_flags(write_fd, cloexec).expect("setting close-on-exec should succeed");
    // A host-backed descriptor, also flagged close-on-exec, for which this process is the sole
    // reference.
    let hostfs_fd: c_int = vfs_alloc_hostfs(70, false, None, file_access_mode::O_RDONLY)
        .expect("hostfs alloc should succeed");
    vfs_set_fd_flags(hostfs_fd, cloexec).expect("setting close-on-exec should succeed");

    let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
    assert_eq!(
        reclaim.orphaned_hostfs_fds.len(),
        1,
        "the host-backed descriptor's remote fd must be reclaimed"
    );
    assert_eq!(reclaim.orphaned_hostfs_fds[0], 70, "the reclaimed remote fd must be 70");
    assert_eq!(reclaim.pipe_closures.len(), 1, "exactly the write end is reclaimed");
    assert_eq!(reclaim.pipe_closures[0].pipe_id, pipe_id, "the closure targets our pipe");
    assert!(reclaim.pipe_closures[0].was_write, "the reclaimed end is the write end");

    // The read end survives because it was not flagged.
    assert!(vfs_resolve(read_fd).is_some(), "the unflagged read end must survive");
    assert_eq!(vfs_get_fd_flags(write_fd), None, "the flagged write end must be dropped");
    assert_eq!(vfs_get_fd_flags(hostfs_fd), None, "the flagged hostfs descriptor must be dropped");

    forget_processes(&[pid]);
}

/// Tests that applying close-on-exec to a process with no flagged descriptor is a no-op: every
/// descriptor survives, nothing is reclaimed, and the generation is left unchanged.
#[test]
fn exec_cloexec_without_flagged_is_noop() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7263);
    set_current_process(pid);

    let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
        .expect("console alloc should succeed");
    let generation_before: u64 = registry_generation(pid).expect("generation should be recorded");

    let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
    assert!(reclaim.orphaned_hostfs_fds.is_empty(), "nothing is reclaimed");
    assert!(reclaim.pipe_closures.is_empty(), "nothing is reclaimed");
    assert!(vfs_resolve(fd).is_some(), "the unflagged descriptor survives");
    assert_eq!(
        registry_generation(pid).expect("generation should be recorded"),
        generation_before,
        "a no-op must not advance the generation"
    );

    forget_processes(&[pid]);
}

/// Tests that applying close-on-exec to an unregistered process reclaims nothing and does not
/// panic.
#[test]
fn exec_cloexec_unknown_pid_is_noop() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7264);
    // Deliberately leave the process unregistered.
    let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
    assert!(reclaim.orphaned_hostfs_fds.is_empty(), "an unknown process reclaims nothing");
    assert!(reclaim.pipe_closures.is_empty(), "an unknown process reclaims nothing");
}

/// Tests that close-on-exec last-reference accounting respects shared open file descriptions: a
/// host-backed description still referenced by a surviving `dup` sibling is not reclaimed when
/// its close-on-exec alias is dropped.
#[test]
fn exec_cloexec_shared_description_not_reclaimed() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7265);
    set_current_process(pid);

    // A host-backed descriptor duplicated onto a second number: the two share one description.
    let hostfs_fd: c_int = vfs_alloc_hostfs(71, false, None, file_access_mode::O_RDONLY)
        .expect("hostfs alloc should succeed");
    let dup_fd: c_int = vfs_dup(hostfs_fd).expect("dup should succeed");
    // Flag only the original close-on-exec; the dup sibling stays unflagged.
    let mut cloexec: FdFlags = FdFlags::default();
    cloexec.set_close_on_exec(true);
    vfs_set_fd_flags(hostfs_fd, cloexec).expect("setting close-on-exec should succeed");

    let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
    assert!(
        reclaim.orphaned_hostfs_fds.is_empty(),
        "a description still held by a dup sibling must not be reclaimed"
    );
    assert_eq!(vfs_get_fd_flags(hostfs_fd), None, "the flagged alias must be dropped");
    assert_eq!(
        vfs_hostfs_remote_fd(dup_fd),
        Some(71),
        "the surviving dup sibling must keep the description"
    );

    forget_processes(&[pid]);
}

/// Tests that `F_SETFD` stores the per-descriptor flags and `F_GETFD` reads them back
/// (<https://github.com/nanvix/nanvix/issues/2604>).
#[test]
fn fcntl_fd_flags_round_trip() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7221);
    set_current_process(pid);

    let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
        .expect("console alloc should succeed");
    let raw_flags: c_int = FD_CLOEXEC | FD_CLOFORK;

    vfs_fcntl(fd, file_control_request::F_SETFD, raw_flags)
        .expect("F_SETFD should store descriptor flags");
    assert_eq!(
        vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("F_GETFD should succeed"),
        raw_flags,
        "F_GETFD should return the flags stored by F_SETFD"
    );

    let stored_flags: FdFlags = vfs_get_fd_flags(fd).expect("fd flags should be recorded");
    assert!(stored_flags.close_on_exec(), "FD_CLOEXEC should be recorded");
    assert!(stored_flags.close_on_fork(), "FD_CLOFORK should be recorded");

    forget_processes(&[pid]);
}

/// Tests that `F_SETFD` updates only the addressed descriptor, so a forked child and its parent
/// keep independent flags (<https://github.com/nanvix/nanvix/issues/2604>).
#[test]
fn fcntl_fd_flags_are_per_descriptor_after_fork() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7231), ProcessIdentifier::from(0x7232));

    set_current_process(parent);
    let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
        .expect("console alloc should succeed");
    vfs_fork_clone(parent, child).expect("fork clone should succeed");

    set_current_process(child);
    vfs_fcntl(fd, file_control_request::F_SETFD, FD_CLOEXEC).expect("child F_SETFD should succeed");
    assert_eq!(
        vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("child F_GETFD should succeed"),
        FD_CLOEXEC,
        "child should see its own descriptor flag"
    );

    set_current_process(parent);
    assert_eq!(
        vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("parent F_GETFD should succeed"),
        0,
        "parent descriptor flags should remain unchanged"
    );

    forget_processes(&[parent, child]);
}

/// Tests that `F_SETFD` updates only the addressed descriptor, so two descriptors that share a
/// single open file description through `dup` keep independent flags: a later update to one
/// never mutates the other (<https://github.com/nanvix/nanvix/issues/2604>).
#[test]
fn fcntl_fd_flags_are_per_descriptor_after_dup() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7521);
    set_current_process(pid);

    // `dup` aliases the source's open file description, so both descriptors address the same
    // underlying file — the precondition for proving the flags stay per descriptor.
    let fd: c_int = vfs_alloc_hostfs(65, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");
    let dup_fd: c_int = vfs_dup(fd).expect("dup should succeed");
    assert!(
        Arc::ptr_eq(&entry_arc(fd).unwrap(), &entry_arc(dup_fd).unwrap()),
        "dup must alias the same open file description"
    );

    // Updating the duplicate must not surface on the source descriptor.
    vfs_fcntl(dup_fd, file_control_request::F_SETFD, FD_CLOFORK)
        .expect("F_SETFD on the duplicate should succeed");
    assert_eq!(
        vfs_fcntl(dup_fd, file_control_request::F_GETFD, 0).expect("duplicate F_GETFD"),
        FD_CLOFORK,
        "the duplicate should report its own descriptor flag"
    );
    assert_eq!(
        vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("source F_GETFD"),
        0,
        "the source descriptor must be unaffected by the duplicate's F_SETFD"
    );

    // The reverse holds too: updating the source leaves the duplicate's flag intact.
    vfs_fcntl(fd, file_control_request::F_SETFD, FD_CLOEXEC)
        .expect("F_SETFD on the source should succeed");
    assert_eq!(
        vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("source F_GETFD"),
        FD_CLOEXEC,
        "the source should report its own descriptor flag"
    );
    assert_eq!(
        vfs_fcntl(dup_fd, file_control_request::F_GETFD, 0).expect("duplicate F_GETFD"),
        FD_CLOFORK,
        "the duplicate's descriptor flag must be unaffected by the source's F_SETFD"
    );

    forget_processes(&[pid]);
}

/// Tests that at process exit a lone socket token surfaces its `networkd` descriptor in
/// [`ProcessExitReclaim::orphaned_socket_fds`] (so the daemon closes the endpoint), while a lone
/// console token contributes nothing. Crucially, neither is mistaken for a hostfs or pipe
/// handle.
#[test]
fn process_exit_reclaims_socket_and_nothing_for_console() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7211);
    set_current_process(pid);

    // A console token and a socket token, each the sole reference held by this process.
    alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stderr)))
        .expect("console alloc should succeed");
    alloc_fd(VfsFileHandle::Socket(SocketHandle::new(99))).expect("socket alloc should succeed");

    // Exit reclaims the socket's networkd descriptor as the sole reference, and nothing for the
    // inert console token or for hostfs/pipe handles.
    let reclaim: ProcessExitReclaim = vfs_process_exit(pid);
    assert!(
        reclaim.orphaned_hostfs_fds.is_empty(),
        "console and socket tokens own no hostfs descriptor"
    );
    assert!(reclaim.pipe_closures.is_empty(), "console and socket tokens trigger no pipe closures");
    assert_eq!(reclaim.orphaned_socket_fds.len(), 1, "exactly one socket descriptor is reclaimed");
    assert_eq!(
        reclaim.orphaned_socket_fds[0], 99,
        "a sole-reference socket surfaces its networkd descriptor for closing"
    );

    forget_processes(&[pid]);
}

/// Tests that process exit reports a duplicated socket endpoint only once. A `dup` sibling
/// shares the same open-file description, so vfsd must not ask `networkd` to close the same
/// remote endpoint once per descriptor slot when the process exits with both aliases live.
#[test]
fn process_exit_reclaims_duplicated_socket_once() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7212);
    set_current_process(pid);

    let fd: c_int = vfs_register_socket(100).expect("register socket");
    let dup_fd: c_int = vfs_dup(fd).expect("dup socket");

    let reclaim: ProcessExitReclaim = vfs_process_exit(pid);
    assert_eq!(
        reclaim.orphaned_socket_fds,
        [100],
        "a duplicated socket endpoint must be reclaimed exactly once"
    );
    assert!(reclaim.orphaned_hostfs_fds.is_empty(), "no hostfs fds were opened");
    assert!(reclaim.pipe_closures.is_empty(), "no pipe ends were opened");
    let _ = dup_fd;

    forget_processes(&[pid]);
}

/// Tests that a socket inherited across `fork()` is reference-counted, not shared: a child
/// closing its inherited copy must not tear down the parent's socket endpoint. This is the core
/// regression guard for `nanvix/nanvix#2609`. Before the child closes, neither process is the
/// last reference (so vfsd would not forward the close to `networkd`); once the child has
/// closed, the parent is the sole, last reference and its socket remains fully resolvable.
#[test]
fn fork_socket_close_is_isolated() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7231), ProcessIdentifier::from(0x7232));

    // Parent creates a socket; it is the sole reference before the fork.
    set_current_process(parent);
    let fd: c_int = vfs_register_socket(2070).expect("register socket should succeed");
    assert!(vfs_socket_is_last_ref(fd), "the only descriptor is the last reference");

    // Fork: parent and child now share the same socket open-file description.
    vfs_fork_clone(parent, child).expect("fork clone should succeed");
    set_current_process(parent);
    assert!(!vfs_socket_is_last_ref(fd), "parent must not be the last reference after fork");
    set_current_process(child);
    assert!(!vfs_socket_is_last_ref(fd), "child must not be the last reference after fork");

    // The child closes its inherited copy. Because it is not the last reference, vfsd would not
    // forward the endpoint close to `networkd`, so the parent's socket survives.
    vfs_close(fd).expect("child close should succeed");
    assert_eq!(vfs_socket_remote_fd(fd), None, "the child's socket descriptor is gone");

    // The parent's socket is untouched and is now the sole, last reference.
    set_current_process(parent);
    assert_eq!(
        vfs_socket_remote_fd(fd),
        Some(2070),
        "the parent's socket must survive the child's close (nanvix/nanvix#2609)"
    );
    assert!(
        vfs_socket_is_last_ref(fd),
        "the parent becomes the last reference once the child has closed"
    );

    forget_processes(&[parent, child]);
}

/// Tests that a child exiting while it still holds an inherited socket does not close the shared
/// endpoint: `vfs_process_exit` surfaces no orphaned socket because the parent still references
/// it. Only when the last holder (the parent) exits is the endpoint reclaimed — exactly once —
/// so there is neither a premature close (`nanvix/nanvix#2609`) nor a leak. This covers the
/// issue's "implicitly when the child exits" case, which the explicit-close acceptance test does
/// not exercise.
#[test]
fn fork_socket_child_exit_preserves_parent() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7241), ProcessIdentifier::from(0x7242));

    // Parent creates a socket and forks; the child inherits a shared reference.
    set_current_process(parent);
    let fd: c_int = vfs_register_socket(2071).expect("register socket should succeed");
    vfs_fork_clone(parent, child).expect("fork clone should succeed");

    // The child exits without closing its inherited socket. The endpoint must not be orphaned,
    // because the parent still holds a reference to it.
    let child_reclaim: ProcessExitReclaim = vfs_process_exit(child);
    assert!(
        child_reclaim.orphaned_socket_fds.is_empty(),
        "a child exiting while the parent holds the socket must not close the endpoint \
         (nanvix/nanvix#2609)"
    );

    // The parent's socket is intact and is now the sole, last reference.
    set_current_process(parent);
    assert_eq!(
        vfs_socket_remote_fd(fd),
        Some(2071),
        "the parent's socket must survive the child's exit"
    );
    assert!(vfs_socket_is_last_ref(fd), "the parent is the last reference after the child exits");

    // When the last holder exits, the endpoint is reclaimed exactly once so `networkd` closes
    // it — no leak.
    let parent_reclaim: ProcessExitReclaim = vfs_process_exit(parent);
    assert_eq!(
        parent_reclaim.orphaned_socket_fds,
        [2071],
        "the last holder's exit closes the endpoint exactly once (no leak)"
    );

    forget_processes(&[parent, child]);
}

// -- root console seeding tests ----------------------------------------------

/// Tests that seeding the root installs console tokens at descriptors `0`/`1`/`2` carrying the
/// matching stream identity, and marks the root's state active. This is what makes vfsd the
/// authoritative bookkeeper of the standard streams.
#[test]
fn seed_root_console_installs_stdio() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();

    vfs_seed_root_console(root);

    let procs = PROCESSES.lock();
    let state = procs
        .get(&root)
        .expect("root state should exist after seeding");
    assert!(state.initialized, "a seeded root must be active");
    assert_eq!(state.slots.len(), 3, "seeding installs exactly the three standard streams");
    for (fd, expected) in [
        (STDIN_FILENO, ConsoleStream::Stdin),
        (STDOUT_FILENO, ConsoleStream::Stdout),
        (STDERR_FILENO, ConsoleStream::Stderr),
    ] {
        let slot = state
            .slots
            .get(&fd)
            .expect("console slot should be installed");
        match &slot.file.lock().handle {
            VfsFileHandle::Console(h) => {
                assert_eq!(h.stream(), expected, "console slot should carry its stream identity")
            },
            _ => panic!("descriptor {fd} should be a console token"),
        }
        assert_eq!(slot.fd_flags, FdFlags::default(), "console slots are not close-on-fork");
    }
    drop(procs);

    forget_processes(&[root]);
}

/// Tests that root-console seeding runs exactly once: the one-shot guard makes a second call —
/// even for a different pid — a no-op that registers nothing. This is what guarantees a forked
/// child, which reaches vfsd only after the root, is never seeded directly.
#[test]
fn seed_runs_once() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    let other: ProcessIdentifier = ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 2);

    vfs_seed_root_console(root);
    // A second seeding attempt (here for a different pid) must do nothing at all.
    vfs_seed_root_console(other);

    assert_eq!(registry_open_fd_count(root), 3, "root keeps its three console descriptors");
    assert!(
        PROCESSES.lock().get(&other).is_none(),
        "a second seeding attempt must not register or seed another process"
    );

    forget_processes(&[root, other]);
}

/// Tests that a forked child whose first request races ahead of the fork-clone notification is
/// not seeded directly. The non-root seeding attempt must leave the child as an uninitialized
/// placeholder and must not consume the root's one-shot latch; once the root is seeded, the
/// fork-clone overwrites the placeholder and installs the shared console slots in the child.
#[test]
fn racing_child_seed_attempt_remains_placeholder() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    let child: ProcessIdentifier = ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 2);

    // This mirrors vfsd's syscall path for a child request that arrives before procd's
    // fork-clone notification: bind the child, then try the root-seeding hook.
    set_current_process(child);
    vfs_seed_root_console(child);
    {
        let procs = PROCESSES.lock();
        let child_state = procs.get(&child).expect("child placeholder should exist");
        assert!(
            !child_state.initialized,
            "a non-root seeding attempt must not activate the racing child"
        );
        assert!(child_state.slots.is_empty(), "racing child must not receive console slots");
        assert!(
            procs.get(&root).is_none(),
            "non-root seeding must not create the root state as a side effect"
        );
    }

    // Now vfsd learns the root pid (e.g., through the fork-clone parent) and seeds it. The
    // clone must overwrite the still-uninitialized child placeholder.
    vfs_seed_root_console(root);
    vfs_fork_clone(root, child).expect("fork clone over racing placeholder should succeed");

    let procs = PROCESSES.lock();
    let root_state = procs.get(&root).expect("root state should exist");
    let child_state = procs.get(&child).expect("child state should exist");
    assert!(child_state.initialized, "fork-cloned child should be active");
    for fd in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
        let root_slot = root_state.slots.get(&fd).expect("root console slot");
        let child_slot = child_state.slots.get(&fd).expect("child console slot");
        assert!(
            Arc::ptr_eq(&root_slot.file, &child_slot.file),
            "child descriptor {fd} must inherit the root's console slot through fork-clone"
        );
    }
    drop(procs);

    forget_processes(&[root, child]);
}

/// Tests that a forked child's console descriptors are the very same open file descriptions as
/// the root's — `Arc`-shared, not freshly created — so the standard streams keep POSIX shared
/// offsets across a fork. Root-only seeding plus cloning is the sole path by which a child
/// receives `0`/`1`/`2`.
#[test]
fn fork_shares_console_slots_with_parent() {
    let _guard = FORK_TEST_GUARD.lock();
    let root: ProcessIdentifier = root_process_identifier();
    let child: ProcessIdentifier = ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 2);

    vfs_seed_root_console(root);
    vfs_fork_clone(root, child).expect("fork clone should succeed");

    let procs = PROCESSES.lock();
    let root_state = procs.get(&root).expect("root state should exist");
    let child_state = procs.get(&child).expect("child state should exist");
    for fd in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
        let root_slot = root_state.slots.get(&fd).expect("root console slot");
        let child_slot = child_state.slots.get(&fd).expect("child console slot");
        assert!(
            Arc::ptr_eq(&root_slot.file, &child_slot.file),
            "child descriptor {fd} must share the root's open file description, not a fresh one"
        );
    }
    drop(procs);

    forget_processes(&[root, child]);
}

/// Tests that an active child that has closed every descriptor is still not overwritten by a
/// fork-clone. The explicit active marker — not a descriptor count — guards the table, so an
/// initialized-but-empty state is correctly refused where the old `is_empty` predicate would
/// have clobbered it.
#[test]
fn fork_into_active_child_with_no_descriptors_is_rejected() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7331), ProcessIdentifier::from(0x7332));

    // The child becomes active by allocating a descriptor, then closes it: its table is empty
    // but it remains a real, initialized process.
    set_current_process(child);
    let fd: c_int = vfs_alloc_hostfs(71, false, None, file_access_mode::O_RDONLY)
        .expect("alloc should succeed");
    vfs_close(fd).expect("close should succeed");
    assert_eq!(registry_open_fd_count(child), 0, "child holds no descriptors after closing");

    // Forking onto it must still be refused: an active state is never an overwritable
    // placeholder, even with an empty table.
    let err: Fat32Error =
        vfs_fork_clone(parent, child).expect_err("fork into active child should fail");
    assert_eq!(err, Fat32Error::AlreadyExists, "active child must be rejected even when empty");

    forget_processes(&[parent, child]);
}

// -- dup / dup2 / F_DUPFD slot-table tests ------------------------------------

/// Tests that `vfs_dup` aliases the source's open file description into the lowest free slot and
/// that the duplicate starts with `FD_CLOEXEC` cleared without disturbing the source's flags.
#[test]
fn dup_aliases_description_and_clears_cloexec() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7501);
    set_current_process(pid);

    let fd: c_int = vfs_alloc_hostfs(60, false, None, file_access_mode::O_RDONLY).expect("alloc");
    // Mark the source close-on-exec so we can prove the duplicate does not inherit it.
    let mut flags: FdFlags = FdFlags::default();
    flags.set_close_on_exec(true);
    vfs_set_fd_flags(fd, flags).expect("set source flags");

    let dup_fd: c_int = vfs_dup(fd).expect("dup");
    assert_eq!(dup_fd, fd + 1, "dup returns the lowest free descriptor");

    // The duplicate shares the source's open file description (same offset).
    {
        let procs = PROCESSES.lock();
        let state = procs.get(&pid).expect("state");
        assert!(
            Arc::ptr_eq(&state.slots[&fd].file, &state.slots[&dup_fd].file),
            "dup must alias the same open file description"
        );
    }
    // POSIX: the duplicate has close-on-exec cleared; the source keeps its own flag.
    assert!(
        !vfs_get_fd_flags(dup_fd).expect("dup flags").close_on_exec(),
        "a dup'd descriptor must start with FD_CLOEXEC cleared"
    );
    assert!(
        vfs_get_fd_flags(fd).expect("source flags").close_on_exec(),
        "the source descriptor's flags must be unaffected by dup"
    );

    forget_processes(&[pid]);
}

/// Tests that `vfs_dup_from` (the `fcntl(F_DUPFD, arg)` primitive) allocates the lowest free
/// descriptor at or above the requested minimum.
#[test]
fn dup_from_allocates_at_or_above_min() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7502);
    set_current_process(pid);

    let fd: c_int = vfs_alloc_hostfs(61, false, None, file_access_mode::O_RDONLY).expect("alloc");
    // Request a duplicate no lower than 10; nothing is allocated there yet, so it lands on 10.
    let high: c_int = vfs_dup_from(fd, 10).expect("dup_from");
    assert_eq!(high, 10, "F_DUPFD must return the lowest free fd >= arg");
    // A second request with the same floor skips the now-occupied 10 and lands on 11.
    let higher: c_int = vfs_dup_from(fd, 10).expect("dup_from again");
    assert_eq!(higher, 11, "the next duplicate skips the occupied floor");
    assert!(
        Arc::ptr_eq(&entry_arc(fd).unwrap(), &entry_arc(high).unwrap()),
        "F_DUPFD must alias the same open file description"
    );

    assert_eq!(
        vfs_dup_from(fd, -1).unwrap_err(),
        Fat32Error::InvalidArgument,
        "F_DUPFD with a negative floor must be rejected"
    );

    forget_processes(&[pid]);
}

/// Tests that `vfs_dup2` re-points `newfd` at `oldfd`'s description across backends and that the
/// previous occupant of `newfd` is released.
#[test]
fn dup2_repoints_across_backends() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7503);
    set_current_process(pid);

    // A console descriptor and an independent host-backed file descriptor.
    let console_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc");
    let file_fd: c_int =
        vfs_alloc_hostfs(62, false, None, file_access_mode::O_RDONLY).expect("file alloc");
    // The file is the sole reference before dup2.
    assert!(vfs_hostfs_is_last_ref(file_fd), "file should be the last reference pre-dup2");

    // dup2(file_fd, console_fd): the console slot now aliases the file's description — the
    // cross-backend redirection the old split model could not express.
    let ret: c_int = vfs_dup2(file_fd, console_fd).expect("dup2");
    assert_eq!(ret, console_fd, "dup2 returns newfd");
    assert!(
        Arc::ptr_eq(&entry_arc(file_fd).unwrap(), &entry_arc(console_fd).unwrap()),
        "newfd must alias oldfd's open file description after dup2"
    );
    // Resolution now reports the former console descriptor as a vfsd-served file.
    assert_eq!(
        vfs_resolve(console_fd),
        Some((VfsRoute::Vfs, console_fd)),
        "the redirected descriptor must resolve to the file backend"
    );

    forget_processes(&[pid]);
}

/// Tests that `dup2(fd, fd)` is a no-op that returns `fd` and leaves the table generation
/// unchanged, while `dup2` with an invalid `oldfd` or an out-of-range `newfd` is rejected.
#[test]
fn dup2_noop_and_invalid_arguments() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7504);
    set_current_process(pid);

    let fd: c_int = vfs_alloc_hostfs(63, false, None, file_access_mode::O_RDONLY).expect("alloc");
    let before: u64 = vfs_current_generation();
    assert_eq!(vfs_dup2(fd, fd).expect("dup2 self"), fd, "dup2(fd, fd) returns fd");
    assert_eq!(vfs_current_generation(), before, "a no-op dup2 must not advance the generation");

    // An invalid source descriptor is rejected.
    assert_eq!(vfs_dup2(4096, fd).unwrap_err(), Fat32Error::InvalidFd, "bad oldfd rejected");
    // A newfd outside the flat namespace is rejected.
    assert_eq!(
        vfs_dup2(fd, MAX_OPEN_FDS).unwrap_err(),
        Fat32Error::InvalidFd,
        "newfd outside the flat namespace is rejected"
    );

    forget_processes(&[pid]);
}

/// Tests that when `dup2` displaces the last reference to a host-backed description, that
/// reference is dropped so the caller (vfsd) becomes responsible for reclaiming the remote
/// handle. The replaced slot must no longer reference the old description.
#[test]
fn dup2_releases_displaced_last_reference() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7505);
    set_current_process(pid);

    let src_fd: c_int =
        vfs_alloc_hostfs(64, false, None, file_access_mode::O_RDONLY).expect("src alloc");
    let victim_fd: c_int =
        vfs_alloc_hostfs(65, false, None, file_access_mode::O_RDONLY).expect("victim alloc");
    // The victim is the only reference to remote fd 65, so the daemon would inspect it for
    // reclaim before calling dup2.
    assert!(vfs_hostfs_is_last_ref(victim_fd), "victim is the last reference pre-dup2");
    assert_eq!(vfs_hostfs_remote_fd(victim_fd), Some(65));

    vfs_dup2(src_fd, victim_fd).expect("dup2");
    // The victim slot now points at the source's remote fd, and the displaced description is
    // gone (its Arc was dropped by dup2).
    assert_eq!(
        vfs_hostfs_remote_fd(victim_fd),
        Some(64),
        "the displaced slot must now alias the source's remote handle"
    );

    forget_processes(&[pid]);
}

/// Reproduces the ash I/O-redirection save/restore dance on the console:
///   savedfd = fcntl(1, F_DUPFD, 10);  // park the console stdout
///   dup2(file, 1);                    // redirect stdout to a file
///   dup2(savedfd, 1);                 // restore stdout to the console
///   close(savedfd);                   // drop the parked alias
/// After the dance, fd 1 must resolve back to the console.
#[test]
fn dup2_console_save_restore_dance_restores_console() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7506);
    set_current_process(pid);

    // Seed console stdin/stdout so stdout lands on fd 1 (the lowest free numbers).
    let stdin_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdin)))
            .expect("stdin alloc");
    assert_eq!(stdin_fd, STDIN_FILENO);
    let stdout_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("stdout alloc");
    assert_eq!(stdout_fd, STDOUT_FILENO, "console stdout must be fd 1");

    // A file to redirect into.
    let file_fd: c_int =
        vfs_alloc_hostfs(70, false, None, file_access_mode::O_RDONLY).expect("file alloc");

    // savedfd = fcntl(1, F_DUPFD, 10): park the console stdout above the low range.
    let saved_fd: c_int = vfs_dup_from(stdout_fd, 10).expect("savefd");
    assert_eq!(saved_fd, 10);
    assert_eq!(
        vfs_resolve(saved_fd),
        Some((VfsRoute::Console, STDOUT_FILENO)),
        "the parked fd must alias the console"
    );

    // dup2(file, 1): redirect stdout to the file.
    vfs_dup2(file_fd, stdout_fd).expect("redirect");
    assert_eq!(
        vfs_resolve(stdout_fd).map(|(r, _)| r),
        Some(VfsRoute::Vfs),
        "fd 1 must route to the file while redirected"
    );

    // dup2(savedfd, 1): restore stdout to the console.
    vfs_dup2(saved_fd, stdout_fd).expect("restore");
    // close(savedfd): drop the parked alias.
    vfs_close(saved_fd).expect("close saved");

    // fd 1 must once again route to the console.
    assert_eq!(
        vfs_resolve(stdout_fd),
        Some((VfsRoute::Console, STDOUT_FILENO)),
        "after restore+close, fd 1 must resolve back to the console"
    );

    forget_processes(&[pid]);
}

// -- synthetic metadata tests -------------------------------------------------

/// Tests that a hostfs descriptor uses the shared hostfs identity allocation.
#[test]
fn fstat_hostfs_uses_hostfs_identity() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7508);
    set_current_process(pid);

    let fd: c_int =
        vfs_alloc_hostfs(80, false, None, file_access_mode::O_RDONLY).expect("hostfs alloc");
    let mut st: ::sysapi::sys_stat::stat = Default::default();
    vfs_fstat(fd, &mut st).expect("fstat hostfs");

    assert_eq!(st.st_dev, FilesystemDeviceId::HostFs.into());
    assert_eq!(st.st_ino, HostFsInodeId::Fallback.into());

    forget_processes(&[pid]);
}

/// Tests that `vfs_fstat` reports a console descriptor as a character device with a stable
/// identity, and that a `dup`'d console descriptor shares that identity.
#[test]
fn fstat_console_reports_stable_character_device() {
    use ::sysapi::sys_stat::file_type;
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7507);
    set_current_process(pid);

    let console_fd: c_int =
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc");

    let mut st: ::sysapi::sys_stat::stat = Default::default();
    vfs_fstat(console_fd, &mut st).expect("fstat console");
    assert_eq!(
        st.st_mode & file_type::S_IFMT,
        file_type::S_IFCHR,
        "a console descriptor must report as a character device"
    );
    assert_eq!(st.st_size, 0, "a console has no size");
    let (dev, ino): (_, _) = (st.st_dev, st.st_ino);

    // fstat is stable across calls.
    let mut st2: ::sysapi::sys_stat::stat = Default::default();
    vfs_fstat(console_fd, &mut st2).expect("fstat console again");
    assert_eq!((st2.st_dev, st2.st_ino), (dev, ino), "console identity must be stable");

    // A dup'd console descriptor shares the same identity.
    let dup_fd: c_int = vfs_dup(console_fd).expect("dup console");
    let mut st3: ::sysapi::sys_stat::stat = Default::default();
    vfs_fstat(dup_fd, &mut st3).expect("fstat dup'd console");
    assert_eq!(
        (st3.st_dev, st3.st_ino),
        (dev, ino),
        "a dup'd console descriptor must share its source's identity"
    );

    forget_processes(&[pid]);
}

/// Device and inode numbers that `vfs_fstat` reports for a descriptor, followed by the
/// `(tv_sec, tv_nsec)` pairs of its access, modification, and status change times.
type StatIdentity = ((dev_t, ino_t), [(::sysapi::sys_types::time_t, ::sysapi::ffi::c_long); 3]);

/// Returns the identity and timestamps that `vfs_fstat` reports for `fd` in the current process.
fn stat_identity(fd: c_int) -> StatIdentity {
    let mut st: ::sysapi::sys_stat::stat = Default::default();
    vfs_fstat(fd, &mut st).expect("fstat should succeed");
    let times: [(::sysapi::sys_types::time_t, ::sysapi::ffi::c_long); 3] =
        [st.st_atim, st.st_mtim, st.st_ctim].map(|time| (time.tv_sec, time.tv_nsec));
    ((st.st_dev, st.st_ino), times)
}

/// Tests that `vfs_fstat` reports a socket descriptor as a socket rather than as a regular file:
/// the socket type with every permission bit set, no size or blocks, the synthetic sockfs device,
/// the identity allocated when the socket was registered, and its registration time for all three
/// times.
#[test]
fn fstat_socket_reports_socket_metadata() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x7509);
    set_current_process(pid);

    let fd: c_int = vfs_register_socket(2080).expect("register socket");
    let file: OpenFile = entry_arc(fd).expect("the socket slot should exist");
    let (socket_id, created): (u64, i64) = match &file.lock().handle {
        VfsFileHandle::Socket(handle) => (handle.socket_id(), handle.created()),
        _ => panic!("a registered socket should hold a socket handle"),
    };

    let mut st: ::sysapi::sys_stat::stat = Default::default();
    vfs_fstat(fd, &mut st).expect("fstat socket");
    assert_eq!(
        st.st_mode & file_type::S_IFMT,
        file_type::S_IFSOCK,
        "a socket descriptor must report as a socket"
    );
    assert_eq!(
        st.st_mode & !file_type::S_IFMT,
        file_mode::S_IRWXU | file_mode::S_IRWXG | file_mode::S_IRWXO,
        "a socket must grant every permission, as Linux sockets do"
    );
    assert_eq!(st.st_size, 0, "a socket has no size");
    assert_eq!(st.st_blocks, 0, "a socket has no blocks");
    assert_eq!(st.st_nlink, 1, "a socket has a single link");
    assert_eq!(
        st.st_dev,
        FilesystemDeviceId::SockFs.into(),
        "a socket must be on the sockfs device"
    );
    assert_eq!(st.st_ino, socket_id, "a socket's inode must be its registered identity");
    for (name, time) in [
        ("access", st.st_atim),
        ("modification", st.st_mtim),
        ("status change", st.st_ctim),
    ] {
        assert_eq!(
            (time.tv_sec, time.tv_nsec),
            (created, 0),
            "a socket must report its registration time as its {name} time"
        );
    }

    forget_processes(&[pid]);
}

/// Tests that a socket's identity and timestamps are stable across `vfs_fstat` calls and shared by
/// every descriptor that refers to its open description: descriptors duplicated with `dup` and
/// `dup2`, and descriptors that a child inherits across `fork`.
#[test]
fn fstat_socket_identity_is_shared_by_descriptors() {
    let _guard = FORK_TEST_GUARD.lock();
    let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
        (ProcessIdentifier::from(0x7511), ProcessIdentifier::from(0x7512));
    set_current_process(parent);

    let fd: c_int = vfs_register_socket(2081).expect("register socket");
    let original: StatIdentity = stat_identity(fd);
    assert_eq!(stat_identity(fd), original, "repeated fstat calls on a socket must agree");

    let dup_fd: c_int = vfs_dup(fd).expect("dup socket");
    assert_eq!(
        stat_identity(dup_fd),
        original,
        "a dup'd socket descriptor must share its source's identity and times"
    );
    let dup2_fd: c_int = vfs_dup2(fd, dup_fd + 1).expect("dup2 socket");
    assert_eq!(
        stat_identity(dup2_fd),
        original,
        "a dup2'd socket descriptor must share its source's identity and times"
    );

    vfs_fork_clone(parent, child).expect("fork clone should succeed");
    set_current_process(child);
    assert_eq!(
        stat_identity(fd),
        original,
        "a fork-inherited socket descriptor must share its parent's identity and times"
    );

    forget_processes(&[parent, child]);
}

/// Tests that distinct live sockets report distinct `(st_dev, st_ino)` pairs, and that a socket
/// that reuses both the `networkd` descriptor and the flat descriptor of a closed socket does not
/// inherit the closed socket's identity.
#[test]
fn fstat_distinct_sockets_have_distinct_identities() {
    let _guard = FORK_TEST_GUARD.lock();
    let pid: ProcessIdentifier = ProcessIdentifier::from(0x750a);
    set_current_process(pid);

    let first_fd: c_int = vfs_register_socket(2082).expect("register first socket");
    let second_fd: c_int = vfs_register_socket(2083).expect("register second socket");
    let (first, _): StatIdentity = stat_identity(first_fd);
    let (second, _): StatIdentity = stat_identity(second_fd);
    assert_ne!(first, second, "distinct live sockets must report distinct identities");

    // `networkd` may assign a closed socket's descriptor to a later socket, which the lowest-free
    // allocator also places at the closed socket's flat descriptor.
    vfs_close(first_fd).expect("close first socket");
    let reused_fd: c_int = vfs_register_socket(2082).expect("register reusing socket");
    assert_eq!(reused_fd, first_fd, "the new socket should reuse the closed socket's descriptor");
    let (reused, _): StatIdentity = stat_identity(reused_fd);
    assert_ne!(reused, first, "a new socket must not inherit the identity of a closed socket");
    assert_ne!(reused, second, "distinct live sockets must report distinct identities");

    forget_processes(&[pid]);
}
