// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Path sandboxing for the host filesystem daemon.
//!
//! Ensures all guest-requested paths resolve within the configured root directory.
//! Rejects path traversal attacks and symlinks that escape the sandbox.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
use self::windows::WindowsPath;
#[cfg(windows)]
use ::fs_core::path::{
    walk,
    FinalComponent,
    WalkError,
};
use ::hostfs_api::HostResolvedPath;
use std::{
    io::{
        self,
        ErrorKind,
    },
    path::{
        Component,
        Path,
        PathBuf,
    },
};

/// Maximum symlink expansions during one Windows path walk.
#[cfg(windows)]
const MAX_SYMLINK_EXPANSIONS: usize = 40;

/// Windows reports cyclic symlink traversal with this error; the hostfs error mapper preserves it.
#[cfg(windows)]
const ERROR_CANT_RESOLVE_FILENAME: i32 = 1921;

/// A sandbox that constrains all filesystem operations to a root directory.
pub struct Sandbox {
    /// The absolute path of the root directory on the host.
    root: PathBuf,
}

impl Sandbox {
    /// Creates a new sandbox rooted at the given directory.
    ///
    /// Returns an error if the root directory does not exist, is not a directory,
    /// or cannot be canonicalized.
    pub fn new(root: PathBuf) -> io::Result<Self> {
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!("sandbox root is not an existing directory: {:?}", root),
            ));
        }
        let root: PathBuf = root.canonicalize()?;
        Ok(Self { root })
    }

    /// Resolves a checked host-relative path to an absolute host path within the sandbox.
    ///
    /// The value checks syntax only, not containment. Components and symlinks are interpreted
    /// here against the host root; a leading `mnt/` is an ordinary directory name.
    ///
    /// Returns a permission error for sandbox rejection and preserves filesystem errors such as
    /// missing ancestors and symlink loops.
    ///
    /// This avoids TOCTOU races by attempting `canonicalize()` directly rather than
    /// branching on `exists()`. If canonicalization fails (e.g., file not yet created),
    /// the parent directory is canonicalized instead and the filename is appended.
    ///
    /// # Symlink TOCTOU Limitation
    ///
    /// When the full path does not exist, only the parent directory is canonicalized.
    /// The final filename component is appended unchecked. If an attacker creates a
    /// symlink at that name between the `resolve()` call and the actual filesystem
    /// operation, the symlink target could escape the sandbox. Fully closing this gap
    /// requires opening the parent with `O_NOFOLLOW`-style flags and using `openat()`
    /// relative to that handle, which is platform-specific (Unix `O_NOFOLLOW` / Windows
    /// `FILE_FLAG_OPEN_REPARSE_POINT`) and out of scope for this PR.
    ///
    /// TODO(#sandbox-toctou): use `openat()` with `O_NOFOLLOW` to eliminate the
    /// symlink TOCTOU window for non-existent paths.
    pub fn resolve(&self, relative_path: &HostResolvedPath) -> io::Result<PathBuf> {
        // Strip leading '/' — wire paths are relative to the mount point.
        let cleaned = relative_path.as_str().trim_start_matches('/');

        // Resolve intermediate symlinks before interpreting parent components on Windows.
        let candidate = self.candidate(cleaned, true)?;

        // Try to canonicalize directly (handles existing files and symlink resolution).
        // Fall back to parent canonicalization for files that don't exist yet (e.g., create).
        let resolved: PathBuf = match candidate.canonicalize() {
            Ok(p) => p,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                // Only a missing final entry is eligible for creation, not a traversal error.
                let parent = candidate.parent().ok_or(ErrorKind::PermissionDenied)?;
                let parent_resolved = parent.canonicalize()?;
                let file_name = candidate.file_name().ok_or(ErrorKind::PermissionDenied)?;
                parent_resolved.join(file_name)
            },
            Err(error) => return Err(error),
        };

        // Verify the resolved path is within the sandbox root.
        if resolved.starts_with(&self.root) {
            Ok(resolved)
        } else {
            Err(ErrorKind::PermissionDenied.into())
        }
    }

    /// Joins a host-relative path without discarding symlink-sensitive parent components.
    /// Containment is checked by the caller after resolution.
    fn candidate(&self, relative: &str, follow_final: bool) -> io::Result<PathBuf> {
        #[cfg(not(windows))]
        {
            let _ = follow_final;
            Ok(self.root.join(relative))
        }
        #[cfg(windows)]
        {
            let path = Path::new(relative);
            if path
                .components()
                .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
            {
                return Err(ErrorKind::PermissionDenied.into());
            }
            let mut adapter = WindowsPath::new(&self.root, path);
            let policy = if follow_final {
                FinalComponent::FollowOrMissing
            } else {
                FinalComponent::LeaveUninspected
            };
            walk(&mut adapter, policy, MAX_SYMLINK_EXPANSIONS).map_err(|error| match error {
                WalkError::Backend(error) => error,
                WalkError::TooManySymlinks => {
                    io::Error::from_raw_os_error(ERROR_CANT_RESOLVE_FILENAME)
                },
                WalkError::NotDirectory => ErrorKind::NotADirectory.into(),
            })?;
            Ok(adapter.into_path())
        }
    }

    /// Anchors a stored Windows target without consulting process-global per-drive cwd state.
    #[cfg(windows)]
    fn windows_target_base(base: &Path, target: &Path) -> io::Result<PathBuf> {
        if target.is_absolute() {
            return Ok(target
                .components()
                .take_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
                .collect());
        }
        if matches!(target.components().next(), Some(Component::Prefix(_))) {
            // `C:foo` is drive-relative, unlike `C:\\foo` or `\\foo`. Do not guess its cwd.
            return Err(ErrorKind::InvalidInput.into());
        }
        if target.has_root() {
            // `\\assets\\file` inherits the link's drive, or its server/share on a UNC volume.
            // Preserve the prefix rather than using the daemon's current drive.
            if !base.is_absolute() {
                return Err(ErrorKind::InvalidInput.into());
            }
            return Ok(base.components().take(2).collect());
        }
        Ok(base.to_path_buf())
    }

    /// Returns the sandbox root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    ///
    /// # Description
    ///
    /// Resolves a checked host-relative path to an absolute host path *without* following the
    /// final path component.
    ///
    /// Behaves like [`Self::resolve`] for every component except the last: the parent
    /// directory is canonicalized (and must exist and lie within the sandbox), and the
    /// unmodified final component is appended. This is the correct resolution mode for
    /// operations that must act on a symbolic link itself rather than on its target —
    /// `lstat`, `readlink`, `unlink` on a link, and `symlink` (creating a link must not
    /// follow any pre-existing link at the destination path).
    ///
    /// Bare names (no parent separator) resolve against the sandbox root.
    ///
    /// # Parameters
    ///
    /// - `relative_path`: A decoded path relative to the sandbox root. Leading `/` characters
    ///   are removed as for [`Self::resolve`]; the guest mount prefix is not stripped again.
    ///
    /// # Symlink TOCTOU
    ///
    /// The same TOCTOU caveat as [`Self::resolve`] applies: after the parent is
    /// canonicalized, an attacker swapping a symlink under the parent could still
    /// influence subsequent operations on the returned path. Closing that gap requires
    /// `openat()`-based dirfd operations.
    ///
    pub fn resolve_nofollow(&self, relative_path: &HostResolvedPath) -> io::Result<PathBuf> {
        let cleaned = relative_path.as_str().trim_start_matches('/');
        if cleaned.is_empty() {
            // Refers to the sandbox root itself; resolve normally.
            return Ok(self.root.clone());
        }
        // Check the raw final component before the Windows walk consumes any parent components.
        let last_component = Path::new(cleaned)
            .components()
            .next_back()
            .ok_or(ErrorKind::PermissionDenied)?;
        if !matches!(last_component, Component::Normal(_)) {
            return Err(ErrorKind::PermissionDenied.into());
        }
        let candidate = self.candidate(cleaned, false)?;
        let file_name = candidate.file_name().ok_or(ErrorKind::PermissionDenied)?;
        let parent = candidate.parent().ok_or(ErrorKind::PermissionDenied)?;
        let parent_resolved = parent.canonicalize()?;
        if !parent_resolved.starts_with(&self.root) {
            return Err(ErrorKind::PermissionDenied.into());
        }
        Ok(parent_resolved.join(file_name))
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::fs;
    use ::tempfile::TempDir;

    /// Creates a sandbox rooted at a fresh temporary directory.
    #[allow(clippy::expect_used)]
    fn make_sandbox() -> (TempDir, Sandbox) {
        let tmp: TempDir = TempDir::new().expect("create tempdir");
        let sandbox: Sandbox = Sandbox::new(tmp.path().to_path_buf()).expect("create sandbox");
        (tmp, sandbox)
    }

    /// Builds the same checked host-relative value produced by request decoding.
    #[allow(clippy::expect_used)]
    fn wire_path(path: &str) -> HostResolvedPath {
        HostResolvedPath::from_wire(path.to_owned()).expect("valid test path")
    }

    /// Creates a file symlink in a cross-platform way.
    ///
    /// On Unix uses `std::os::unix::fs::symlink`. On Windows uses
    /// `std::os::windows::fs::symlink_file`, which requires either Administrator
    /// privileges or Developer Mode (`SeCreateSymbolicLinkPrivilege`).
    fn symlink_file(target: &Path, link: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            ::std::os::unix::fs::symlink(target, link)
        }
        #[cfg(windows)]
        {
            ::std::os::windows::fs::symlink_file(target, link)
        }
    }

    /// Creates a directory symlink in a cross-platform way.
    fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            ::std::os::unix::fs::symlink(target, link)
        }
        #[cfg(windows)]
        {
            ::std::os::windows::fs::symlink_dir(target, link)
        }
    }

    /// Probes whether the host allows unprivileged symlink creation.
    ///
    /// On Windows this returns `false` unless the process holds
    /// `SeCreateSymbolicLinkPrivilege` (granted by Developer Mode or running as
    /// Administrator). Symlink-dependent tests early-return when this returns
    /// `false` so they are silently skipped rather than failing on unsupported
    /// hosts.
    fn symlinks_supported() -> bool {
        let Ok(tmp) = TempDir::new() else {
            return false;
        };
        let target: PathBuf = tmp.path().join("t");
        if fs::write(&target, b"x").is_err() {
            return false;
        }
        symlink_file(&target, &tmp.path().join("l")).is_ok()
    }

    // ---------------------------------------------------------------------------------------------
    // Sandbox::new
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn new_rejects_nonexistent_root() {
        let tmp: TempDir = TempDir::new().unwrap();
        let missing: PathBuf = tmp.path().join("does-not-exist");
        let err: io::Error = match Sandbox::new(missing) {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        };
        assert_eq!(err.kind(), io::ErrorKind::NotADirectory);
    }

    #[test]
    fn new_rejects_file_as_root() {
        let tmp: TempDir = TempDir::new().unwrap();
        let file: PathBuf = tmp.path().join("a-file");
        fs::write(&file, b"x").unwrap();
        let err: io::Error = match Sandbox::new(file) {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        };
        assert_eq!(err.kind(), io::ErrorKind::NotADirectory);
    }

    #[test]
    fn new_canonicalizes_root() {
        let (tmp, sandbox) = make_sandbox();
        assert_eq!(sandbox.root(), tmp.path().canonicalize().unwrap().as_path());
    }

    // ---------------------------------------------------------------------------------------------
    // Sandbox::resolve
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn resolve_existing_file() {
        let (_tmp, sandbox) = make_sandbox();
        let target: PathBuf = sandbox.root().join("file.txt");
        fs::write(&target, b"hello").unwrap();

        let resolved: PathBuf = sandbox.resolve(&wire_path("file.txt")).expect("resolve");
        assert_eq!(resolved, target.canonicalize().unwrap());
    }

    #[test]
    fn resolve_strips_leading_slash() {
        let (_tmp, sandbox) = make_sandbox();
        fs::write(sandbox.root().join("file.txt"), b"hello").unwrap();

        let a: PathBuf = sandbox.resolve(&wire_path("/file.txt")).expect("resolve");
        let b: PathBuf = sandbox.resolve(&wire_path("file.txt")).expect("resolve");
        assert_eq!(a, b);
    }

    #[test]
    fn resolve_nonexistent_file_via_parent_canonicalization() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox
            .resolve(&wire_path("not-yet-created.txt"))
            .expect("resolve");
        assert_eq!(resolved, sandbox.root().join("not-yet-created.txt"));
    }

    #[test]
    fn resolve_nonexistent_parent_returns_error() {
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve(&wire_path("missing-dir/file.txt")).is_err());
    }

    #[test]
    fn resolve_rejects_dotdot_escape() {
        let (_tmp, sandbox) = make_sandbox();
        // Create a sibling outside the sandbox and try to traverse to it.
        let escape: &str = "../escape.txt";
        assert!(sandbox.resolve(&wire_path(escape)).is_err());
    }

    #[test]
    fn checked_paths_keep_physical_symlink_parent_resolution() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir_all(sandbox.root().join("real/child")).expect("create physical directories");
        fs::write(sandbox.root().join("file"), b"lexical").expect("create decoy");
        fs::write(sandbox.root().join("real/file"), b"physical").expect("create target");
        symlink_dir(Path::new("./real//child/../child"), &sandbox.root().join("alias"))
            .expect("create ancestor symlink");
        symlink_file(&sandbox.root().join("real/file"), &sandbox.root().join("real/link"))
            .expect("create final symlink");

        let path = wire_path("alias//.././file");
        assert_eq!(path.as_str(), "alias//.././file", "construction must preserve spelling");
        assert_eq!(
            sandbox.resolve(&path).expect("follow physical parent"),
            sandbox
                .root()
                .join("real/file")
                .canonicalize()
                .expect("canonical target"),
        );
        let link = wire_path("alias//../link");
        assert_eq!(
            sandbox.resolve_nofollow(&link).expect("keep final symlink"),
            sandbox
                .root()
                .join("real")
                .canonicalize()
                .expect("canonical parent")
                .join("link"),
        );
        assert_eq!(
            sandbox.resolve(&link).expect("follow final symlink"),
            sandbox.resolve(&path).expect("physical target"),
        );
    }

    #[test]
    fn physical_parent_walk_preserves_creation_and_containment() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir_all(sandbox.root().join("real/child")).expect("create directories");
        symlink_dir(&sandbox.root().join("real/child"), &sandbox.root().join("alias"))
            .expect("create ancestor symlink");
        let expected = sandbox
            .root()
            .join("real")
            .canonicalize()
            .expect("physical parent")
            .join("new");
        assert_eq!(sandbox.resolve(&wire_path("alias/../new")).ok(), Some(expected.clone()));
        assert_eq!(sandbox.resolve_nofollow(&wire_path("alias/../new")).ok(), Some(expected));
        fs::write(sandbox.root().join("file"), b"not a directory").expect("create file");
        assert!(sandbox.resolve(&wire_path("file/../new")).is_err());
        assert!(sandbox.resolve_nofollow(&wire_path("file/../new")).is_err());
        assert!(sandbox.resolve(&wire_path("missing/../new")).is_err());
        assert!(sandbox
            .resolve_nofollow(&wire_path("missing/../new"))
            .is_err());
        let outside = TempDir::new().expect("outside sandbox");
        fs::create_dir(outside.path().join("child")).expect("outside child");
        fs::write(outside.path().join("secret"), b"secret").expect("outside sentinel");
        symlink_dir(&outside.path().join("child"), &sandbox.root().join("escape"))
            .expect("outside ancestor symlink");
        assert!(sandbox.resolve(&wire_path("escape/../secret")).is_err());
        assert!(sandbox
            .resolve_nofollow(&wire_path("escape/../secret"))
            .is_err());
    }

    #[test]
    fn symlink_target_spelling_and_cycles() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir(sandbox.root().join("dir")).expect("create directory");
        fs::write(sandbox.root().join("dir/file"), b"target").expect("create file");
        let spelling = Path::new("./dir//../dir/file");
        symlink_file(spelling, &sandbox.root().join("link")).expect("create spelled symlink");
        assert_eq!(fs::read_link(sandbox.root().join("link")).expect("read target"), spelling);
        assert_eq!(
            sandbox
                .resolve(&wire_path("link"))
                .expect("follow spelled target"),
            sandbox
                .root()
                .join("dir/file")
                .canonicalize()
                .expect("canonical target"),
        );
        assert_eq!(
            sandbox.resolve_nofollow(&wire_path("link")).ok(),
            Some(sandbox.root().join("link"))
        );
        symlink_file(Path::new("cycle"), &sandbox.root().join("cycle")).expect("create cycle");
        // Following a cycle must not loop indefinitely; no-follow still addresses the link.
        assert!(sandbox.resolve(&wire_path("cycle/child")).is_err());
        assert_eq!(
            sandbox.resolve_nofollow(&wire_path("cycle")).ok(),
            Some(sandbox.root().join("cycle"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_symlink_budget_boundary() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::write(sandbox.root().join("file"), b"target").expect("create target");
        let mut target = PathBuf::from("file");
        for index in (1..=MAX_SYMLINK_EXPANSIONS + 1).rev() {
            let name = format!("link-{index}");
            symlink_file(&target, &sandbox.root().join(&name)).expect("create link chain");
            target = PathBuf::from(name);
        }
        assert_eq!(
            sandbox
                .resolve(&wire_path("link-2"))
                .expect("exactly 40 expansions"),
            sandbox
                .root()
                .join("file")
                .canonicalize()
                .expect("canonical target")
        );
        assert_eq!(
            sandbox
                .resolve(&wire_path("link-1"))
                .expect_err("41 expansions must fail")
                .raw_os_error(),
            Some(ERROR_CANT_RESOLVE_FILENAME)
        );
        assert_eq!(
            sandbox
                .resolve_nofollow(&wire_path("link-1"))
                .expect("nofollow uses no expansion budget"),
            sandbox.root().join("link-1")
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_final_entry_policies() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir(sandbox.root().join("real")).expect("create directory");
        fs::write(sandbox.root().join("real/file"), b"target").expect("create file");
        symlink_dir(Path::new("real"), &sandbox.root().join("alias")).expect("directory link");
        for (name, target) in [
            ("leaf", "real/file"),
            ("dangling", "absent"),
            ("chain", "dangling"),
            ("missing-parent", "absent/../real/file"),
            ("cycle", "cycle"),
        ] {
            symlink_file(Path::new(target), &sandbox.root().join(name)).expect("file link");
        }
        for name in ["real/file", "leaf"] {
            assert_eq!(
                sandbox.resolve(&wire_path(name)).expect("existing target"),
                sandbox
                    .root()
                    .join("real/file")
                    .canonicalize()
                    .expect("canonical file")
            );
        }
        for name in ["new", "real/new", "alias/new"] {
            let expected = if name == "new" { "new" } else { "real/new" };
            assert_eq!(
                sandbox
                    .resolve(&wire_path(name))
                    .expect("ordinary missing final entry"),
                sandbox.root().join(expected)
            );
            assert_eq!(
                sandbox
                    .resolve_nofollow(&wire_path(name))
                    .expect("uninspected final entry"),
                sandbox.root().join(expected)
            );
        }
        for name in ["leaf", "dangling", "chain", "missing-parent", "cycle"] {
            assert_eq!(
                sandbox
                    .resolve_nofollow(&wire_path(name))
                    .expect("keep final link"),
                sandbox.root().join(name)
            );
        }
        for name in ["dangling", "chain", "missing-parent"] {
            assert_eq!(
                sandbox
                    .resolve(&wire_path(name))
                    .expect_err("target must exist")
                    .kind(),
                ErrorKind::NotFound
            );
            for follow in [false, true] {
                assert_eq!(
                    sandbox
                        .candidate(&format!("{name}/new"), follow)
                        .expect_err("ancestor must exist")
                        .kind(),
                    ErrorKind::NotFound
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_budget_spans_separate_link_components() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::write(sandbox.root().join("file"), b"target").expect("create target");
        symlink_dir(Path::new("."), &sandbox.root().join("hop")).expect("link to current dir");
        symlink_file(Path::new("file"), &sandbox.root().join("leaf")).expect("final file link");
        let prefix = "hop/".repeat(MAX_SYMLINK_EXPANSIONS);
        assert_eq!(
            sandbox
                .resolve(&wire_path(&format!("{prefix}file")))
                .expect("40 separate expansions"),
            sandbox
                .root()
                .join("file")
                .canonicalize()
                .expect("canonical target")
        );
        assert_eq!(
            sandbox
                .resolve_nofollow(&wire_path(&format!("{prefix}leaf")))
                .expect("keep final link"),
            sandbox.root().join("leaf")
        );
        assert_eq!(
            sandbox
                .resolve(&wire_path(&format!("{prefix}leaf")))
                .expect_err("41st expansion")
                .raw_os_error(),
            Some(ERROR_CANT_RESOLVE_FILENAME)
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_target_parsing_keeps_native_trailing_syntax_policy() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::write(sandbox.root().join("file"), b"target").expect("create target");
        for (name, target) in [("dot", "file/."), ("slash", "file/")] {
            symlink_file(Path::new(target), &sandbox.root().join(name)).expect("stored target");
            // Native components discard these endings; do not import libc's trailing-slash policy.
            assert_eq!(
                sandbox
                    .resolve(&wire_path(name))
                    .expect("native target syntax"),
                sandbox
                    .root()
                    .join("file")
                    .canonicalize()
                    .expect("canonical target")
            );
            for follow in [false, true] {
                assert_eq!(
                    sandbox
                        .candidate(&format!("{name}/child"), follow)
                        .expect_err("a real suffix still requires a directory")
                        .kind(),
                    ErrorKind::NotADirectory
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_symlink_target_preserves_non_unicode_name() {
        use std::{
            ffi::OsString,
            os::windows::ffi::OsStringExt,
        };

        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        // NTFS names can contain an unpaired UTF-16 surrogate; no UTF-8 conversion is valid here.
        let name = OsString::from_wide(&[0x66, 0xd800, 0x78]);
        let target = sandbox.root().join(&name);
        fs::write(&target, b"native").expect("create native name");
        symlink_file(Path::new(&name), &sandbox.root().join("link")).expect("native target");
        assert_eq!(
            sandbox
                .resolve(&wire_path("link"))
                .expect("follow native target"),
            target.canonicalize().expect("canonical target")
        );
        assert_eq!(fs::read(&target).expect("target contents"), b"native");
    }

    #[cfg(windows)]
    #[test]
    fn windows_unavailable_target_root_requires_directory_before_suffix() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        let drive = (b'A'..=b'Z')
            .rev()
            .find(|drive| !Path::new(&format!("{}:\\", char::from(*drive))).exists())
            .expect("an unavailable drive for the link target");
        for (name, suffix) in [("root", ""), ("dot", "."), ("dots", r".\.")] {
            let target = format!(r"\\?\{}:\{suffix}", char::from(drive));
            symlink_dir(Path::new(&target), &sandbox.root().join(name))
                .expect("unavailable target");
            for follow in [false, true] {
                assert_eq!(
                    sandbox
                        .candidate(&format!("{name}/child"), follow)
                        .expect_err("target root must be a directory before the suffix")
                        .kind(),
                    ErrorKind::NotADirectory,
                    "target={target:?}, follow={follow}"
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_target_anchor_matrix() {
        // UNC coverage is syntactic and requires no SMB share, credentials, or network access.
        for (base, root) in [
            (r"C:\work\links", r"C:\"),
            (r"D:\work\links", r"D:\"),
            (r"\\?\D:\work\links", r"\\?\D:\"),
            (r"\\server\share\work\links", r"\\server\share\"),
            (r"\\?\UNC\server\share\work\links", r"\\?\UNC\server\share\"),
        ] {
            for target in [r"assets\file", "./assets//file"] {
                assert_eq!(
                    Sandbox::windows_target_base(Path::new(base), Path::new(target))
                        .expect("relative target"),
                    Path::new(base)
                );
            }
            for target in [r"\assets\file", "/assets/file"] {
                assert_eq!(
                    Sandbox::windows_target_base(Path::new(base), Path::new(target))
                        .expect("root-relative target"),
                    Path::new(root)
                );
            }
            for (target, expected_root) in [
                (r"E:\assets\file", r"E:\"),
                (r"\\?\E:\assets\file", r"\\?\E:\"),
                (r"\\other\data\file", r"\\other\data\"),
                (r"\\?\UNC\other\data\file", r"\\?\UNC\other\data\"),
            ] {
                assert_eq!(
                    Sandbox::windows_target_base(Path::new(base), Path::new(target))
                        .expect("absolute target"),
                    Path::new(expected_root)
                );
            }
            for target in [r"C:assets\file", r"D:assets\file"] {
                assert_eq!(
                    Sandbox::windows_target_base(Path::new(base), Path::new(target))
                        .expect_err("drive-relative target needs process-global cwd")
                        .kind(),
                    ErrorKind::InvalidInput
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_root_relative_symlink_target() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::write(sandbox.root().join("file"), b"target").expect("create target file");
        fs::create_dir(sandbox.root().join("dir")).expect("create target directory");
        for (name, is_directory) in [("file", false), ("dir", true)] {
            let target = sandbox.root().join(name);
            let rooted = target.components().skip(1).collect::<PathBuf>();
            let link_name = format!("{name}-link");
            let link = sandbox.root().join(&link_name);
            let create = if is_directory {
                symlink_dir
            } else {
                symlink_file
            };
            create(&rooted, &link).expect("create root-relative link");
            assert_eq!(
                sandbox
                    .resolve(&wire_path(&link_name))
                    .expect("inherit link volume"),
                target.canonicalize().expect("canonical target")
            );
            assert_eq!(
                sandbox
                    .resolve_nofollow(&wire_path(&link_name))
                    .expect("keep final link"),
                link
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn resolve_rejects_windows_path_prefixes() {
        let (_tmp, sandbox) = make_sandbox();
        for path in [
            r"C:\file",
            r"C:file",
            r"\file",
            r"\\server\share\file",
            r"\\?\C:\file",
        ] {
            assert!(
                sandbox.resolve(&wire_path(path)).is_err(),
                "hostfs paths must be relative: {path}"
            );
            assert!(
                sandbox.resolve_nofollow(&wire_path(path)).is_err(),
                "hostfs paths must be relative: {path}"
            );
        }
    }

    #[test]
    fn resolve_normalizes_interior_dotdot() {
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir(sandbox.root().join("sub")).unwrap();
        fs::write(sandbox.root().join("file.txt"), b"x").unwrap();

        let resolved: PathBuf = sandbox
            .resolve(&wire_path("sub/../file.txt"))
            .expect("resolve");
        assert_eq!(resolved, sandbox.root().join("file.txt").canonicalize().unwrap());
    }

    #[test]
    fn resolve_nested_existing_path() {
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir_all(sandbox.root().join("a/b")).unwrap();
        fs::write(sandbox.root().join("a/b/c.txt"), b"x").unwrap();

        let resolved: PathBuf = sandbox.resolve(&wire_path("a/b/c.txt")).expect("resolve");
        assert_eq!(resolved, sandbox.root().join("a/b/c.txt").canonicalize().unwrap());
    }

    #[test]
    fn resolve_root_itself() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve(&wire_path("")).expect("resolve");
        assert_eq!(resolved, sandbox.root());
    }

    #[test]
    fn resolve_follows_symlink_within_sandbox() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        let target: PathBuf = sandbox.root().join("target.txt");
        fs::write(&target, b"x").unwrap();
        let link: PathBuf = sandbox.root().join("link.txt");
        symlink_file(&target, &link).unwrap();

        let resolved: PathBuf = sandbox.resolve(&wire_path("link.txt")).expect("resolve");
        assert_eq!(resolved, target.canonicalize().unwrap());
    }

    #[test]
    fn resolve_rejects_symlink_escaping_sandbox() {
        if !symlinks_supported() {
            return;
        }
        let outside: TempDir = TempDir::new().unwrap();
        let outside_file: PathBuf = outside.path().join("secret.txt");
        fs::write(&outside_file, b"secret").unwrap();

        let (_tmp, sandbox) = make_sandbox();
        let link: PathBuf = sandbox.root().join("escape");
        symlink_file(&outside_file, &link).unwrap();

        assert!(sandbox.resolve(&wire_path("escape")).is_err());
    }

    // ---------------------------------------------------------------------------------------------
    // Sandbox::resolve_nofollow
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn resolve_nofollow_bare_name_against_root() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox
            .resolve_nofollow(&wire_path("file.txt"))
            .expect("resolve");
        assert_eq!(resolved, sandbox.root().join("file.txt"));
    }

    #[test]
    fn resolve_nofollow_strips_leading_slash() {
        let (_tmp, sandbox) = make_sandbox();
        let a: PathBuf = sandbox
            .resolve_nofollow(&wire_path("/file.txt"))
            .expect("resolve");
        let b: PathBuf = sandbox
            .resolve_nofollow(&wire_path("file.txt"))
            .expect("resolve");
        assert_eq!(a, b);
    }

    #[test]
    fn resolve_nofollow_empty_returns_root() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve_nofollow(&wire_path("")).expect("resolve");
        assert_eq!(resolved, sandbox.root());
    }

    #[test]
    fn resolve_nofollow_root_slash_returns_root() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve_nofollow(&wire_path("/")).expect("resolve");
        assert_eq!(resolved, sandbox.root());
    }

    #[test]
    fn resolve_nofollow_missing_parent_returns_error() {
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox
            .resolve_nofollow(&wire_path("missing/file.txt"))
            .is_err());
    }

    #[test]
    fn resolve_nofollow_rejects_dotdot_escape() {
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox
            .resolve_nofollow(&wire_path("../escape.txt"))
            .is_err());
    }

    #[test]
    fn resolve_nofollow_rejects_bare_dotdot() {
        // `resolve_nofollow("..")` must not produce `<root>/..`, which would
        // escape the sandbox once handed to a filesystem syscall.
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve_nofollow(&wire_path("..")).is_err());
        assert!(sandbox.resolve_nofollow(&wire_path("/..")).is_err());
    }

    #[test]
    fn resolve_nofollow_rejects_bare_dot() {
        // `.` as the final component is also rejected: it is not a meaningful
        // target for operations that act on a named entry (lstat, readlink,
        // unlink, symlink). Callers wanting the root should pass "" or "/".
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve_nofollow(&wire_path(".")).is_err());
        assert!(sandbox.resolve_nofollow(&wire_path("/.")).is_err());
    }

    #[test]
    fn resolve_nofollow_rejects_trailing_dotdot_after_subdir() {
        // Even with an existing parent, a trailing `..` must be rejected so
        // the returned path cannot reference the parent directory itself.
        // Note: a trailing `.` is normalized away by `Path::components()`
        // (`sub/.` ≡ `sub`), so it is not a separate escape vector here.
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir(sandbox.root().join("sub")).unwrap();
        assert!(sandbox.resolve_nofollow(&wire_path("sub/..")).is_err());
    }

    #[test]
    fn resolve_nofollow_nested_existing_parent() {
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir_all(sandbox.root().join("a/b")).unwrap();

        let resolved: PathBuf = sandbox
            .resolve_nofollow(&wire_path("a/b/c.txt"))
            .expect("resolve");
        assert_eq!(
            resolved,
            sandbox
                .root()
                .join("a/b")
                .canonicalize()
                .unwrap()
                .join("c.txt")
        );
    }

    #[test]
    fn resolve_nofollow_does_not_follow_final_symlink() {
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        let target: PathBuf = sandbox.root().join("target.txt");
        fs::write(&target, b"x").unwrap();
        let link: PathBuf = sandbox.root().join("link.txt");
        symlink_file(&target, &link).unwrap();

        let resolved: PathBuf = sandbox
            .resolve_nofollow(&wire_path("link.txt"))
            .expect("resolve");
        // The returned path must still point at the link itself, not the target.
        assert_eq!(resolved, sandbox.root().join("link.txt"));
        assert!(resolved
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn resolve_nofollow_follows_parent_symlink() {
        // A symlink *in the parent chain* (not the final component) is still followed,
        // matching the documented semantics.
        if !symlinks_supported() {
            return;
        }
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir(sandbox.root().join("real")).unwrap();
        symlink_dir(&sandbox.root().join("real"), &sandbox.root().join("alias")).unwrap();

        let resolved: PathBuf = sandbox
            .resolve_nofollow(&wire_path("alias/file.txt"))
            .expect("resolve");
        assert_eq!(
            resolved,
            sandbox
                .root()
                .join("real")
                .canonicalize()
                .unwrap()
                .join("file.txt")
        );
    }

    #[test]
    fn resolve_nofollow_rejects_parent_symlink_escape() {
        if !symlinks_supported() {
            return;
        }
        let outside: TempDir = TempDir::new().unwrap();

        let (_tmp, sandbox) = make_sandbox();
        symlink_dir(outside.path(), &sandbox.root().join("escape")).unwrap();

        assert!(sandbox
            .resolve_nofollow(&wire_path("escape/file.txt"))
            .is_err());
    }
}
