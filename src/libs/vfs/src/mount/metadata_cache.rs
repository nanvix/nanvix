// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Metadata cache of a mounted filesystem.
//!
//! This module provides a small, bounded cache of the answers that a mount gives to the queries
//! that look a path up: what its metadata is, and where its data can be read in place. Repeated
//! queries about the same path then skip the directory walk of the FAT backend. Paths that do not
//! exist are cached too, because probing for them (for example, when searching a list of
//! directories for a file) is common, and every probe otherwise walks a directory just to find
//! nothing.

//==================================================================================================
// Imports
//==================================================================================================

use crate::filesystem::Stat;
use ::alloc::string::String;
use ::cache::Cache;
use ::fat32::Fat32Error;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of paths whose `stat()` outcome a [`MetadataCache`] retains.
///
/// Kept small to bound guest memory use: the cache is bounded by its number of entries, and each
/// entry holds a path and a few dozen bytes of metadata.
pub(crate) const STAT_CACHE_CAPACITY: usize = 64;

/// Maximum number of paths whose `file_raw_region()` outcome a [`MetadataCache`] retains.
///
/// Kept small for the same reason as [`STAT_CACHE_CAPACITY`]. Only the files that are read in
/// place, such as executables, libraries and data files, need an entry, so fewer entries are kept
/// than for the paths that are probed.
pub(crate) const REGION_CACHE_CAPACITY: usize = 32;

//==================================================================================================
// Structures
//==================================================================================================

/// Location of the data of a file that is stored contiguously in the FAT image of a mount.
#[derive(Clone, Copy)]
struct RawRegion {
    /// Address of the first byte of the data, inside the FAT image.
    ptr: *const u8,
    /// Size of the data, in bytes.
    len: usize,
}

// SAFETY: A RawRegion is only an address and a length, which the cache stores and copies but never
// dereferences. Moving it to another thread is harmless, and whoever dereferences the address that
// the cache hands out is responsible for doing so soundly, exactly as for the address that
// Fat::file_raw_region() returns.
unsafe impl Send for RawRegion {}

/// Bounded cache of the lookups that one mounted filesystem answers.
///
/// # Description
///
/// Two kinds of lookups are cached, each in a table of its own: the metadata of a path, and the
/// region of the FAT image where the data of a file can be read in place. A lookup is answered from
/// the cache when the same path was looked up before, while the filesystem was at the same
/// modification epoch, and by the filesystem otherwise. Both what a lookup finds and the fact that
/// it finds nothing, such as a path that does not exist, are cached. The cache is keyed by the path
/// relative to the mount, exactly as it was queried, so different spellings of the name of one entry
/// (FAT matches names regardless of their case) are cached independently.
///
/// # Coherence
///
/// The cache does not track which paths a modification affects. Instead, every query tells the
/// cache the current modification epoch of the filesystem, see
/// [`Fat::epoch()`](::fat32::Fat::epoch), and the cache drops all of its entries when it sees an
/// epoch other than the one that they were recorded at. The filesystem advances its epoch before it
/// is modified in any way, including by the writes of open file handles, so an entry that survives
/// is the answer that the filesystem would give now.
///
/// Dropping every entry is coarse, but it is simple and always right: a modification can affect
/// many paths at once (renaming a directory moves every path below it), and the filesystem is the
/// only one who knows which of them. It also keeps a filesystem that rarely changes, such as the
/// initial ramdisk, fully cached, and a read-only filesystem never changes its epoch at all.
///
/// A region points into the image of the filesystem, so the cache must not outlive the filesystem.
/// It does not, because the mount that owns the cache owns the filesystem too.
pub(crate) struct MetadataCache {
    /// Modification epoch of the filesystem that the cached entries were recorded at.
    epoch: u64,
    /// Outcomes of `stat()` queries, keyed by the path relative to the mount. `None` records a path
    /// that does not exist.
    stats: Cache<String, Option<Stat>>,
    /// Outcomes of `file_raw_region()` queries, keyed by the path relative to the mount. `None`
    /// records a path that has no data to read in place: it does not name a file, or the file is
    /// empty or not stored contiguously.
    regions: Cache<String, Option<RawRegion>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl MetadataCache {
    /// Creates an empty cache.
    ///
    /// The cache adopts the epoch of the filesystem the first time that it is queried.
    pub(crate) fn new() -> Self {
        Self {
            epoch: 0,
            stats: Cache::new(STAT_CACHE_CAPACITY),
            regions: Cache::new(REGION_CACHE_CAPACITY),
        }
    }

    /// Answers a `stat()` query from the cache, or from the filesystem when the cache cannot.
    ///
    /// The metadata of an existing path and [`Fat32Error::NotFound`] are cached. Any other error is
    /// returned without being cached, because it may not happen again.
    ///
    /// # Parameters
    ///
    /// - `epoch`: Current modification epoch of the filesystem.
    /// - `path`: Path to query, relative to the mount.
    /// - `query`: Asks the filesystem for the metadata of `path`. It runs only on a cache miss.
    ///
    /// # Returns
    ///
    /// The metadata of `path`.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotFound`] if `path` doesn't exist.
    /// - Any other error that `query` returns.
    pub(crate) fn stat<F>(&mut self, epoch: u64, path: &str, query: F) -> Result<Stat, Fat32Error>
    where
        F: FnOnce() -> Result<Stat, Fat32Error>,
    {
        self.sync(epoch);

        if let Some(cached) = self.stats.get(path) {
            let outcome: Option<Stat> = *cached;
            return outcome.ok_or(Fat32Error::NotFound);
        }

        let result: Result<Stat, Fat32Error> = query();
        match result {
            Ok(stat) => self.stats.put(String::from(path), Some(stat)),
            Err(Fat32Error::NotFound) => self.stats.put(String::from(path), None),
            Err(_) => {},
        }
        result
    }

    /// Answers a `file_raw_region()` query from the cache, or from the filesystem when the cache
    /// cannot.
    ///
    /// Both a region and the lack of one are cached. A path has no region when it doesn't name a
    /// file, or names a file that is empty or is not stored contiguously. None of that changes
    /// unless the filesystem is modified, which is what the epoch tracks, so it is as safe to
    /// remember as a region is.
    ///
    /// # Parameters
    ///
    /// - `epoch`: Current modification epoch of the filesystem.
    /// - `path`: Path to query, relative to the mount.
    /// - `query`: Asks the filesystem for the region of `path`. It runs only on a cache miss.
    ///
    /// # Returns
    ///
    /// A pointer to the data of the file and its size, if the file can be read in place.
    pub(crate) fn raw_region<F>(
        &mut self,
        epoch: u64,
        path: &str,
        query: F,
    ) -> Option<(*const u8, usize)>
    where
        F: FnOnce() -> Option<(*const u8, usize)>,
    {
        self.sync(epoch);

        if let Some(cached) = self.regions.get(path) {
            let outcome: Option<RawRegion> = *cached;
            return outcome.map(|region| (region.ptr, region.len));
        }

        let region: Option<(*const u8, usize)> = query();
        self.regions.put(
            String::from(path),
            region.map(|(ptr, len)| RawRegion {
                ptr,
                len,
            }),
        );
        region
    }

    /// Drops every entry that was not recorded at the given epoch.
    ///
    /// # Parameters
    ///
    /// - `epoch`: Current modification epoch of the filesystem.
    fn sync(&mut self, epoch: u64) {
        if self.epoch != epoch {
            self.stats.clear();
            self.regions.clear();
            self.epoch = epoch;
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
