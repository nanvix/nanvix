// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::core::cell::Cell;

/// Metadata of a regular file of the given size.
fn file(size: u64) -> Stat {
    Stat::new(size, false, 1, 2, 3)
}

/// Queries the cache for a path, counting how many times the filesystem has to answer.
///
/// `answer` is what the filesystem says if it is asked.
fn query(
    cache: &mut MetadataCache,
    epoch: u64,
    path: &str,
    answer: Result<Stat, Fat32Error>,
    asked: &Cell<usize>,
) -> Result<Stat, Fat32Error> {
    cache.stat(epoch, path, || {
        asked.set(asked.get() + 1);
        answer
    })
}

/// Tests that a path that is not cached is answered by the filesystem.
#[test]
fn miss_asks_the_filesystem() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);

    assert_eq!(
        query(&mut cache, 1, "a", Ok(file(10)), &asked),
        Ok(file(10)),
        "a miss should return the answer of the filesystem"
    );
    assert_eq!(asked.get(), 1, "a miss should ask the filesystem once");
}

/// Tests that a path that is cached is answered without asking the filesystem.
#[test]
fn hit_does_not_ask_the_filesystem() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    query(&mut cache, 1, "a", Ok(file(10)), &asked).expect("first query should succeed");

    // The filesystem would now say something else, so only the cache can explain the answer.
    assert_eq!(
        query(&mut cache, 1, "a", Ok(file(99)), &asked),
        Ok(file(10)),
        "a hit should return the cached metadata"
    );
    assert_eq!(asked.get(), 1, "a hit should not ask the filesystem");
}

/// Tests that a path that does not exist is cached too.
#[test]
fn not_found_is_cached() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);

    assert_eq!(
        query(&mut cache, 1, "missing", Err(Fat32Error::NotFound), &asked),
        Err(Fat32Error::NotFound),
        "a miss should return the error of the filesystem"
    );
    assert_eq!(
        query(&mut cache, 1, "missing", Ok(file(1)), &asked),
        Err(Fat32Error::NotFound),
        "a hit should report that the path does not exist"
    );
    assert_eq!(asked.get(), 1, "a missing path should be asked only once");
}

/// Tests that errors other than a missing path are not cached, since they may not recur.
#[test]
fn other_errors_are_not_cached() {
    for error in [
        Fat32Error::NotADirectory,
        Fat32Error::InvalidArgument,
        Fat32Error::IoError,
        Fat32Error::OutOfMemory,
    ] {
        let mut cache: MetadataCache = MetadataCache::new();
        let asked: Cell<usize> = Cell::new(0);

        assert_eq!(
            query(&mut cache, 1, "a", Err(error), &asked),
            Err(error),
            "the error of the filesystem should be returned"
        );
        assert_eq!(
            query(&mut cache, 1, "a", Ok(file(10)), &asked),
            Ok(file(10)),
            "a path that failed with {error:?} should be asked again"
        );
        assert_eq!(asked.get(), 2, "{error:?} should not be cached");
    }
}

/// Tests that a new epoch drops the metadata of the paths that exist.
#[test]
fn new_epoch_drops_metadata() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    query(&mut cache, 1, "a", Ok(file(10)), &asked).expect("first query should succeed");

    assert_eq!(
        query(&mut cache, 2, "a", Ok(file(20)), &asked),
        Ok(file(20)),
        "metadata of an earlier epoch should not be returned"
    );
    assert_eq!(
        query(&mut cache, 2, "a", Ok(file(30)), &asked),
        Ok(file(20)),
        "the answer of the new epoch should be cached"
    );
    assert_eq!(asked.get(), 2, "the new epoch should ask the filesystem once");
}

/// Tests that a new epoch drops the paths that were found to be missing.
#[test]
fn new_epoch_drops_not_found() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    query(&mut cache, 1, "a", Err(Fat32Error::NotFound), &asked)
        .expect_err("first query should fail");

    assert_eq!(
        query(&mut cache, 2, "a", Ok(file(20)), &asked),
        Ok(file(20)),
        "a path that appeared should not be reported as missing"
    );
    assert_eq!(asked.get(), 2, "the new epoch should ask the filesystem again");
}

/// Tests that a new epoch drops every path, not only the one that is queried.
#[test]
fn new_epoch_drops_every_path() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    query(&mut cache, 1, "a", Ok(file(1)), &asked).expect("query should succeed");
    query(&mut cache, 1, "b", Ok(file(2)), &asked).expect("query should succeed");
    query(&mut cache, 2, "c", Ok(file(3)), &asked).expect("query should succeed");
    assert_eq!(asked.get(), 3, "each path should be asked once");

    assert_eq!(query(&mut cache, 2, "a", Ok(file(10)), &asked), Ok(file(10)));
    assert_eq!(query(&mut cache, 2, "b", Ok(file(20)), &asked), Ok(file(20)));
    assert_eq!(
        query(&mut cache, 2, "c", Ok(file(30)), &asked),
        Ok(file(3)),
        "a path recorded at the new epoch should survive"
    );
    assert_eq!(asked.get(), 5, "only the paths of the earlier epoch should be asked again");
}

/// Tests that paths are cached exactly as they are queried, so that spellings of the same entry
/// do not share an answer.
#[test]
fn paths_are_exact_keys() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    let paths: [&str; 4] = ["a", "A", "a/b", "a/"];

    for (size, path) in paths.iter().enumerate() {
        assert_eq!(
            query(&mut cache, 1, path, Ok(file(size as u64)), &asked),
            Ok(file(size as u64)),
            "{path:?} should not be answered with the metadata of another path"
        );
    }
    assert_eq!(asked.get(), paths.len(), "each spelling should be asked once");

    for (size, path) in paths.iter().enumerate() {
        assert_eq!(
            query(&mut cache, 1, path, Err(Fat32Error::IoError), &asked),
            Ok(file(size as u64)),
            "{path:?} should be cached"
        );
    }
    assert_eq!(asked.get(), paths.len(), "cached spellings should not be asked again");
}

/// Tests that the cache holds a bounded number of paths and evicts the least recently used one.
#[test]
fn cache_is_bounded() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    let paths: ::alloc::vec::Vec<String> =
        (0..=STAT_CACHE_CAPACITY).map(|i| ::alloc::format!("path{i}")).collect();

    for path in &paths {
        query(&mut cache, 1, path, Ok(file(1)), &asked).expect("query should succeed");
    }
    assert_eq!(asked.get(), paths.len(), "each path should be asked once");

    // The last paths fit, so they are all cached.
    for path in &paths[1..] {
        assert_eq!(
            query(&mut cache, 1, path, Err(Fat32Error::IoError), &asked),
            Ok(file(1)),
            "{path:?} should be cached"
        );
    }
    assert_eq!(asked.get(), paths.len(), "cached paths should not be asked again");

    // The first path did not fit, so it was evicted.
    assert_eq!(
        query(&mut cache, 1, &paths[0], Ok(file(2)), &asked),
        Ok(file(2)),
        "the least recently used path should have been evicted"
    );
    assert_eq!(asked.get(), paths.len() + 1, "the evicted path should be asked again");
}

/// Tests that a hit protects a path from eviction.
#[test]
fn hit_refreshes_recency() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    let paths: ::alloc::vec::Vec<String> =
        (0..STAT_CACHE_CAPACITY).map(|i| ::alloc::format!("path{i}")).collect();
    for path in &paths {
        query(&mut cache, 1, path, Ok(file(1)), &asked).expect("query should succeed");
    }

    // Use the oldest path, which makes the second oldest one the least recently used.
    assert_eq!(query(&mut cache, 1, &paths[0], Err(Fat32Error::IoError), &asked), Ok(file(1)));
    query(&mut cache, 1, "extra", Ok(file(1)), &asked).expect("query should succeed");
    assert_eq!(asked.get(), paths.len() + 1, "only the extra path should be asked");

    assert_eq!(
        query(&mut cache, 1, &paths[0], Err(Fat32Error::IoError), &asked),
        Ok(file(1)),
        "a path that was used recently should have been kept"
    );
    assert_eq!(
        query(&mut cache, 1, &paths[1], Ok(file(2)), &asked),
        Ok(file(2)),
        "the least recently used path should have been evicted"
    );
    assert_eq!(asked.get(), paths.len() + 2, "only the evicted path should be asked again");
}

/// A region of `len` bytes that starts at the address `addr`, which is never dereferenced.
fn region(addr: usize, len: usize) -> Option<(*const u8, usize)> {
    Some((::core::ptr::without_provenance(addr), len))
}

/// Queries the cache for the region of a path, counting how many times the filesystem has to
/// answer.
///
/// `answer` is what the filesystem says if it is asked.
fn query_region(
    cache: &mut MetadataCache,
    epoch: u64,
    path: &str,
    answer: Option<(*const u8, usize)>,
    asked: &Cell<usize>,
) -> Option<(*const u8, usize)> {
    cache.raw_region(epoch, path, || {
        asked.set(asked.get() + 1);
        answer
    })
}

/// Tests that a region that is not cached is answered by the filesystem.
#[test]
fn region_miss_asks_the_filesystem() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);

    assert_eq!(
        query_region(&mut cache, 1, "a", region(0x1000, 10), &asked),
        region(0x1000, 10),
        "a miss should return the answer of the filesystem"
    );
    assert_eq!(asked.get(), 1, "a miss should ask the filesystem once");
}

/// Tests that a region that is cached is answered without asking the filesystem.
#[test]
fn region_hit_does_not_ask_the_filesystem() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    query_region(&mut cache, 1, "a", region(0x1000, 10), &asked);

    // The filesystem would now say something else, so only the cache can explain the answer.
    assert_eq!(
        query_region(&mut cache, 1, "a", region(0x2000, 99), &asked),
        region(0x1000, 10),
        "a hit should return the cached region"
    );
    assert_eq!(asked.get(), 1, "a hit should not ask the filesystem");
}

/// Tests that the lack of a region is cached too.
#[test]
fn absent_region_is_cached() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);

    assert_eq!(query_region(&mut cache, 1, "a", None, &asked), None);
    assert_eq!(
        query_region(&mut cache, 1, "a", region(0x1000, 10), &asked),
        None,
        "a hit should report that there is no region"
    );
    assert_eq!(asked.get(), 1, "a path without a region should be asked only once");
}

/// Tests that a new epoch drops the regions, and the lack of them.
#[test]
fn new_epoch_drops_regions() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    query_region(&mut cache, 1, "found", region(0x1000, 10), &asked);
    query_region(&mut cache, 1, "absent", None, &asked);

    assert_eq!(
        query_region(&mut cache, 2, "found", region(0x2000, 20), &asked),
        region(0x2000, 20),
        "a region of an earlier epoch should not be returned"
    );
    assert_eq!(
        query_region(&mut cache, 2, "absent", region(0x3000, 30), &asked),
        region(0x3000, 30),
        "a path that gained data should not be reported as having none"
    );
    assert_eq!(asked.get(), 4, "the new epoch should ask the filesystem again");
}

/// Tests that metadata and regions share one epoch, so that a query of either kind that sees a
/// new epoch drops the entries of both kinds.
#[test]
fn new_epoch_drops_regions_and_metadata_together() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    query_region(&mut cache, 1, "a", region(0x1000, 10), &asked);
    query(&mut cache, 1, "a", Ok(file(10)), &asked).expect("query should succeed");
    assert_eq!(asked.get(), 2, "each kind of query should be asked once");

    // Only a metadata query sees the new epoch, but the region of the earlier epoch must go.
    query(&mut cache, 2, "b", Ok(file(20)), &asked).expect("query should succeed");
    assert_eq!(
        query_region(&mut cache, 2, "a", region(0x2000, 20), &asked),
        region(0x2000, 20),
        "a region of an earlier epoch should not be returned"
    );

    // Only a region query sees the new epoch, but the metadata of the earlier epoch must go.
    query_region(&mut cache, 3, "c", region(0x3000, 30), &asked);
    assert_eq!(
        query(&mut cache, 3, "b", Ok(file(30)), &asked),
        Ok(file(30)),
        "metadata of an earlier epoch should not be returned"
    );
    assert_eq!(asked.get(), 6, "every entry of an earlier epoch should have been asked again");
}

/// Tests that the metadata and the region of a path are cached independently.
#[test]
fn regions_and_metadata_are_independent() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);

    // A path can have metadata and no data to read in place, such as an empty file.
    query(&mut cache, 1, "a", Ok(file(0)), &asked).expect("query should succeed");
    assert_eq!(query_region(&mut cache, 1, "a", None, &asked), None);
    assert_eq!(asked.get(), 2, "the region should not be answered by the metadata");

    assert_eq!(query(&mut cache, 1, "a", Err(Fat32Error::IoError), &asked), Ok(file(0)));
    assert_eq!(
        query_region(&mut cache, 1, "a", region(0x1000, 10), &asked),
        None,
        "the metadata should not change the region"
    );
    assert_eq!(asked.get(), 2, "both answers should have been cached");
}

/// Tests that the cache holds a bounded number of regions and evicts the least recently used one.
#[test]
fn region_cache_is_bounded() {
    let mut cache: MetadataCache = MetadataCache::new();
    let asked: Cell<usize> = Cell::new(0);
    let paths: ::alloc::vec::Vec<String> =
        (0..=REGION_CACHE_CAPACITY).map(|i| ::alloc::format!("path{i}")).collect();

    for (i, path) in paths.iter().enumerate() {
        query_region(&mut cache, 1, path, region(0x1000 * (i + 1), 1), &asked);
    }
    assert_eq!(asked.get(), paths.len(), "each path should be asked once");

    // The last paths fit, so they are all cached.
    for (i, path) in paths.iter().enumerate().skip(1) {
        assert_eq!(
            query_region(&mut cache, 1, path, None, &asked),
            region(0x1000 * (i + 1), 1),
            "{path:?} should be cached"
        );
    }
    assert_eq!(asked.get(), paths.len(), "cached paths should not be asked again");

    // The first path did not fit, so it was evicted.
    assert_eq!(
        query_region(&mut cache, 1, &paths[0], region(0x1000, 2), &asked),
        region(0x1000, 2),
        "the least recently used path should have been evicted"
    );
    assert_eq!(asked.get(), paths.len() + 1, "the evicted path should be asked again");
}
