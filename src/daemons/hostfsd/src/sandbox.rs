// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Path sandboxing for the host filesystem daemon.
//!
//! Ensures all guest-requested paths resolve within the configured root directory.
//! Rejects path traversal attacks and symlinks that escape the sandbox.

use std::{
    io,
    path::{
        Path,
        PathBuf,
    },
};

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

    /// Resolves a guest-relative path to an absolute host path within the sandbox.
    ///
    /// Returns `None` if the resolved path escapes the sandbox root (path traversal).
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
    pub fn resolve(&self, relative_path: &str) -> Option<PathBuf> {
        // Strip leading '/' — guest paths are relative to the mount point.
        let cleaned: &str = relative_path.trim_start_matches('/');

        // Join with root and canonicalize to resolve `.` and `..`.
        let candidate: PathBuf = self.root.join(cleaned);

        // Try to canonicalize directly (handles existing files and symlink resolution).
        // Fall back to parent canonicalization for files that don't exist yet (e.g., create).
        let resolved: PathBuf = match candidate.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Canonicalize the parent directory (must exist).
                let parent: &Path = candidate.parent()?;
                let parent_resolved: PathBuf = parent.canonicalize().ok()?;
                let file_name: &std::ffi::OsStr = candidate.file_name()?;
                parent_resolved.join(file_name)
            },
        };

        // Verify the resolved path is within the sandbox root.
        if resolved.starts_with(&self.root) {
            Some(resolved)
        } else {
            None
        }
    }

    /// Returns the sandbox root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    ///
    /// # Description
    ///
    /// Resolves a guest-relative path to an absolute host path *without* following the
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
    /// - `relative_path`: The guest path to resolve, relative to the sandbox root. A
    ///   leading `/` is also accepted and treated as guest-absolute; it is normalized
    ///   to a sandbox-relative path before resolution.
    ///
    /// # Symlink TOCTOU
    ///
    /// The same TOCTOU caveat as [`Self::resolve`] applies: after the parent is
    /// canonicalized, an attacker swapping a symlink under the parent could still
    /// influence subsequent operations on the returned path. Closing that gap requires
    /// `openat()`-based dirfd operations.
    ///
    pub fn resolve_nofollow(&self, relative_path: &str) -> Option<PathBuf> {
        let cleaned: &str = relative_path.trim_start_matches('/');
        if cleaned.is_empty() {
            // Refers to the sandbox root itself; resolve normally.
            return Some(self.root.clone());
        }
        let candidate: PathBuf = self.root.join(cleaned);

        // Reject `.` or `..` as the final component: these would let the resolved path step outside
        // the sandbox after the parent-only containment check (e.g. `resolve_nofollow("..")` would
        // otherwise yield `<root>/..`).
        let last_component: ::std::path::Component<'_> = candidate.components().next_back()?;
        if !matches!(last_component, ::std::path::Component::Normal(_)) {
            return None;
        }
        let file_name: &std::ffi::OsStr = candidate.file_name()?;
        let parent: &Path = candidate.parent()?;
        let parent_resolved: PathBuf = parent.canonicalize().ok()?;
        if !parent_resolved.starts_with(&self.root) {
            return None;
        }
        Some(parent_resolved.join(file_name))
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

        let resolved: PathBuf = sandbox.resolve("file.txt").expect("resolve");
        assert_eq!(resolved, target.canonicalize().unwrap());
    }

    #[test]
    fn resolve_strips_leading_slash() {
        let (_tmp, sandbox) = make_sandbox();
        fs::write(sandbox.root().join("file.txt"), b"hello").unwrap();

        let a: PathBuf = sandbox.resolve("/file.txt").expect("resolve");
        let b: PathBuf = sandbox.resolve("file.txt").expect("resolve");
        assert_eq!(a, b);
    }

    #[test]
    fn resolve_nonexistent_file_via_parent_canonicalization() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve("not-yet-created.txt").expect("resolve");
        assert_eq!(resolved, sandbox.root().join("not-yet-created.txt"));
    }

    #[test]
    fn resolve_nonexistent_parent_returns_none() {
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve("missing-dir/file.txt").is_none());
    }

    #[test]
    fn resolve_rejects_dotdot_escape() {
        let (_tmp, sandbox) = make_sandbox();
        // Create a sibling outside the sandbox and try to traverse to it.
        let escape: &str = "../escape.txt";
        assert!(sandbox.resolve(escape).is_none());
    }

    #[test]
    fn resolve_normalizes_interior_dotdot() {
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir(sandbox.root().join("sub")).unwrap();
        fs::write(sandbox.root().join("file.txt"), b"x").unwrap();

        let resolved: PathBuf = sandbox.resolve("sub/../file.txt").expect("resolve");
        assert_eq!(resolved, sandbox.root().join("file.txt").canonicalize().unwrap());
    }

    #[test]
    fn resolve_nested_existing_path() {
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir_all(sandbox.root().join("a/b")).unwrap();
        fs::write(sandbox.root().join("a/b/c.txt"), b"x").unwrap();

        let resolved: PathBuf = sandbox.resolve("a/b/c.txt").expect("resolve");
        assert_eq!(resolved, sandbox.root().join("a/b/c.txt").canonicalize().unwrap());
    }

    #[test]
    fn resolve_root_itself() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve("").expect("resolve");
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

        let resolved: PathBuf = sandbox.resolve("link.txt").expect("resolve");
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

        assert!(sandbox.resolve("escape").is_none());
    }

    // ---------------------------------------------------------------------------------------------
    // Sandbox::resolve_nofollow
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn resolve_nofollow_bare_name_against_root() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve_nofollow("file.txt").expect("resolve");
        assert_eq!(resolved, sandbox.root().join("file.txt"));
    }

    #[test]
    fn resolve_nofollow_strips_leading_slash() {
        let (_tmp, sandbox) = make_sandbox();
        let a: PathBuf = sandbox.resolve_nofollow("/file.txt").expect("resolve");
        let b: PathBuf = sandbox.resolve_nofollow("file.txt").expect("resolve");
        assert_eq!(a, b);
    }

    #[test]
    fn resolve_nofollow_empty_returns_root() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve_nofollow("").expect("resolve");
        assert_eq!(resolved, sandbox.root());
    }

    #[test]
    fn resolve_nofollow_root_slash_returns_root() {
        let (_tmp, sandbox) = make_sandbox();
        let resolved: PathBuf = sandbox.resolve_nofollow("/").expect("resolve");
        assert_eq!(resolved, sandbox.root());
    }

    #[test]
    fn resolve_nofollow_missing_parent_returns_none() {
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve_nofollow("missing/file.txt").is_none());
    }

    #[test]
    fn resolve_nofollow_rejects_dotdot_escape() {
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve_nofollow("../escape.txt").is_none());
    }

    #[test]
    fn resolve_nofollow_rejects_bare_dotdot() {
        // `resolve_nofollow("..")` must not produce `<root>/..`, which would
        // escape the sandbox once handed to a filesystem syscall.
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve_nofollow("..").is_none());
        assert!(sandbox.resolve_nofollow("/..").is_none());
    }

    #[test]
    fn resolve_nofollow_rejects_bare_dot() {
        // `.` as the final component is also rejected: it is not a meaningful
        // target for operations that act on a named entry (lstat, readlink,
        // unlink, symlink). Callers wanting the root should pass "" or "/".
        let (_tmp, sandbox) = make_sandbox();
        assert!(sandbox.resolve_nofollow(".").is_none());
        assert!(sandbox.resolve_nofollow("/.").is_none());
    }

    #[test]
    fn resolve_nofollow_rejects_trailing_dotdot_after_subdir() {
        // Even with an existing parent, a trailing `..` must be rejected so
        // the returned path cannot reference the parent directory itself.
        // Note: a trailing `.` is normalized away by `Path::components()`
        // (`sub/.` ≡ `sub`), so it is not a separate escape vector here.
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir(sandbox.root().join("sub")).unwrap();
        assert!(sandbox.resolve_nofollow("sub/..").is_none());
    }

    #[test]
    fn resolve_nofollow_nested_existing_parent() {
        let (_tmp, sandbox) = make_sandbox();
        fs::create_dir_all(sandbox.root().join("a/b")).unwrap();

        let resolved: PathBuf = sandbox.resolve_nofollow("a/b/c.txt").expect("resolve");
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

        let resolved: PathBuf = sandbox.resolve_nofollow("link.txt").expect("resolve");
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

        let resolved: PathBuf = sandbox.resolve_nofollow("alias/file.txt").expect("resolve");
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

        assert!(sandbox.resolve_nofollow("escape/file.txt").is_none());
    }
}
