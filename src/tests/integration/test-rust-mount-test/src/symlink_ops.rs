// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Symbolic link tests over hostfs: symlink, readlink, lstat.
//!
//! These tests exercise the `/mnt` mount point routing of symbolic-link operations
//! through vfsd → hostfsd. On hosts where symbolic links cannot be created (notably
//! Windows without Developer Mode), the host returns `ENOTSUP`; the tests detect
//! this and skip gracefully so the rest of the mount-test suite remains usable.

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    FileType,
    RegularFile,
};

pub fn test() -> Result<(), Error> {
    // Exercise the daemon route directly; the compatibility symlink wrapper may reject locally.
    test_physical_parent_resolution()?;
    // Create a regular file as the symlink target so readlink/lstat have something
    // to refer to (the target is stored verbatim and not validated at create time).
    let target_path: FileSystemPath = FileSystemPath::new("/mnt/symlink-target.txt")?;
    let perms: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);
    {
        let mut f: RegularFile = FileSystem::create_regular_file(&target_path, Some(perms))?;
        f.write(b"hello-symlink")?;
    }

    // Probe whether the host supports symlink creation by attempting a small one
    // and skipping the remaining tests on `OperationNotSupported`.
    let probe_link: FileSystemPath = FileSystemPath::new("/mnt/symlink-probe")?;
    let _ = ::syscall::safe::fs::unlink(&probe_link);
    let probe_target: FileSystemPath = FileSystemPath::new("symlink-target.txt")?;
    match ::syscall::safe::fs::symlink(&probe_target, &probe_link) {
        Ok(()) => {
            let _ = ::syscall::safe::fs::unlink(&probe_link);
        },
        Err(e) if e.code == ErrorCode::OperationNotSupported => {
            ::syslog::warn!(
                "mount-test: [SKIP] host does not support symlinks (e.g., Windows without \
                 Developer Mode); skipping symlink_ops tests"
            );
            // Clean up the target file before returning.
            let _ = ::syscall::safe::fs::unlink(&target_path);
            return Ok(());
        },
        Err(e) => return Err(e),
    }

    // Inline-path tests: all use short link names whose wire path (after stripping
    // the `/mnt/` mount prefix) fits within `MAX_INLINE_PATH_LEN`, so readlink/lstat take the
    // single-message fast path.
    test_symlink_create_readlink()?;
    test_lstat_does_not_follow()?;
    test_stat_follows_symlink()?;
    test_unlink_removes_symlink_not_target(&target_path)?;
    test_symlink_to_nonexistent_target()?;

    // Multi-part-path test: uses a link name whose wire path exceeds
    // `MAX_INLINE_PATH_LEN`, forcing readlink/lstat through the multi-part
    // assembler instead of the inline single-message form.
    test_long_path_multipart()?;

    // Final cleanup of the target file.
    ::syscall::safe::fs::unlink(&target_path)?;
    Ok(())
}

/// Tests creating a symlink and reading its target back verbatim.
fn test_symlink_create_readlink() -> Result<(), Error> {
    let link_path: FileSystemPath = FileSystemPath::new("/mnt/symlink-readlink.lnk")?;
    let target: FileSystemPath = FileSystemPath::new("symlink-target.txt")?;
    let _ = ::syscall::safe::fs::unlink(&link_path);

    ::syscall::safe::fs::symlink(&target, &link_path)?;
    ::syslog::info!("mount-test: [PASS] symlink /mnt/symlink-readlink.lnk -> symlink-target.txt");

    match ::syscall::safe::fs::readlink(&link_path) {
        Ok(read) => {
            if read.as_str() != target.as_str() {
                panic!("readlink: expected '{}', got '{}'", target.as_str(), read.as_str());
            }
        },
        Err(e) => panic!("readlink failed: {e:?}"),
    }
    ::syslog::info!("mount-test: [PASS] readlink returns stored target");

    ::syscall::safe::fs::unlink(&link_path)?;
    Ok(())
}

/// Tests that lstat reports SymbolicLink for the link itself.
fn test_lstat_does_not_follow() -> Result<(), Error> {
    let link_path: FileSystemPath = FileSystemPath::new("/mnt/symlink-lstat.lnk")?;
    let target: FileSystemPath = FileSystemPath::new("symlink-target.txt")?;
    let _ = ::syscall::safe::fs::unlink(&link_path);

    ::syscall::safe::fs::symlink(&target, &link_path)?;

    let attr = ::syscall::safe::fs::lstat(&link_path)?;
    if attr.file_type() != FileType::SymbolicLink {
        panic!("lstat: expected SymbolicLink, got {:?}", attr.file_type());
    }
    ::syslog::info!("mount-test: [PASS] lstat reports SymbolicLink without following");

    ::syscall::safe::fs::unlink(&link_path)?;
    Ok(())
}

/// Tests that `stat` follows a symlink while `lstat` does not, exercising both
/// hostfs stat paths (following pathstat vs. no-follow lstat) over the same link.
fn test_stat_follows_symlink() -> Result<(), Error> {
    let link_path: FileSystemPath = FileSystemPath::new("/mnt/symlink-stat.lnk")?;
    let target: FileSystemPath = FileSystemPath::new("symlink-target.txt")?;
    let _ = ::syscall::safe::fs::unlink(&link_path);

    ::syscall::safe::fs::symlink(&target, &link_path)?;

    // No-follow path: lstat reports the link itself.
    let lattr = ::syscall::safe::fs::lstat(&link_path)?;
    if lattr.file_type() != FileType::SymbolicLink {
        panic!("lstat: expected SymbolicLink, got {:?}", lattr.file_type());
    }

    // Following path: stat resolves the link and reports the target's type.
    let sattr = ::syscall::safe::fs::stat(&link_path)?;
    if sattr.file_type() != FileType::RegularFile {
        panic!("stat: expected RegularFile (followed target), got {:?}", sattr.file_type());
    }
    ::syslog::info!("mount-test: [PASS] stat follows symlink while lstat does not");

    ::syscall::safe::fs::unlink(&link_path)?;
    Ok(())
}

/// Tests that unlink on a symlink removes the link itself, not its target.
fn test_unlink_removes_symlink_not_target(target_path: &FileSystemPath) -> Result<(), Error> {
    let link_path: FileSystemPath = FileSystemPath::new("/mnt/symlink-unlink.lnk")?;
    let target: FileSystemPath = FileSystemPath::new("symlink-target.txt")?;
    let _ = ::syscall::safe::fs::unlink(&link_path);

    ::syscall::safe::fs::symlink(&target, &link_path)?;
    ::syscall::safe::fs::unlink(&link_path)?;

    // The target must still exist.
    let attr = ::syscall::safe::fs::lstat(target_path)?;
    if attr.file_type() == FileType::SymbolicLink {
        panic!("unlink removed the target instead of the symlink");
    }
    ::syslog::info!("mount-test: [PASS] unlink removes symlink, not its target");

    Ok(())
}

/// Tests creating a symlink whose target does not exist (POSIX allows this).
fn test_symlink_to_nonexistent_target() -> Result<(), Error> {
    let link_path: FileSystemPath = FileSystemPath::new("/mnt/symlink-dangling.lnk")?;
    let target: FileSystemPath = FileSystemPath::new("does-not-exist.txt")?;
    let _ = ::syscall::safe::fs::unlink(&link_path);

    // Symlink creation must succeed even if the target does not exist.
    ::syscall::safe::fs::symlink(&target, &link_path)?;

    // lstat on the dangling link must succeed and report SymbolicLink.
    let attr = ::syscall::safe::fs::lstat(&link_path)?;
    if attr.file_type() != FileType::SymbolicLink {
        panic!("dangling symlink: expected SymbolicLink, got {:?}", attr.file_type());
    }

    // readlink on the dangling link must return the stored target.
    match ::syscall::safe::fs::readlink(&link_path) {
        Ok(read) => {
            if read.as_str() != target.as_str() {
                panic!(
                    "readlink dangling: expected '{}', got '{}'",
                    target.as_str(),
                    read.as_str()
                );
            }
        },
        Err(e) => panic!("readlink dangling failed: {e:?}"),
    }
    ::syslog::info!("mount-test: [PASS] dangling symlink: create, lstat, readlink all OK");

    ::syscall::safe::fs::unlink(&link_path)?;
    Ok(())
}

/// A symlink ancestor followed by `..` must be interpreted on the host, not collapsed in vfsd.
fn test_physical_parent_resolution() -> Result<(), Error> {
    use ::alloc::format;
    use ::hostfs_api::MAX_INLINE_PATH_LEN;
    use ::sysapi::{
        fcntl::{
            atflags::{
                AT_FDCWD,
                AT_REMOVEDIR,
            },
            file_access_mode::O_RDONLY,
            file_creation_flags::O_DIRECTORY,
        },
        sys_stat::{
            file_mode::S_IRWXU,
            stat,
        },
    };
    use ::syscall::{
        fcntl::{
            openat,
            unlinkat,
        },
        safe::{
            fs::lstat,
            RegularFileOpenFlags,
        },
        sys::stat::{
            fstatat,
            mkdir,
        },
        unistd::{
            chdir,
            close,
            getcwd,
            read,
            readlinkat,
            symlinkat,
        },
    };
    use ::syslog::{
        info,
        warn,
    };

    const ROOT: &str = "/mnt/route-links";
    const PHYSICAL: &str = "/mnt/route-links/p";
    const CHILD: &str = "/mnt/route-links/p/sub";
    const TARGET: &str = "./p//sub/../sub";
    const LINK_TARGET: &str = "../missing//./target";
    const DATA: &[u8] = b"physical-parent";
    const PROBE: &str = "/mnt/typed-symlink-probe";

    match symlinkat("missing-target", AT_FDCWD, PROBE) {
        Ok(()) => unlinkat(AT_FDCWD, PROBE, 0)?,
        Err(error) if error.code == ErrorCode::OperationNotSupported => {
            warn!("mount-test: [SKIP] host symlinkat is not supported");
            return Ok(());
        },
        Err(error) => return Err(error),
    }

    for directory in [ROOT, PHYSICAL, CHILD] {
        mkdir(directory, S_IRWXU)?;
    }
    let permissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);
    for (path, content) in [
        ("/mnt/route-links/marker", b"lexical".as_slice()),
        ("/mnt/route-links/p/marker", DATA),
    ] {
        let path = FileSystemPath::new(path)?;
        let mut file = FileSystem::create_regular_file(&path, Some(permissions))?;
        file.write(content)?;
    }
    symlinkat(LINK_TARGET, AT_FDCWD, "/mnt/route-links/p/link")?;
    symlinkat("lexical-target", AT_FDCWD, "/mnt/route-links/link")?;

    for (name, multipart) in [
        ("a", false),
        ("long-directory-alias-for-multipart-routing", true),
    ] {
        let alias = format!("{ROOT}/{name}");
        symlinkat(TARGET, AT_FDCWD, &alias)?;
        let mut target_buf = [0; 64];
        let count = readlinkat(AT_FDCWD, &alias, &mut target_buf)? as usize;
        assert_eq!(&target_buf[..count], TARGET.as_bytes(), "symlink targets must stay verbatim");

        let path = FileSystemPath::new(&format!("{alias}/../marker"))?;
        let wire_len = path.as_str().len() - "/mnt/".len();
        assert_eq!(wire_len > MAX_INLINE_PATH_LEN, multipart);
        let mut st = stat::default();
        fstatat(AT_FDCWD, path.as_str(), &mut st, 0)?;
        assert_eq!(st.st_size, DATA.len() as i64, "stat must select the physical parent");
        let file = FileSystem::open_regular_file(&path, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf = [0; 32];
        let count = file.read(&mut buf)?;
        assert_eq!(&buf[..count], DATA, "symlink/.. must select the physical parent");
        drop(file);

        let path = FileSystemPath::new(&format!("{alias}/../link"))?;
        let attr = lstat(&path)?;
        assert_eq!(attr.file_type(), FileType::SymbolicLink);
        let count = readlinkat(AT_FDCWD, path.as_str(), &mut target_buf)? as usize;
        assert_eq!(&target_buf[..count], LINK_TARGET.as_bytes());

        // Open completion must retain the uncollapsed guest path for later dirfd anchoring.
        let dirfd = openat(AT_FDCWD, &alias, O_RDONLY | O_DIRECTORY, 0)?;
        let fd = openat(dirfd, "../marker", O_RDONLY, 0)?;
        let count = read(fd, &mut buf)? as usize;
        assert_eq!(&buf[..count], DATA, "directory bookkeeping must not normalize host symlinks");
        close(fd)?;
        close(dirfd)?;

        // The same spelling must survive deferred chdir completion and later cwd-relative calls.
        let previous_cwd = getcwd()?;
        let spelled_cwd = format!("{alias}//.././");
        chdir(&spelled_cwd)?;
        let reported_cwd = getcwd()?;
        let fd = openat(AT_FDCWD, "marker", O_RDONLY, 0)?;
        let count = read(fd, &mut buf)? as usize;
        close(fd)?;
        let mut st = stat::default();
        fstatat(AT_FDCWD, "marker", &mut st, 0)?;
        let failed_chdir = chdir("marker");
        let cwd_after_failure = getcwd()?;
        chdir(&previous_cwd)?;
        assert_eq!(&buf[..count], DATA, "cwd-relative open must use the host-selected directory");
        assert_eq!(st.st_size, DATA.len() as i64, "cwd-relative stat must use the same directory");
        assert_eq!(reported_cwd, spelled_cwd, "getcwd must retain a usable hostfs spelling");
        assert_eq!(failed_chdir.err().map(|error| error.code), Some(ErrorCode::InvalidDirectory));
        assert_eq!(cwd_after_failure, reported_cwd, "failed chdir must not change cwd");
        unlinkat(AT_FDCWD, &alias, 0)?;
    }

    for path in [
        "/mnt/route-links/p/link",
        "/mnt/route-links/link",
        "/mnt/route-links/p/marker",
        "/mnt/route-links/marker",
    ] {
        unlinkat(AT_FDCWD, path, 0)?;
    }
    for directory in [CHILD, PHYSICAL, ROOT] {
        unlinkat(AT_FDCWD, directory, AT_REMOVEDIR)?;
    }
    info!("mount-test: [PASS] physical symlink parent resolution, inline and multipart");
    Ok(())
}

/// Tests that readlink/lstat/stat work over the multi-part wire format.
///
/// Uses a link path whose wire form (after the `/mnt/` prefix is stripped) exceeds
/// `hostfs_api::MAX_INLINE_PATH_LEN`, so vfsd sends the request through the multi-part assembler
/// rather than the inline single-message fast path. This
/// covers both no-follow (`lstat`) and follow (`stat`) multi-part dispatch paths.
fn test_long_path_multipart() -> Result<(), Error> {
    // Stripped wire path: "long-symlink-name-padding-AAAAAAAAAAAA.lnk".
    let link_path: FileSystemPath =
        FileSystemPath::new("/mnt/long-symlink-name-padding-AAAAAAAAAAAA.lnk")?;
    let target: FileSystemPath = FileSystemPath::new("symlink-target.txt")?;
    let _ = ::syscall::safe::fs::unlink(&link_path);

    // Symlink creation always uses the multi-part wire format; here we exercise
    // it together with the multi-part readlink and lstat request paths.
    ::syscall::safe::fs::symlink(&target, &link_path)?;

    let read = ::syscall::safe::fs::readlink(&link_path)?;
    if read.as_str() != target.as_str() {
        panic!(
            "readlink (multi-part request): expected '{}', got '{}'",
            target.as_str(),
            read.as_str()
        );
    }
    ::syslog::info!("mount-test: [PASS] readlink works over multi-part request path");

    let attr = ::syscall::safe::fs::lstat(&link_path)?;
    if attr.file_type() != FileType::SymbolicLink {
        panic!("lstat (multi-part request): expected SymbolicLink, got {:?}", attr.file_type());
    }
    ::syslog::info!("mount-test: [PASS] lstat works over multi-part request path");

    // Following stat over the multi-part request path: `stat` must resolve the link
    // and report the target's type (RegularFile), exercising `handle_long_pathstat`.
    let sattr = ::syscall::safe::fs::stat(&link_path)?;
    if sattr.file_type() != FileType::RegularFile {
        panic!(
            "stat (multi-part request): expected RegularFile (followed target), got {:?}",
            sattr.file_type()
        );
    }
    ::syslog::info!("mount-test: [PASS] stat follows symlink over multi-part request path");

    ::syscall::safe::fs::unlink(&link_path)?;
    Ok(())
}
