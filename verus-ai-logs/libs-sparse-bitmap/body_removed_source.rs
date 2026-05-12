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
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]

//==================================================================================================
// Extern Crates
//==================================================================================================

extern crate alloc;

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::vec::Vec;
use ::bitmap::Bitmap;
#[cfg(verus_keep_ghost)]
use ::bitmap::BitmapView;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("lib.spec.rs");
#[cfg(verus_keep_ghost)]
include!("lib.proof.rs");

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
#[verus_verify(external_derive)]
#[derive(Debug)]
pub struct Chunk {
    offset: usize,
    bitmap: Bitmap,
}

#[verus_verify]
impl Chunk {
    /// Global index of the first bit covered by this chunk.
    #[verus_spec(result =>
        ensures
            result as int == self.spec_offset(),
    )]
    pub fn offset(&self) -> usize { ... }

    /// Number of bits in this chunk.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result as int == self.spec_num_bits(),
            result > 0,
    )]
    pub fn num_bits(&self) -> usize { ... }

    /// End of the chunk's covered range (exclusive).
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result as int == self.spec_end(),
    )]
    pub fn end(&self) -> usize { ... }

    /// Whether the chunk covers `index`.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result == self.spec_covers(index as int),
    )]
    fn covers(&self, index: usize) -> bool { ... }
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
#[verus_verify(external_derive)]
#[derive(Debug)]
#[verus_verify]
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

#[verus_verify]
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
    // VERUS: moved to verus! block below for loop invariant support

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
    // VERUS: moved to verus! block below for while/invariant support

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
    // VERUS: moved to verus! block below for remove+insert pattern

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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result is Ok,
            result matches Ok(b) ==> {
                &&& b == self@.is_bit_set(index as int)
            },
    )]
    pub fn test(&self, index: usize) -> Result<bool, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates a single free bit, sets it, and returns its global
    /// index.
    ///
    /// Thin wrapper over [`Self::alloc_range`] with `count = 1`. Lets
    /// single-bit and range allocation share the same hint-maintenance
    /// and cross-chunk logic.
    ///
    /// # Returns
    ///
    /// Upon success, the global index of the allocated bit. Upon failure,
    /// [`ErrorCode::OutOfMemory`] if no chunk has a free bit.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            match result {
                Ok(idx) => {
                    &&& self.inv()
                    &&& old(self)@.is_covered(idx as int)
                    &&& !old(self)@.is_bit_set(idx as int)
                    &&& self@.set_bits =~= old(self)@.set_bits.insert(idx as int)
                    &&& self@.chunks =~= old(self)@.chunks
                },
                Err(e) => {
                    &&& self.inv()
                    &&& old(self)@.is_full()
                    &&& self@ =~= old(self)@
                    &&& e.code == ErrorCode::OutOfMemory
                },
            },
    )]
    // VERUS REWRITE: intermediate variable `r` added for proof block
    pub fn alloc(&mut self) -> Result<usize, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of `count` free bits, sets them,
    /// and returns the global index of the first bit.
    ///
    /// ## Search order
    ///
    /// 1. **Single-chunk pass.** Starting from the cached
    ///    [`Self::next_chunk_hint`] and wrapping around, try
    ///    [`Bitmap::alloc_range`] on each chunk whose capacity is at
    ///    least `count`. Fast path.
    ///
    /// 2. **Cross-chunk pass.** If no single chunk fits, walk chunk
    ///    pairs looking for a run that starts inside one chunk, spans
    ///    its suffix, and continues into the prefix of the next chunk
    ///    (and possibly further chunks beyond that). Only *touching*
    ///    chunks — where `chunk[i].end() == chunk[i+1].offset` — are
    ///    eligible, since a gap between chunks means the global indices
    ///    aren't actually contiguous.
    ///
    /// The cross-chunk pass preserves the invariant that every returned
    /// range is genuinely contiguous in the global index space. Callers
    /// that don't want cross-chunk allocations can pre-provision non-
    /// touching chunks.
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
    /// - [`ErrorCode::OutOfMemory`] if neither pass can satisfy the
    ///   request.
    ///
    // VERUS: alloc_range moved to verus! block below for body verification.
    // Original had #[verifier::external_body]; now verified with remove/insert pattern,
    // while loops, and pattern matching (no continue, no &mut indexing).

    // try_alloc_cross_chunk_from: moved to verus! block below for body verification.

    ///
    /// # Description
    ///
    /// Returns the total capacity across all chunks, in bits. O(1) —
    /// maintained as chunks are registered.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result as int == self@.capacity(),
    )]
    pub fn capacity(&self) -> usize { ... }

    ///
    /// # Description
    ///
    /// Returns the number of chunks currently held.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result as int == self@.chunk_count(),
    )]
    pub fn chunk_count(&self) -> usize { ... }

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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result.is_some() <==> self@.is_covered(index as int),
            result matches Some(chunk) ==> {
                &&& chunk.spec_offset() <= index as int
                &&& (index as int) < chunk.spec_end()
                &&& chunk.inv()
            },
    )]
    pub fn find_chunk(&self, index: usize) -> Option<&Chunk> { ... }

}

verus! {

/// Verified selection sort: sorts chunks by offset using remove + insert.
fn sort_chunks_by_offset(built: &mut Vec<Chunk>)
    requires
        forall|i: int| 0 <= i < old(built)@.len() ==> (#[trigger] old(built)@[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < old(built)@.len() ==> old(built)@[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < old(built)@.len() ==> old(built)@[i].bitmap@.set_bits.finite(),
        forall|i: int| #![auto] 0 <= i < old(built)@.len() ==>
            old(built)@[i].offset as int + old(built)@[i].bitmap@.num_bits <= usize::MAX as int,
    ensures
        built@.len() == old(built)@.len(),
        forall|i: int, j: int| 0 <= i < j < built@.len()
            ==> (#[trigger] built@[i]).offset <= (#[trigger] built@[j]).offset,
        forall|i: int| 0 <= i < built@.len() ==> (#[trigger] built@[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < built@.len() ==> built@[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < built@.len() ==> built@[i].bitmap@.set_bits.finite(),
        forall|i: int| #![auto] 0 <= i < built@.len() ==>
            built@[i].offset as int + built@[i].bitmap@.num_bits <= usize::MAX as int,
        chunk_seq_capacity(built@, 0) == chunk_seq_capacity(old(built)@, 0),
        lifted_set_bits(built@, 0) =~= lifted_set_bits(old(built)@, 0),
{ ... }

impl SparseBitmap {
    /// Creates a sparse bitmap from the full, pre-provisioned set of
    /// `(offset, bitmap)` chunks.
    pub fn new(chunks: Vec<(usize, Bitmap)>) -> (result: Result<Self, Error>)
        requires
            forall|i: int| 0 <= i < chunks@.len() ==> (#[trigger] chunks@[i]).1.inv(),
        ensures
            result matches Ok(sb) ==> {
                &&& sb.inv()
                &&& sb@.chunk_count() == chunks@.len()
                &&& sb@.capacity() == input_capacity(chunks@, 0)
                &&& sb@.set_bits =~= input_lifted_set_bits(chunks@, 0)
            },
            result matches Err(e) ==> e.code == ErrorCode::InvalidArgument,
    { ... }

    /// Commit phase of cross-chunk allocation: sets bits in the entry
    /// chunk's trailing region and in subsequent touching chunks.
    ///
    /// Factored out of `try_alloc_cross_chunk_from` to keep each function
    /// within the solver's resource limit.
    #[verifier::rlimit(20)]
    fn commit_cross_chunk_alloc(
        &mut self,
        entry: usize,
        count: usize,
        trailing_free: usize,
        entry_cap: usize,
        last_chunk: usize,
        Ghost(phase1b_free_prefixes): Ghost<Seq<int>>,
        Ghost(old_chunks_seq): Ghost<Seq<Chunk>>,
    ) -> (global_start: usize)
        requires
            old(self).inv(),
            old(self).chunks@ =~= old_chunks_seq,
            entry < old_chunks_seq.len(),
            count > 0,
            trailing_free > 0,
            trailing_free <= entry_cap,
            entry_cap as int == old_chunks_seq[entry as int].bitmap@.num_bits,
            last_chunk >= entry,
            last_chunk < old_chunks_seq.len(),
            // Entry chunk trailing bits are free
            forall|b: int| (entry_cap - trailing_free) as int <= b < entry_cap as int
                ==> !old_chunks_seq[entry as int].bitmap@.set_bits.contains(b),
            // Phase 1b free prefix information
            phase1b_free_prefixes.len() == (last_chunk - entry) as int,
            forall|i: int| 0 <= i < phase1b_free_prefixes.len() ==> {
                let k = entry as int + 1 + i;
                &&& k < old_chunks_seq.len()
                &&& 0 < (#[trigger] phase1b_free_prefixes[i])
                        <= old_chunks_seq[k].bitmap@.num_bits
                &&& forall|b: int| 0 <= b < phase1b_free_prefixes[i]
                    ==> !old_chunks_seq[k].bitmap@.set_bits.contains(b)
            },
            // Each pfp entry: full capacity or final
            forall|i: int| 0 <= i < phase1b_free_prefixes.len() ==> (
                (#[trigger] phase1b_free_prefixes[i])
                    == old_chunks_seq[(entry as int + 1 + i)].bitmap@.num_bits
                || seq_sum_from(phase1b_free_prefixes, i + 1) == 0
            ),
            // Sum of pfp == initial_need
            seq_sum_from(phase1b_free_prefixes, 0) ==
                (if count > trailing_free { (count - trailing_free) as int }
                 else { 0int }),
            // Touching chain on old_chunks_seq
            forall|i: int| entry as int <= i < last_chunk as int ==>
                old_chunks_seq[i].offset as int + old_chunks_seq[i].bitmap@.num_bits
                    == (#[trigger] old_chunks_seq[(i + 1) as int]).offset as int,
        ensures
            self.inv(),
            self@.chunks =~= old(self)@.chunks,
            self@.set_bits =~= old(self)@.set_bits.union(
                BitmapView::range_set(
                    (old_chunks_seq[entry as int].offset + entry_cap - trailing_free) as int,
                    (old_chunks_seq[entry as int].offset + entry_cap - trailing_free + count) as int)),
            global_start as int ==
                (old_chunks_seq[entry as int].offset + entry_cap - trailing_free) as int,
    { ... }


    fn try_alloc_cross_chunk_from(
        &mut self,
        entry: usize,
        count: usize,
    ) -> (result: Result<Option<usize>, Error>)
        requires
            old(self).inv(),
            count > 0,
            entry < old(self).chunks@.len(),
        ensures
            self.inv(),
            self@.chunks =~= old(self)@.chunks,
            result matches Ok(Some(start)) ==> {
                &&& old(self)@.has_contiguous_free_range(start as int, count as int)
                &&& self@.set_bits =~= old(self)@.set_bits.union(
                        BitmapView::range_set(start as int, start as int + count as int))
            },
            result matches Ok(None) ==> {
                &&& self@ =~= old(self)@
                &&& !old(self)@.has_cross_chunk_free_range_from(entry as int, count as int)
            },
            !(result matches Err(_)),
    { ... }


    /// Allocates a contiguous range of `count` free bits and returns the
    /// global start index.
    /// VERUS REWRITE: moved from outside verus! block; applied compile
    /// workarounds (remove/insert, match on ErrorCode, no continue,
    /// while loops instead of for).
    pub fn alloc_range(&mut self, count: usize) -> (result: Result<usize, Error>)
        requires
            old(self).inv(),
        ensures
            match result {
                Ok(start) => {
                    &&& self.inv()
                    &&& count > 0
                    &&& forall|k: int| start as int <= k < start as int + count as int
                        ==> old(self)@.is_covered(k)
                    &&& forall|k: int| start as int <= k < start as int + count as int
                        ==> !old(self)@.is_bit_set(k)
                    &&& self@.set_bits =~= old(self)@.set_bits.union(
                            BitmapView::range_set(start as int, start as int + count as int))
                    &&& self@.chunks =~= old(self)@.chunks
                },
                Err(e) => {
                    &&& self.inv()
                    &&& self@ =~= old(self)@
                    &&& count == 0 ==> e.code == ErrorCode::InvalidArgument
                    &&& count > 0 ==> {
                        &&& !old(self)@.exists_contiguous_free_range(count as int)
                        &&& e.code == ErrorCode::OutOfMemory
                    }
                },
            },
    { ... }

    /// Sets the bit at the given global `index`.
    /// Uses Vec::remove + mutate + Vec::insert to avoid &mut indexing.
    // VERUS REWRITE: find_chunk_mut → find_chunk_index + remove/insert (&mut index unsupported)
    pub fn set(&mut self, index: usize) -> (result: Result<(), Error>)
        requires
            old(self).inv(),
        ensures
            match result {
                Ok(()) => {
                    &&& self.inv()
                    &&& old(self)@.is_covered(index as int)
                    &&& !old(self)@.is_bit_set(index as int)
                    &&& self@.set_bits =~= old(self)@.set_bits.insert(index as int)
                    &&& self@.chunks =~= old(self)@.chunks
                },
                Err(_) => {
                    &&& self.inv()
                    &&& self@ =~= old(self)@
                    &&& !old(self)@.is_covered(index as int) || old(self)@.is_bit_set(index as int)
                },
            },
    { ... }

    /// Clears the bit at the given global `index`.
    /// Uses Vec::remove + mutate + Vec::insert to avoid &mut indexing.
    // VERUS REWRITE: find_chunk_mut → find_chunk_index + remove/insert (&mut index unsupported)
    pub fn clear(&mut self, index: usize) -> (result: Result<(), Error>)
        requires
            old(self).inv(),
        ensures
            match result {
                Ok(()) => {
                    &&& self.inv()
                    &&& old(self)@.is_covered(index as int)
                    &&& old(self)@.is_bit_set(index as int)
                    &&& self@.set_bits =~= old(self)@.set_bits.remove(index as int)
                    &&& self@.chunks =~= old(self)@.chunks
                },
                Err(_) => {
                    &&& self.inv()
                    &&& self@ =~= old(self)@
                    &&& !old(self)@.is_covered(index as int) || !old(self)@.is_bit_set(index as int)
                },
            },
    { ... }

    /// Returns the index into `self.chunks` of the chunk covering
    /// `index`, if any. Single source of truth for the binary-search
    /// walk; [`Self::find_chunk`] derives from it.
    // VERUS REWRITE: partition_point → manual binary search (no vstd spec for partition_point)
    fn find_chunk_index(&self, index: usize) -> (result: Option<usize>)
        requires
            self.inv(),
        ensures
            result matches Some(ci) ==> {
                &&& (ci as int) < self@.chunk_count()
                &&& self@.chunk_offset(ci as int) <= index as int
                &&& (index as int) < self@.chunk_end(ci as int)
            },
            result.is_some() <==> self@.is_covered(index as int),
    { ... }
}

} // verus!

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bitmap(num_bits: usize) -> Bitmap { ... }

    fn single_chunk(offset: usize, num_bits: usize) -> SparseBitmap { ... }

    // --- Construction ---

    #[test]
    fn new_single_chunk() { ... }

    #[test]
    fn new_with_offset() { ... }

    #[test]
    fn new_sorts_chunks_by_offset() { ... }

    #[test]
    fn new_rejects_overlap() { ... }

    #[test]
    fn new_accepts_touching_chunks() { ... }

    #[test]
    fn new_rejects_usize_overflow() { ... }

    #[test]
    fn new_rejects_empty() { ... }

    #[test]
    fn new_initializes_cached_capacity() { ... }

    // --- set / clear / test ---

    #[test]
    fn set_clear_test_round_trip() { ... }

    #[test]
    fn set_rejects_duplicate_with_resource_busy() { ... }

    #[test]
    fn set_rejects_uncovered_with_invalid_argument() { ... }

    #[test]
    fn clear_rejects_unset_with_bad_address() { ... }

    #[test]
    fn clear_rejects_uncovered_with_bad_address() { ... }

    #[test]
    fn test_uncovered_returns_false() { ... }

    // --- alloc / alloc_range ---

    #[test]
    fn alloc_returns_first_free_bit() { ... }

    #[test]
    fn alloc_walks_chunks() { ... }

    #[test]
    fn alloc_hint_amortizes_subsequent_allocs() { ... }

    #[test]
    fn alloc_fails_when_all_chunks_full() { ... }

    #[test]
    fn alloc_range_within_chunk() { ... }

    #[test]
    fn alloc_range_skips_too_small_chunks() { ... }

    #[test]
    fn alloc_range_stitches_across_touching_chunks() { ... }

    #[test]
    fn alloc_range_refuses_to_stitch_across_gap() { ... }

    #[test]
    fn alloc_range_stitches_across_three_chunks() { ... }

    #[test]
    fn alloc_range_stitch_requires_free_tail_of_entry() { ... }

    #[test]
    fn alloc_range_rejects_zero_count() { ... }

    #[test]
    fn alloc_range_fails_when_no_chunk_fits() { ... }

    // --- Sparse-use-case scenarios ---

    #[test]
    fn high_address_indices_work() { ... }

    #[test]
    fn multiple_ranges_share_no_state() { ... }

    #[test]
    fn gaps_between_chunks_are_not_covered() { ... }

    // --- Bitmap-as-special-case ---

    #[test]
    fn behaves_like_bitmap_when_single_chunk_at_zero() { ... }
}
