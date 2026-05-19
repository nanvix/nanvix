// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! File descriptor table for the host filesystem daemon.
//!
//! Maps remote file descriptors (returned to the guest) to host-side file handles.

use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{
        self,
        File,
    },
    io,
    path::PathBuf,
};

/// Maximum number of open file descriptors per guest.
const MAX_OPEN_FDS: usize = 256;

/// Manages the mapping between remote FDs (guest-visible) and host file handles.
pub struct FdTable {
    /// Maps remote FD → host File handle.
    entries: HashMap<i32, FdEntry>,
    /// Next remote FD to allocate.
    next_fd: i32,
}

/// An entry in the FD table.
pub struct FdEntry {
    /// The host-side file handle.
    pub file: File,
    /// Whether this entry is a directory (for readdir operations).
    pub is_dir: bool,
    /// The path of the file (for stat and readdir operations).
    pub path: String,
    /// Cached directory entries (lazily populated on first readdir call).
    dir_cache: Option<Vec<DirCacheEntry>>,
}

/// A cached directory entry for O(1) indexed readdir access.
pub struct DirCacheEntry {
    /// Entry filename.
    pub name: OsString,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes.
    pub size: u64,
}

impl FdEntry {
    /// Returns the directory entry at the given offset, populating the cache on first access.
    ///
    /// Returns `None` if the offset is past the end of the directory.
    ///
    /// NOTE: The cache is built once on first call and never invalidated. If files are
    /// created or deleted after the first readdir, subsequent calls will return stale results.
    pub fn readdir_at(&mut self, offset: usize) -> Option<&DirCacheEntry> {
        if self.dir_cache.is_none() {
            self.dir_cache = Some(Self::build_dir_cache(&self.path));
        }
        self.dir_cache
            .as_ref()
            .and_then(|entries| entries.get(offset))
    }

    fn build_dir_cache(path: &str) -> Vec<DirCacheEntry> {
        let dir_path: PathBuf = PathBuf::from(path);
        match fs::read_dir(&dir_path) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| {
                    let meta = e.metadata().or_else(|_| fs::metadata(e.path())).ok();
                    let (is_dir, size) = match meta {
                        Some(m) => (m.is_dir(), m.len()),
                        None => (false, 0),
                    };
                    DirCacheEntry {
                        name: e.file_name(),
                        is_dir,
                        size,
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

impl FdTable {
    /// Creates a new empty FD table.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            next_fd: 1, // Start at 1 (0 is reserved).
        }
    }

    /// Allocates a new remote FD for the given file.
    ///
    /// Returns the remote FD, or an error if the table is full.
    pub fn alloc(&mut self, file: File, is_dir: bool, path: String) -> io::Result<i32> {
        if self.entries.len() >= MAX_OPEN_FDS {
            return Err(io::Error::other("too many open file descriptors"));
        }
        let mut fd: i32 = self.next_fd;
        // Skip FDs that are already in use (handles wrap-around collisions).
        let mut attempts: usize = 0;
        while self.entries.contains_key(&fd) {
            fd = fd.wrapping_add(1);
            if fd <= 0 {
                fd = 1;
            }
            attempts += 1;
            if attempts >= MAX_OPEN_FDS {
                return Err(io::Error::other("too many open file descriptors"));
            }
        }
        self.next_fd = fd.wrapping_add(1);
        if self.next_fd <= 0 {
            self.next_fd = 1;
        }
        self.entries.insert(
            fd,
            FdEntry {
                file,
                is_dir,
                path,
                dir_cache: None,
            },
        );
        Ok(fd)
    }

    /// Retrieves a mutable reference to the file entry for the given remote FD.
    pub fn get_mut(&mut self, fd: i32) -> Option<&mut FdEntry> {
        self.entries.get_mut(&fd)
    }

    /// Retrieves a reference to the file entry for the given remote FD.
    pub fn get(&self, fd: i32) -> Option<&FdEntry> {
        self.entries.get(&fd)
    }

    /// Closes and removes the entry for the given remote FD.
    ///
    /// Returns `true` if the FD was valid and was removed.
    pub fn close(&mut self, fd: i32) -> bool {
        self.entries.remove(&fd).is_some()
    }

    /// Invalidates all cached directory listings.
    ///
    /// Must be called after any operation that mutates the directory tree
    /// (mkdir, rmdir, unlink, rename) so that subsequent readdir calls
    /// return up-to-date results.
    pub fn invalidate_dir_caches(&mut self) {
        for entry in self.entries.values_mut() {
            entry.dir_cache = None;
        }
    }
}
