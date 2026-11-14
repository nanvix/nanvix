// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Metadata cache of a mounted filesystem.
//!
//! This module provides a small, bounded cache of the answers that a mount gives to metadata
//! queries, so that repeated queries about the same path skip the directory walk of the FAT
//! backend. Paths that do not exist are cached too, because probing for them (for example, when
//! searching a list of directories for a file) is common, and every probe otherwise walks a
//! directory just to find nothing.

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

//==================================================================================================
// Structures
//==================================================================================================

/// Bounded cache of the metadata queries that one mounted filesystem answers.
///
/// # Description
///
/// A query is answered from the cache when the same path was queried before, while the filesystem
/// was at the same modification epoch, and by the filesystem otherwise. Both the metadata of an
/// existing path and the fact that a path does not exist are cached. The cache is keyed by the path
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
pub(crate) struct MetadataCache {
    /// Modification epoch of the filesystem that the cached entries were recorded at.
    epoch: u64,
    /// Outcomes of `stat()` queries, keyed by the path relative to the mount. `None` records a path
    /// that does not exist.
    stats: Cache<String, Option<Stat>>,
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

    /// Drops every entry that was not recorded at the given epoch.
    ///
    /// # Parameters
    ///
    /// - `epoch`: Current modification epoch of the filesystem.
    fn sync(&mut self, epoch: u64) {
        if self.epoch != epoch {
            self.stats.clear();
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
