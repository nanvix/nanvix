// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Sparse Bitmap
//!
//! A bitmap that represents a (potentially sparse) set of bit indices as a
//! vector of disjoint dense [`Bitmap`] chunks, each paired with the global
//! offset it covers. Suitable when a small number of non-contiguous ranges
//! needs to be tracked across an arbitrarily-wide index space.
//!
//! The shape — chunk layout, chunk sizes, chunk boundaries — is fixed at
//! construction time. Callers must declare the full set of tracked
//! ranges up front by passing them to [`SparseBitmap::new`]. This keeps
//! the backing storage, the global capacity, and the per-bit validation
//! surface predictable: there is no way to grow or shrink the collection
//! after construction.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Extern Crates
//==================================================================================================

extern crate alloc;

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::vec::Vec;
use ::bitmap::Bitmap;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A single dense chunk in a sparse bitmap. Covers the global index range
/// `[offset, offset + bitmap.number_of_bits())`.
///
/// The dense bitmap field is intentionally private: [`SparseBitmap`] owns
/// the per-bit state. Callers that reach for a chunk (via
/// [`SparseBitmap::find_chunk`]) only need its boundaries; they go through
/// [`SparseBitmap::set`] / [`SparseBitmap::clear`] / [`SparseBitmap::test`]
/// for the actual bits.
#[derive(Debug)]
pub struct Chunk {
    offset: usize,
    bitmap: Bitmap,
}

impl Chunk {
    /// Global index of the first bit covered by this chunk.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Number of bits in this chunk.
    pub fn num_bits(&self) -> usize {
        self.bitmap.number_of_bits()
    }

    /// End of the chunk's covered range (exclusive).
    pub fn end(&self) -> usize {
        self.offset + self.bitmap.number_of_bits()
    }

    /// Whether the chunk covers `index`.
    fn covers(&self, index: usize) -> bool {
        index >= self.offset && index < self.end()
    }
}

///
/// # Description
///
/// A sparse bitmap: a bitmap whose storage is distributed across a fixed
/// number of dense [`Bitmap`] chunks declared at construction time. Memory
/// use is proportional to the number of chunks and their sizes, not to
/// the index space covered.
///
/// Each chunk is identified by its *global offset* — the starting index it
/// covers in the "global view." Chunks are kept sorted by offset and are
/// guaranteed to be non-overlapping. The set of chunks does not change
/// after construction.
///
/// ## When to use
///
/// Prefer [`SparseBitmap`] over [`Bitmap`] when:
///
/// - Indices come from a wide address space (e.g. the full 32-bit physical
///   address space) but only a small fraction is actually tracked, and
/// - Those tracked indices fall in a small number of clusters, so per-chunk
///   fixed overhead is amortized across many indices.
///
/// A dense [`Bitmap`] is strictly more memory-efficient when nearly every
/// index in its range is active. A [`SparseBitmap`] with a single chunk
/// at offset 0 is semantically equivalent to a [`Bitmap`].
///
/// ## Error semantics
///
/// The API mirrors [`Bitmap`]'s: [`Self::set`] returns
/// [`ErrorCode::ResourceBusy`] when the bit is already set, [`Self::clear`]
/// returns [`ErrorCode::BadAddress`] when the bit is not set (including
/// when no chunk covers the index), and [`Self::test`] returns `Ok(false)`
/// for uncovered indices rather than an error.
///
#[derive(Debug)]
pub struct SparseBitmap {
    /// Chunks, sorted by `offset` and non-overlapping. Populated at
    /// construction time by [`Self::new`]; never mutated afterwards.
    chunks: Vec<Chunk>,
    /// Total capacity across all chunks, in bits. Computed at
    /// construction time so [`Self::capacity`] is O(1).
    capacity_bits: usize,
    /// Index into `chunks` to try first on the next [`Self::alloc`] or
    /// [`Self::alloc_range`] call. Amortizes searches when earlier
    /// chunks are fully allocated: a successful alloc updates this
    /// hint to the satisfying chunk, so subsequent allocs resume there
    /// rather than re-scanning from the front.
    next_chunk_hint: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SparseBitmap {
    ///
    /// # Description
    ///
    /// Creates a sparse bitmap from the full, pre-provisioned set of
    /// `(offset, bitmap)` chunks. The shape is fixed at construction:
    /// there is no later grow/shrink. Callers that need several
    /// disjoint tracked ranges must declare them all here.
    ///
    /// Each bitmap's backing storage is chosen by the caller — heap via
    /// [`Bitmap::new`], or a static BSS buffer via [`Bitmap::from_raw_array`]
    /// when no allocator is available.
    ///
    /// # Parameters
    ///
    /// - `chunks`: Pairs `(offset, bitmap)` where `offset` is the global
    ///   index of bit 0 of `bitmap`. Order doesn't matter — the
    ///   constructor sorts by offset. Ranges must not overlap.
    ///
    /// # Returns
    ///
    /// Upon success, the constructed [`SparseBitmap`]. Upon failure:
    /// - [`ErrorCode::InvalidArgument`] if `chunks` is empty, if a
    ///   chunk's range would overflow `usize`, or if two chunks'
    ///   ranges overlap.
    ///
    pub fn new(chunks: Vec<(usize, Bitmap)>) -> Result<Self, Error> {
        if chunks.is_empty() {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "sparse bitmap requires at least one chunk",
            ));
        }
        // Validate each individual chunk's overflow guard and compute
        // total capacity.
        let mut capacity_bits: usize = 0;
        let mut built: Vec<Chunk> = Vec::with_capacity(chunks.len());
        for (offset, bitmap) in chunks {
            let num_bits = bitmap.number_of_bits();
            offset.checked_add(num_bits).ok_or_else(|| {
                Error::new(ErrorCode::InvalidArgument, "chunk range overflows usize")
            })?;
            capacity_bits = capacity_bits.checked_add(num_bits).ok_or_else(|| {
                Error::new(ErrorCode::InvalidArgument, "capacity overflows usize")
            })?;
            built.push(Chunk { offset, bitmap });
        }

        // Sort by offset, then reject overlaps by walking the sorted
        // list pairwise.
        built.sort_by_key(|c| c.offset);
        for pair in built.windows(2) {
            if pair[0].end() > pair[1].offset {
                return Err(Error::new(ErrorCode::InvalidArgument, "chunk overlaps another chunk"));
            }
        }

        Ok(Self {
            chunks: built,
            capacity_bits,
            next_chunk_hint: 0,
        })
    }

    ///
    /// # Description
    ///
    /// Sets the bit at the given global `index`.
    ///
    /// # Parameters
    ///
    /// - `index`: Global index of the bit to set.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())`. Upon failure:
    /// - [`ErrorCode::InvalidArgument`] if no chunk covers `index`.
    /// - [`ErrorCode::ResourceBusy`] if the bit is already set.
    /// - Any error propagated from [`Bitmap::set`].
    ///
    pub fn set(&mut self, index: usize) -> Result<(), Error> {
        match self.find_chunk_mut(index) {
            Some(chunk) => {
                let local = index - chunk.offset;
                chunk.bitmap.set(local)
            },
            None => Err(Error::new(ErrorCode::InvalidArgument, "no chunk covers index")),
        }
    }

    ///
    /// # Description
    ///
    /// Clears the bit at the given global `index`.
    ///
    /// # Parameters
    ///
    /// - `index`: Global index of the bit to clear.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())`. Upon failure:
    /// - [`ErrorCode::BadAddress`] if no chunk covers `index` (treated as
    ///   "bit is not set," matching [`Bitmap::clear`]'s semantics for a
    ///   cleared bit), or if the bit is already cleared.
    /// - Any error propagated from [`Bitmap::clear`].
    ///
    pub fn clear(&mut self, index: usize) -> Result<(), Error> {
        match self.find_chunk_mut(index) {
            Some(chunk) => {
                let local = index - chunk.offset;
                chunk.bitmap.clear(local)
            },
            None => Err(Error::new(ErrorCode::BadAddress, "bit is not set")),
        }
    }

    ///
    /// # Description
    ///
    /// Returns whether the bit at the given global `index` is set.
    ///
    /// # Parameters
    ///
    /// - `index`: Global index of the bit to test.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the bit is set; `Ok(false)` if it is cleared, *or* if
    /// no chunk covers `index`. Propagates errors from [`Bitmap::test`].
    ///
    pub fn test(&self, index: usize) -> Result<bool, Error> {
        match self.find_chunk(index) {
            Some(chunk) => {
                let local = index - chunk.offset;
                chunk.bitmap.test(local)
            },
            None => Ok(false),
        }
    }

    ///
    /// # Description
    ///
    /// Allocates a single free bit, sets it, and returns its global
    /// index.
    ///
    /// Thin wrapper over [`Self::alloc_range`] with `count = 1`. Lets
    /// single-bit and range allocation share the same hint-maintenance
    /// logic.
    ///
    /// # Returns
    ///
    /// Upon success, the global index of the allocated bit. Upon failure,
    /// [`ErrorCode::OutOfMemory`] if no chunk has a free bit.
    ///
    pub fn alloc(&mut self) -> Result<usize, Error> {
        self.alloc_range(1)
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of `count` free bits, sets them,
    /// and returns the global index of the first bit.
    ///
    /// Starting from the cached [`Self::next_chunk_hint`] and wrapping
    /// around, tries [`Bitmap::alloc_range`] on each chunk whose
    /// capacity is at least `count`. The entire range must fit within
    /// a single chunk; cross-chunk spanning is not supported.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of contiguous bits to allocate. Must be non-zero.
    ///
    /// # Returns
    ///
    /// Upon success, the global index of the first bit in the allocated
    /// range. Upon failure:
    /// - [`ErrorCode::InvalidArgument`] if `count == 0`.
    /// - [`ErrorCode::OutOfMemory`] if no single chunk can satisfy the
    ///   request.
    ///
    pub fn alloc_range(&mut self, count: usize) -> Result<usize, Error> {
        if count == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "count must be non-zero"));
        }
        // By construction (`Self::new` rejects empty input), `chunks`
        // has at least one entry.
        let n: usize = self.chunks.len();

        // Start searching from the hint, then wrap around to the front. This amortizes the cost of
        // scanning past full chunks: a successful alloc updates the hint to the satisfying chunk,
        // so subsequent allocs resume there rather than re-scanning from the front.
        let start: usize = self.next_chunk_hint;
        for step in 0..n {
            let idx = (start + step) % n;
            let chunk = &mut self.chunks[idx];
            if count > chunk.bitmap.number_of_bits() {
                continue;
            }
            match chunk.bitmap.alloc_range(count) {
                Ok(local) => {
                    self.next_chunk_hint = idx;
                    return Ok(chunk.offset + local);
                },
                Err(e) if e.code == ErrorCode::OutOfMemory => continue,
                Err(e) => return Err(e),
            }
        }

        Err(Error::new(ErrorCode::OutOfMemory, "no contiguous free range of the requested size"))
    }

    ///
    /// # Description
    ///
    /// Returns the total capacity across all chunks, in bits. O(1) —
    /// maintained as chunks are registered.
    ///
    pub fn capacity(&self) -> usize {
        self.capacity_bits
    }

    ///
    /// # Description
    ///
    /// Returns the number of chunks currently held.
    ///
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    ///
    /// # Description
    ///
    /// Locates the chunk whose covered range contains `index`, if any.
    /// Binary-searches the offset-sorted chunks in O(log n). Callers
    /// that only need a yes/no coverage answer can use
    /// `find_chunk(index).is_some()`.
    ///
    /// # Parameters
    ///
    /// - `index`: Global bit index to look up.
    ///
    /// # Returns
    ///
    /// `Some(&Chunk)` if a chunk covers `index`; `None` if the index
    /// falls in a gap between chunks or before / after all of them.
    ///
    pub fn find_chunk(&self, index: usize) -> Option<&Chunk> {
        self.find_chunk_index(index).map(|i| &self.chunks[i])
    }

    /// Mutable variant of [`Self::find_chunk`]. Uses the shared
    /// [`Self::find_chunk_index`] helper so the binary-search logic
    /// lives in one place.
    fn find_chunk_mut(&mut self, index: usize) -> Option<&mut Chunk> {
        self.find_chunk_index(index)
            .map(move |i| &mut self.chunks[i])
    }

    /// Returns the index into `self.chunks` of the chunk covering
    /// `index`, if any. Single source of truth for the binary-search
    /// walk; both [`Self::find_chunk`] and [`Self::find_chunk_mut`]
    /// derive from it.
    fn find_chunk_index(&self, index: usize) -> Option<usize> {
        let idx = self.chunks.partition_point(|c| c.offset <= index);
        if idx == 0 {
            return None;
        }
        let candidate = idx - 1;
        if self.chunks[candidate].covers(index) {
            Some(candidate)
        } else {
            None
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bitmap(num_bits: usize) -> Bitmap {
        Bitmap::new(num_bits).expect("bitmap construction failed")
    }

    fn single_chunk(offset: usize, num_bits: usize) -> SparseBitmap {
        SparseBitmap::new(vec![(offset, make_bitmap(num_bits))])
            .expect("single-chunk construction should succeed")
    }

    // --- Construction ---

    #[test]
    fn new_single_chunk() {
        let s = single_chunk(0, 64);
        assert_eq!(s.chunk_count(), 1);
        assert_eq!(s.capacity(), 64);
    }

    #[test]
    fn new_with_offset() {
        let s = single_chunk(1_000, 64);
        assert!(s.find_chunk(1_000).is_some());
        assert!(s.find_chunk(1_063).is_some());
        assert!(s.find_chunk(999).is_none());
        assert!(s.find_chunk(1_064).is_none());
    }

    #[test]
    fn new_sorts_chunks_by_offset() {
        let s = SparseBitmap::new(vec![
            (200, make_bitmap(64)),
            (0, make_bitmap(64)),
            (400, make_bitmap(64)),
        ])
        .unwrap();
        assert_eq!(s.chunk_count(), 3);
        assert!(s.find_chunk(0).is_some());
        assert!(s.find_chunk(200).is_some());
        assert!(s.find_chunk(400).is_some());
        assert!(s.find_chunk(100).is_none());
        assert!(s.find_chunk(300).is_none());
    }

    #[test]
    fn new_rejects_overlap() {
        let err = SparseBitmap::new(vec![(0, make_bitmap(64)), (32, make_bitmap(64))])
            .expect_err("overlap should fail");
        assert_eq!(err.code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn new_accepts_touching_chunks() {
        let s = SparseBitmap::new(vec![(0, make_bitmap(64)), (64, make_bitmap(64))]).unwrap();
        assert_eq!(s.chunk_count(), 2);
    }

    #[test]
    fn new_rejects_usize_overflow() {
        let err = SparseBitmap::new(vec![(usize::MAX - 32, make_bitmap(64))])
            .expect_err("overflow should fail");
        assert_eq!(err.code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn new_rejects_empty() {
        let err = SparseBitmap::new(Vec::new()).expect_err("empty chunk list should fail");
        assert_eq!(err.code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn new_initializes_cached_capacity() {
        let s = SparseBitmap::new(vec![
            (0, make_bitmap(64)),
            (1_000, make_bitmap(128)),
            (2_000, make_bitmap(16)),
        ])
        .unwrap();
        assert_eq!(s.capacity(), 64 + 128 + 16);
        assert_eq!(s.chunk_count(), 3);
    }

    // --- set / clear / test ---

    #[test]
    fn set_clear_test_round_trip() {
        let mut s = single_chunk(100, 64);
        assert!(!s.test(150).unwrap());
        s.set(150).unwrap();
        assert!(s.test(150).unwrap());
        s.clear(150).unwrap();
        assert!(!s.test(150).unwrap());
    }

    #[test]
    fn set_rejects_duplicate_with_resource_busy() {
        let mut s = single_chunk(0, 64);
        s.set(3).unwrap();
        let err = s.set(3).expect_err("duplicate set should fail");
        assert_eq!(err.code, ErrorCode::ResourceBusy);
    }

    #[test]
    fn set_rejects_uncovered_with_invalid_argument() {
        let mut s = single_chunk(0, 64);
        let err = s.set(1_000_000).expect_err("uncovered set should fail");
        assert_eq!(err.code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn clear_rejects_unset_with_bad_address() {
        let mut s = single_chunk(0, 64);
        let err = s.clear(10).expect_err("clear of unset bit should fail");
        assert_eq!(err.code, ErrorCode::BadAddress);
    }

    #[test]
    fn clear_rejects_uncovered_with_bad_address() {
        let mut s = single_chunk(0, 64);
        let err = s
            .clear(1_000_000)
            .expect_err("clear of uncovered bit should fail");
        assert_eq!(err.code, ErrorCode::BadAddress);
    }

    #[test]
    fn test_uncovered_returns_false() {
        let s = single_chunk(100, 64);
        assert!(!s.test(0).unwrap());
        assert!(!s.test(1_000_000).unwrap());
        assert!(!s.test(usize::MAX).unwrap());
    }

    // --- alloc / alloc_range ---

    #[test]
    fn alloc_returns_first_free_bit() {
        let mut s = single_chunk(100, 64);
        let idx = s.alloc().unwrap();
        assert_eq!(idx, 100);
        assert!(s.test(100).unwrap());
    }

    #[test]
    fn alloc_walks_chunks() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(8)), (1_000, make_bitmap(8))]).unwrap();
        for _ in 0..8 {
            s.alloc().unwrap();
        }
        assert_eq!(s.alloc().unwrap(), 1_000);
    }

    #[test]
    fn alloc_hint_amortizes_subsequent_allocs() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(8)), (1_000, make_bitmap(8))]).unwrap();
        for _ in 0..8 {
            s.alloc().unwrap();
        }
        assert_eq!(s.alloc().unwrap(), 1_000);
        s.clear(3).unwrap();
        assert_eq!(s.alloc().unwrap(), 1_001);
    }

    #[test]
    fn alloc_fails_when_all_chunks_full() {
        let mut s = single_chunk(0, 8);
        for _ in 0..8 {
            s.alloc().unwrap();
        }
        let err = s.alloc().expect_err("no free bits left");
        assert_eq!(err.code, ErrorCode::OutOfMemory);
    }

    #[test]
    fn alloc_range_within_chunk() {
        let mut s = single_chunk(1_000, 64);
        let start = s.alloc_range(16).unwrap();
        assert_eq!(start, 1_000);
        for i in 0..16 {
            assert!(s.test(1_000 + i).unwrap());
        }
        assert!(!s.test(1_016).unwrap());
    }

    #[test]
    fn alloc_range_skips_too_small_chunks() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(8)), (1_000, make_bitmap(64))]).unwrap();
        let start = s.alloc_range(16).unwrap();
        assert_eq!(start, 1_000);
    }

    #[test]
    fn alloc_range_does_not_span_chunks() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(64)), (64, make_bitmap(64))]).unwrap();
        let err = s
            .alloc_range(65)
            .expect_err("cross-chunk spanning not supported");
        assert_eq!(err.code, ErrorCode::OutOfMemory);
    }

    #[test]
    fn alloc_range_rejects_zero_count() {
        let mut s = single_chunk(0, 64);
        let err = s.alloc_range(0).expect_err("zero count");
        assert_eq!(err.code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn alloc_range_fails_when_no_chunk_fits() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(8)), (1_000, make_bitmap(8))]).unwrap();
        let err = s.alloc_range(16).expect_err("no chunk big enough");
        assert_eq!(err.code, ErrorCode::OutOfMemory);
    }

    // --- Sparse-use-case scenarios ---

    #[test]
    fn high_address_indices_work() {
        let base = 0xFFFF_0000 >> 12;
        let mut s = single_chunk(base, 64);
        let idx = base + 5;
        assert!(!s.test(idx).unwrap());
        s.set(idx).unwrap();
        assert!(s.test(idx).unwrap());
        s.clear(idx).unwrap();
        assert!(!s.test(idx).unwrap());
    }

    #[test]
    fn multiple_ranges_share_no_state() {
        let mut s = SparseBitmap::new(vec![
            (0, make_bitmap(64)),
            (1_000, make_bitmap(64)),
            (2_000, make_bitmap(64)),
        ])
        .unwrap();

        s.set(10).unwrap();
        s.set(1_010).unwrap();
        s.set(2_010).unwrap();

        assert!(s.test(10).unwrap());
        assert!(s.test(1_010).unwrap());
        assert!(s.test(2_010).unwrap());

        assert!(!s.test(11).unwrap());
        assert!(!s.test(1_011).unwrap());
        assert!(!s.test(2_011).unwrap());
    }

    #[test]
    fn gaps_between_chunks_are_not_covered() {
        let s = SparseBitmap::new(vec![(0, make_bitmap(64)), (1_000, make_bitmap(64))]).unwrap();
        assert!(s.find_chunk(0).is_some());
        assert!(s.find_chunk(63).is_some());
        assert!(s.find_chunk(64).is_none());
        assert!(s.find_chunk(500).is_none());
        assert!(s.find_chunk(999).is_none());
        assert!(s.find_chunk(1_000).is_some());
    }

    // --- Bitmap-as-special-case ---

    #[test]
    fn behaves_like_bitmap_when_single_chunk_at_zero() {
        let mut sparse = single_chunk(0, 64);
        let mut dense = make_bitmap(64);

        for i in [0, 5, 17, 63] {
            sparse.set(i).unwrap();
            dense.set(i).unwrap();
        }

        for i in 0..64 {
            assert_eq!(sparse.test(i).unwrap(), dense.test(i).unwrap(), "mismatch at bit {}", i);
        }
    }
}
