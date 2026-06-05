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
    // the `/mnt/` mount prefix) fits within `MAX_INLINE_PATH_LEN` (36 bytes), so
    // readlink/lstat take the single-message fast path.
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

/// Tests that readlink/lstat/stat work over the multi-part wire format.
///
/// Uses a link path whose wire form (after the `/mnt/` prefix is stripped) exceeds
/// `hostfs_api::MAX_INLINE_PATH_LEN` (36 bytes), so vfsd sends the request through
/// the multi-part assembler rather than the inline single-message fast path. This
/// covers both no-follow (`lstat`) and follow (`stat`) multi-part dispatch paths.
fn test_long_path_multipart() -> Result<(), Error> {
    // Stripped wire path: "long-symlink-name-padding-AAAAAAAAAAAA.lnk" (42 bytes > 36).
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
