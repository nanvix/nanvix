// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Native path storage and filesystem operations for the shared traversal loop.

use super::Sandbox;
use ::fs_core::path::{
    Adapter,
    Component as WalkComponent,
    EntryKind,
};
use std::{
    collections::VecDeque,
    ffi::OsString,
    fs,
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

pub(super) struct WindowsPath {
    current: PathBuf,
    pending: VecDeque<WalkComponent<OsString>>,
}

impl WindowsPath {
    pub(super) fn new(base: &Path, relative: &Path) -> Self {
        Self {
            current: base.to_path_buf(),
            pending: components(relative).collect(),
        }
    }

    pub(super) fn into_path(self) -> PathBuf {
        self.current
    }
}

impl Adapter for WindowsPath {
    type Name = OsString;
    // The current path retains the appended name until expansion, so its parent is recoverable.
    type Parent = ();
    type Error = io::Error;

    fn next_component(&mut self) -> Option<WalkComponent<OsString>> {
        self.pending.pop_front()
    }

    fn requires_directory(&self) -> bool {
        !self.pending.is_empty()
    }

    fn push(&mut self, name: OsString) -> io::Result<()> {
        // Joining an entire suffix onto a verbatim root would collapse symlink-sensitive `..`.
        self.current.push(name);
        Ok(())
    }

    fn parent(&mut self) -> io::Result<()> {
        if !self.current.pop() {
            return Err(ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }

    fn inspect(&mut self) -> io::Result<EntryKind> {
        let metadata = fs::symlink_metadata(&self.current)?;
        if metadata.file_type().is_symlink() {
            return Ok(EntryKind::Symlink);
        }
        self.current = self.current.canonicalize()?;
        Ok(if self.current.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::Other
        })
    }

    fn expand_link(&mut self, (): ()) -> io::Result<()> {
        let target = fs::read_link(&self.current)?;
        let parent = self.current.parent().ok_or(ErrorKind::PermissionDenied)?;
        self.current = Sandbox::windows_target_base(parent, &target)?;
        // Root/dot-only targets have no entry to inspect. Preserve the former post-target check
        // when that volume/share root must serve as the directory for an original suffix.
        let anchor_only = target.components().all(|component| {
            matches!(component, Component::Prefix(_) | Component::RootDir | Component::CurDir)
        });
        if anchor_only && !self.pending.is_empty() && !self.current.is_dir() {
            return Err(ErrorKind::NotADirectory.into());
        }
        for component in components(&target).rev() {
            self.pending.push_front(component);
        }
        Ok(())
    }

    fn is_not_found(error: &io::Error) -> bool {
        error.kind() == ErrorKind::NotFound
    }
}

// Parse each stored target separately from the original suffix, retaining native dot/separator
// policy. Prefixes select the anchor above; names stay native, including non-Unicode UTF-16.
fn components(path: &Path) -> impl DoubleEndedIterator<Item = WalkComponent<OsString>> + '_ {
    path.components().filter_map(|component| match component {
        Component::Normal(name) => Some(WalkComponent::Name(name.to_os_string())),
        Component::CurDir => Some(WalkComponent::Current),
        Component::ParentDir => Some(WalkComponent::Parent),
        Component::Prefix(_) | Component::RootDir => None,
    })
}
