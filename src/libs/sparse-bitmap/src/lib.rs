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
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Number of bits in this chunk.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result as int == self.spec_num_bits(),
            result > 0,
    )]
    pub fn num_bits(&self) -> usize {
        self.bitmap.number_of_bits()
    }

    /// End of the chunk's covered range (exclusive).
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result as int == self.spec_end(),
    )]
    pub fn end(&self) -> usize {
        self.offset + self.bitmap.number_of_bits()
    }

    /// Whether the chunk covers `index`.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result == self.spec_covers(index as int),
    )]
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
    pub fn test(&self, index: usize) -> Result<bool, Error> {
        // VERUS REWRITE: find_chunk(index) → find_chunk_index(index); need chunk index for proof
        match self.find_chunk_index(index) {
            Some(ci) => {
                let chunk = &self.chunks[ci];
                let local = index - chunk.offset;
                let r = chunk.bitmap.test(local);
                proof! {
                    self.lemma_test_chunk(ci as int, local as int);
                }
                r
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
    pub fn alloc(&mut self) -> Result<usize, Error> {
        let r = self.alloc_range(1);
        proof! {
            if r.is_err() {
                lemma_no_free_single_implies_full(old(self)@);
            }
        }
        r
    }

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
    pub fn capacity(&self) -> usize {
        self.capacity_bits
    }

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
    pub fn find_chunk(&self, index: usize) -> Option<&Chunk> {
        // VERUS REWRITE: .map(|i| &self.chunks[i]) → match; closure loses find_chunk_index postcondition context
        match self.find_chunk_index(index) {
            Some(i) => Some(&self.chunks[i]),
            None => None,
        }
    }

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
{
    let ghost old_built = built@;
    let n = built.len();

    let mut i: usize = 0;
    while i < n
        invariant
            n == built@.len(),
            n == old_built.len(),
            i as int <= n as int,
            built@.len() == old_built.len(),
            // Prefix [0, i) is sorted
            forall|k: int, l: int| 0 <= k < l && l < i as int
                ==> (#[trigger] built@[k]).offset <= (#[trigger] built@[l]).offset,
            // Prefix elements <= suffix elements
            forall|k: int, l: int| 0 <= k && k < i as int && i as int <= l && l < n as int
                ==> (#[trigger] built@[k]).offset <= (#[trigger] built@[l]).offset,
            // Element properties preserved
            forall|k: int| 0 <= k < built@.len() ==> (#[trigger] built@[k]).bitmap.inv(),
            forall|k: int| #![auto] 0 <= k < built@.len() ==> built@[k].bitmap@.num_bits > 0,
            forall|k: int| #![auto] 0 <= k < built@.len() ==> built@[k].bitmap@.set_bits.finite(),
            forall|k: int| #![auto] 0 <= k < built@.len() ==>
                built@[k].offset as int + built@[k].bitmap@.num_bits <= usize::MAX as int,
            // Aggregate properties preserved
            chunk_seq_capacity(built@, 0) == chunk_seq_capacity(old_built, 0),
            lifted_set_bits(built@, 0) =~= lifted_set_bits(old_built, 0),
        decreases n - i,
    {
        // Find minimum offset in [i, n)
        let mut min_idx: usize = i;
        let mut j: usize = i + 1;
        while j < n
            invariant
                n == built@.len(),
                i as int <= min_idx as int,
                (min_idx as int) < (n as int),
                (i as int) + 1 <= (j as int),
                (j as int) <= (n as int),
                (min_idx as int) < (j as int),
                // min_idx has smallest offset in [i, j)
                forall|k: int| i as int <= k && k < j as int
                    ==> (#[trigger] built@[min_idx as int]).offset
                        <= (#[trigger] built@[k]).offset,
            decreases n - j,
        {
            if built[j].offset < built[min_idx].offset {
                min_idx = j;
            }
            j += 1;
        }
        // Now min_idx has the smallest offset in [i, n)

        if min_idx != i {
            let ghost pre_swap = built@;
            let chunk = built.remove(min_idx);
            built.insert(i, chunk);

            proof {
                lemma_sort_swap_step(pre_swap, built@, i as int, min_idx as int);
            }
        }

        i += 1;
    }
}

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
    {
        if chunks.is_empty() {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "sparse bitmap requires at least one chunk",
            ));
        }

        let ghost original_chunks = chunks@;
        let n = chunks.len();

        let mut capacity_bits: usize = 0;
        let mut built: Vec<Chunk> = Vec::with_capacity(n);

        for item in iter: chunks
            invariant
                iter.elements == original_chunks,
                built@.len() == iter.pos,
                n == original_chunks.len(),
                0 < n,
                // Propagate precondition
                forall|k: int| 0 <= k < original_chunks.len()
                    ==> (#[trigger] original_chunks[k]).1.inv(),
                // All built chunks have valid bitmaps
                forall|k: int| 0 <= k < built@.len()
                    ==> (#[trigger] built@[k]).bitmap.inv(),
                // Built chunks match input chunks in order
                forall|k: int| #![auto] 0 <= k < built@.len() ==> {
                    &&& built@[k].offset == original_chunks[k].0
                    &&& built@[k].bitmap == original_chunks[k].1
                },
                // Bitmap sizes positive
                forall|k: int| 0 <= k < built@.len()
                    ==> (#[trigger] built@[k]).bitmap@.num_bits > 0,
                // Bitmap set_bits finite
                forall|k: int| 0 <= k < built@.len()
                    ==> (#[trigger] built@[k]).bitmap@.set_bits.finite(),
                // Overflow guards
                forall|k: int| 0 <= k < built@.len() ==>
                    (#[trigger] built@[k]).offset as int + built@[k].bitmap@.num_bits
                        <= usize::MAX as int,
                // Capacity tracks partial sum
                capacity_bits as int == chunk_seq_capacity(built@, 0),
        {
            let offset: usize = item.0;
            let bitmap: Bitmap = item.1;

            proof {
                // In the body, iter.pos has been advanced, so current item
                // index is built@.len() (which is old iter.pos before advance).
                assert(original_chunks[built@.len() as int].1.inv());
            }

            let num_bits = bitmap.number_of_bits();

            proof {
                // number_of_bits() ensures result > 0 and result == bitmap@.num_bits
                // From inv(): bitmap@.wf(). Combined with num_bits > 0 >= 0:
                bitmap@.lemma_set_bits_finite();
            }

            // VERUS REWRITE: ok_or_else(closure)? → match (closure spec chain unsupported)
            match offset.checked_add(num_bits) {
                Some(_) => {},
                None => return Err(Error::new(
                    ErrorCode::InvalidArgument, "chunk range overflows usize",
                )),
            }

            capacity_bits = match capacity_bits.checked_add(num_bits) {
                Some(v) => v,
                None => return Err(Error::new(
                    ErrorCode::InvalidArgument, "capacity overflows usize",
                )),
            };

            proof {
                let ghost old_built = built@;
                let ghost new_chunk = Chunk { offset, bitmap };
                lemma_chunk_seq_capacity_push(old_built, new_chunk, 0);
            }
            built.push(Chunk { offset, bitmap });
        }

        // Connect to input BEFORE sort (built matches original_chunks order)
        proof {
            lemma_chunk_seq_capacity_matches_input(built@, original_chunks, 0);
            lemma_lifted_set_bits_matches_input(built@, original_chunks, 0);
        }
        let ghost capacity_input = input_capacity(original_chunks, 0);
        let ghost set_bits_input = input_lifted_set_bits(original_chunks, 0);

        // Sort by offset
        sort_chunks_by_offset(&mut built);
        // After sort: chunk_seq_capacity and lifted_set_bits preserved (sort ensures)

        // VERUS REWRITE: windows(2) → while loop (Windows lacks ForLoopGhostIteratorNew)
        let mut i: usize = 1;
        while i < built.len()
            invariant
                1 <= i,
                i <= built@.len(),
                built@.len() == n,
                n > 0,
                // All chunks valid
                forall|k: int| 0 <= k < built@.len()
                    ==> (#[trigger] built@[k]).bitmap.inv(),
                forall|k: int| #![auto] 0 <= k < built@.len()
                    ==> built@[k].bitmap@.num_bits > 0,
                forall|k: int| #![auto] 0 <= k < built@.len()
                    ==> built@[k].bitmap@.set_bits.finite(),
                forall|k: int| #![auto] 0 <= k < built@.len() ==>
                    built@[k].offset as int + built@[k].bitmap@.num_bits
                        <= usize::MAX as int,
                // Sorted by offset
                forall|k: int, l: int| 0 <= k < l < built@.len()
                    ==> (#[trigger] built@[k]).offset <= (#[trigger] built@[l]).offset,
                // No overlaps in checked prefix [0, i)
                forall|k: int| 0 <= k < i as int - 1 ==>
                    (#[trigger] built@[k]).offset as int + built@[k].bitmap@.num_bits
                        <= built@[k + 1].offset as int,
                // Carried through (ghost variable form avoids recursive unfolding)
                capacity_bits as int == chunk_seq_capacity(built@, 0),
                chunk_seq_capacity(built@, 0) == capacity_input,
                lifted_set_bits(built@, 0) =~= set_bits_input,
            decreases built@.len() - i,
        {
            proof {
                assert(built@[(i - 1) as int].bitmap.inv());
                assert(built@[(i - 1) as int].offset as int + built@[(i - 1) as int].bitmap@.num_bits <= usize::MAX as int);
            }
            if built[i - 1].end() > built[i].offset {
                return Err(Error::new(ErrorCode::InvalidArgument, "chunk overlaps another chunk"));
            }
            i += 1;
        }

        // Construct result
        let sb = Self {
            chunks: built,
            capacity_bits,
            next_chunk_hint: 0,
        };

        proof {
            lemma_constructor_postcondition(
                &sb, capacity_input, set_bits_input, original_chunks.len() as int);
        }

        Ok(sb)
    }

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
    {
        let take_from_entry: usize = if count < trailing_free { count } else { trailing_free };
        let start_bit_in_entry: usize = entry_cap - trailing_free;
        let ghost old_chunks_seq = old_chunks_seq;  // bind ghost param

        // Remove entry chunk, set bits, re-insert
        let ghost pre_commit_chunks = self.chunks@;
        let ghost pre_commit_view = self@;
        let mut chunk_e = self.chunks.remove(entry);


        let ghost old_entry_set_bits = chunk_e.bitmap@.set_bits;
        let mut bit_e: usize = start_bit_in_entry;
        while bit_e < start_bit_in_entry + take_from_entry
            invariant
                chunk_e.bitmap.inv(),
                chunk_e.offset == old_chunks_seq[entry as int].offset,
                chunk_e.bitmap@.num_bits == old_chunks_seq[entry as int].bitmap@.num_bits,
                start_bit_in_entry + take_from_entry <= entry_cap,
                entry_cap as int == chunk_e.bitmap@.num_bits,
                start_bit_in_entry <= bit_e,
                bit_e <= start_bit_in_entry + take_from_entry,
                chunk_e.bitmap@.set_bits.finite(),
                // Bits not yet processed are still free
                forall|b: int| bit_e as int <= b < (start_bit_in_entry + take_from_entry) as int
                    ==> !chunk_e.bitmap@.set_bits.contains(b),
                // Track cumulative set_bits change
                chunk_e.bitmap@.set_bits =~= old_entry_set_bits.union(
                    BitmapView::range_set(start_bit_in_entry as int, bit_e as int)),
            decreases (start_bit_in_entry + take_from_entry) - bit_e,
        {
            proof {
                assert(!chunk_e.bitmap@.set_bits.contains(bit_e as int));
                assert((bit_e as int) < chunk_e.bitmap@.num_bits);
            }
            let set_result = chunk_e.bitmap.set(bit_e);
            match set_result {
                Ok(()) => {
                    proof {
                        lemma_range_set_insert_end(start_bit_in_entry as int, bit_e as int);
                    }
                },
                Err(_e) => { proof { assert(false); } },
            }
            bit_e = bit_e + 1;
        }

        self.chunks.insert(entry, chunk_e);
        proof {
            lemma_lifted_set_bits_alloc_range(
                old_chunks_seq, self.chunks@, entry as int,
                start_bit_in_entry as int, take_from_entry as int);
        }

        // ── Phase 2b: commit — set bits in subsequent chunks ──
        let mut cur: usize = entry;
        let mut remaining: usize = if count > trailing_free {
            count - trailing_free
        } else {
            0
        };
        // Ghost: track how many global bits committed so far (entry chunk + follow chunks)
        let ghost gs = (old_chunks_seq[entry as int].offset + start_bit_in_entry) as int;
        let ghost mut committed: int = 0;


        while remaining > 0
            invariant
                self.chunks@.len() == old_chunks_seq.len(),
                cur >= entry,
                cur < self.chunks@.len(),
                self.chunks@.len() > 0,
                (self.next_chunk_hint as int) < self.chunks@.len(),
                self.capacity_bits == old(self).capacity_bits,
                forall|i: int| 0 <= i < self.chunks@.len()
                    ==> (#[trigger] self.chunks@[i]).bitmap.inv(),
                forall|i: int| #![auto] 0 <= i < self.chunks@.len()
                    ==> self.chunks@[i].bitmap@.num_bits > 0,
                forall|i: int| #![auto] 0 <= i < self.chunks@.len()
                    ==> self.chunks@[i].bitmap@.set_bits.finite(),
                forall|i: int| #![auto] 0 <= i < self.chunks@.len()
                    ==> self.chunks@[i].offset as int + self.chunks@[i].bitmap@.num_bits
                        <= usize::MAX as int,
                forall|i: int, j: int| #![auto] 0 <= i < j < self.chunks@.len()
                    ==> self.chunks@[i].offset as int + self.chunks@[i].bitmap@.num_bits
                        <= self.chunks@[j].offset as int,
                // Every chunk preserves offset and num_bits from old_chunks_seq
                forall|k: int| #![auto] 0 <= k < self.chunks@.len() ==> {
                    &&& self.chunks@[k].offset == old_chunks_seq[k].offset
                    &&& self.chunks@[k].bitmap@.num_bits == old_chunks_seq[k].bitmap@.num_bits
                },
                // View chunk sequence matches concrete chunks
                self@.chunks.len() == self.chunks@.len(),
                forall|k: int| #![auto] 0 <= k < self.chunks@.len() ==> {
                    &&& self@.chunks[k].0 == self.chunks@[k].offset as int
                    &&& self@.chunks[k].1 == self.chunks@[k].bitmap@.num_bits
                },
                // Chunks past cur haven't been committed yet — identical to old
                forall|k: int| #![auto] cur as int + 1 <= k < self.chunks@.len() as int
                    ==> self.chunks@[k] == old_chunks_seq[k],
                // Phase 1b free prefix info
                phase1b_free_prefixes.len() == (last_chunk - entry) as int,
                cur <= last_chunk,
                last_chunk < self.chunks@.len(),
                // For chunks past cur: free prefix info still applies via old_chunks_seq
                forall|i: int| 0 <= i < phase1b_free_prefixes.len() ==> {
                    let k = entry as int + 1 + i;
                    &&& k < old_chunks_seq.len()
                    &&& 0 < (#[trigger] phase1b_free_prefixes[i]) <= old_chunks_seq[k].bitmap@.num_bits
                    &&& forall|b: int| 0 <= b < phase1b_free_prefixes[i]
                        ==> !old_chunks_seq[k].bitmap@.set_bits.contains(b)
                },
                // Sum invariant: remaining tracks unconsumed pfp entries
                remaining as int == seq_sum_from(phase1b_free_prefixes, (cur - entry) as int),
                // When remaining > 0, we haven't reached last_chunk
                remaining > 0 ==> cur < last_chunk,
                // Each pfp entry either took full capacity or was the final entry
                forall|i: int| 0 <= i < phase1b_free_prefixes.len() ==> (
                    (#[trigger] phase1b_free_prefixes[i]) == old_chunks_seq[(entry as int + 1 + i)].bitmap@.num_bits
                    || seq_sum_from(phase1b_free_prefixes, i + 1) == 0
                ),
                // Lifted set_bits tracking: cumulative change from old_chunks_seq
                // Uses (count - remaining) to avoid arithmetic bridge at loop exit
                lifted_set_bits(self.chunks@, 0) =~=
                    lifted_set_bits(old_chunks_seq, 0).union(
                        BitmapView::range_set(gs,
                            gs + count as int - remaining as int)),
                committed >= 0,
                committed + remaining as int == seq_sum_from(phase1b_free_prefixes, 0),
                // Direct count tracking (avoids post-loop arithmetic bridge)
                take_from_entry as int + committed + remaining as int == count as int,
                // Touching chain position: next chunk to commit starts at gs + count - remaining
                remaining > 0 ==> old_chunks_seq[(cur as int + 1)].offset as int
                    == gs + count as int - remaining as int,
                // Touching chain on old_chunks_seq (needed for maintenance of position invariant)
                forall|i: int| entry as int <= i < last_chunk as int ==>
                    old_chunks_seq[i].offset as int + old_chunks_seq[i].bitmap@.num_bits
                        == (#[trigger] old_chunks_seq[(i + 1) as int]).offset as int,
            ensures remaining == 0usize,
            decreases remaining,
        {
            let n_chunks: usize = self.chunks.len();
            let next = cur + 1;
            if next >= n_chunks {
                proof { assert(cur < last_chunk); assert(false); }
                break;
            }

            let cap_next = self.chunks[next].bitmap.number_of_bits();
            let take: usize = if remaining < cap_next { remaining } else { cap_next };

            // Remove next chunk, set bits [0, take), re-insert
            let ghost pre_inner_chunks = self.chunks@;
            let mut chunk_n = self.chunks.remove(next);

            proof {
                let idx = next as int - entry as int - 1;
                lemma_seq_sum_from_unfold(phase1b_free_prefixes, idx);
                lemma_seq_sum_from_nonneg(phase1b_free_prefixes, idx + 1);
                assert forall|b: int| 0 <= b < take as int
                    implies !chunk_n.bitmap@.set_bits.contains(b)
                by { assert(!old_chunks_seq[next as int].bitmap@.set_bits.contains(b)); }
            }

            let ghost old_next_set_bits = chunk_n.bitmap@.set_bits;
            let mut bit_n: usize = 0;
            while bit_n < take
                invariant
                    chunk_n.bitmap.inv(),
                    chunk_n.offset == pre_inner_chunks[next as int].offset,
                    chunk_n.bitmap@.num_bits == pre_inner_chunks[next as int].bitmap@.num_bits,
                    take <= cap_next,
                    cap_next as int == chunk_n.bitmap@.num_bits,
                    0 <= bit_n <= take,
                    chunk_n.bitmap@.set_bits.finite(),
                    // Bits not yet processed are still free
                    forall|b: int| bit_n as int <= b < take as int
                        ==> !chunk_n.bitmap@.set_bits.contains(b),
                    // Track cumulative set_bits change
                    chunk_n.bitmap@.set_bits =~= old_next_set_bits.union(
                        BitmapView::range_set(0int, bit_n as int)),
                decreases take - bit_n,
            {
                proof {
                    assert(!chunk_n.bitmap@.set_bits.contains(bit_n as int));
                    assert((bit_n as int) < chunk_n.bitmap@.num_bits);
                }
                let set_result = chunk_n.bitmap.set(bit_n);
                match set_result {
                    Ok(()) => {
                        proof {
                            assert forall|b: int| (bit_n + 1) as int <= b < take as int
                                implies !chunk_n.bitmap@.set_bits.contains(b) by {}
                            lemma_range_set_insert_end(0int, bit_n as int);
                        }
                    },
                    Err(_e) => { proof { assert(false); } },
                }
                bit_n = bit_n + 1;
            }

            self.chunks.insert(next, chunk_n);
            proof {
                let remaining_prime = (remaining - take) as int;
                let cur_prime_idx = (next - entry) as int;
                if remaining_prime > 0 {
                    lemma_seq_sum_from_positive_implies_in_range(
                        phase1b_free_prefixes, cur_prime_idx);
                }
                lemma_lifted_set_bits_alloc_range(
                    pre_inner_chunks, self.chunks@, next as int, 0int, take as int);
                lemma_range_set_union_contiguous(
                    gs, gs + count as int - remaining as int,
                    gs + count as int - remaining as int + take as int);
                committed = committed + take as int;
                if remaining_prime > 0 {
                    let ghost _trig = old_chunks_seq[(next as int + 1)];
                }
            }

            remaining = remaining - take;
            cur = next;
        }

        // remaining == 0 follows from the loop ensures clause.

        // ── Re-establish self.inv() and prove postcondition ──
        proof {
            lemma_capacity_from_depends_only_on_chunks(self@, old(self)@, 0);
            Self::lemma_new_establishes_inv(&*self);
        }

        let global_start = self.chunks[entry].offset + start_bit_in_entry;
        global_start
    }


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
    {
        let ghost old_view = self@;
        let ghost old_chunks_seq = self.chunks@;

        // ── Phase 1: count trailing free bits in entry chunk ──
        let entry_cap = self.chunks[entry].bitmap.number_of_bits();
        let mut trailing_free: usize = 0;
        let mut bit_idx: usize = entry_cap;
        while bit_idx > 0
            invariant_except_break
                trailing_free == entry_cap - bit_idx,
                // all bits in [bit_idx, entry_cap) are free
                forall|b: int| bit_idx <= b < entry_cap as int
                    ==> !self.chunks@[entry as int].bitmap@.set_bits.contains(b),
            invariant
                self.inv(),
                self@ =~= old_view,
                old_view =~= old(self)@,
                self.chunks@ =~= old_chunks_seq,
                entry < self.chunks@.len(),
                entry_cap as int == self.chunks@[entry as int].bitmap@.num_bits,
                0 <= bit_idx <= entry_cap,
            ensures
                self.inv(),
                self@ =~= old_view,
                old_view =~= old(self)@,
                self.chunks@ =~= old_chunks_seq,
                entry < self.chunks@.len(),
                entry_cap as int == self.chunks@[entry as int].bitmap@.num_bits,
                0 <= trailing_free <= entry_cap,
                forall|b: int| (entry_cap - trailing_free) as int <= b < entry_cap as int
                    ==> !self.chunks@[entry as int].bitmap@.set_bits.contains(b),
                // If trailing_free < entry_cap, the bit just below is set
                trailing_free < entry_cap ==>
                    self.chunks@[entry as int].bitmap@.set_bits.contains(
                        (entry_cap - trailing_free - 1) as int),
            decreases bit_idx,
        {
            bit_idx = bit_idx - 1;
            let test_result = self.chunks[entry].bitmap.test(bit_idx);
            match test_result {
                Ok(is_set) => {
                    if is_set {
                        // Found a set bit — stop counting
                        break;
                    }
                    trailing_free = trailing_free + 1;
                },
                Err(e) => {
                    // bit_idx < entry_cap (from loop invariant), so test cannot fail
                    proof {
                        assert(bit_idx < entry_cap);
                        assert(false);
                    }
                    return Err(e);
                },
            }
        }

        if trailing_free == 0 {
            proof {
                lemma_trailing_zero_no_cross_range(
                    old(self)@, self.chunks@, entry as int, count as int);
            }
            return Ok(None);
        }

        // ── Phase 1b: check walk — verify subsequent touching chunks have free heads ──
        let mut need: usize = if count > trailing_free {
            count - trailing_free
        } else {
            0
        };
        let mut last_chunk: usize = entry;
        let ghost mut phase1b_free_prefixes: Seq<int> = Seq::empty();
        let ghost initial_need: int = need as int;

        while need > 0
            invariant
                self.inv(),
                self@ =~= old_view,
                old_view =~= old(self)@,
                self.chunks@ =~= old_chunks_seq,
                entry < self.chunks@.len(),
                last_chunk >= entry,
                last_chunk < self.chunks@.len(),
                trailing_free > 0,
                trailing_free <= entry_cap,
                count > 0,
                // Ghost sequence tracks verified free prefix length per follow-chunk
                phase1b_free_prefixes.len() == (last_chunk - entry) as int,
                forall|i: int| 0 <= i < phase1b_free_prefixes.len() ==> {
                    let k = entry as int + 1 + i;
                    &&& k < self.chunks@.len()
                    &&& 0 < (#[trigger] phase1b_free_prefixes[i]) <= self.chunks@[k].bitmap@.num_bits
                    &&& forall|b: int| 0 <= b < phase1b_free_prefixes[i]
                        ==> !self.chunks@[k].bitmap@.set_bits.contains(b)
                },
                // Each pfp entry either took full capacity or was the final entry
                forall|i: int| 0 <= i < phase1b_free_prefixes.len() ==> (
                    (#[trigger] phase1b_free_prefixes[i]) == self.chunks@[(entry as int + 1 + i)].bitmap@.num_bits
                    || seq_sum_from(phase1b_free_prefixes, i + 1) == 0
                ),
                // Stronger: when need > 0, ALL entries took full capacity
                need > 0 ==> forall|i: int| 0 <= i < phase1b_free_prefixes.len() ==>
                    (#[trigger] phase1b_free_prefixes[i]) == self.chunks@[(entry as int + 1 + i)].bitmap@.num_bits,
                // Sum invariant: need + sum(pfp) == initial_need
                need as int + seq_sum_from(phase1b_free_prefixes, 0) == initial_need,
                initial_need >= 0,
                seq_sum_from(phase1b_free_prefixes, 0) >= 0,
                // Touching chain: chunks[entry..=last_chunk] are pairwise touching
                forall|i: int| entry as int <= i < last_chunk as int ==>
                    self.chunks@[i].offset as int + self.chunks@[i].bitmap@.num_bits
                        == (#[trigger] self.chunks@[(i + 1) as int]).offset as int,
                // Sum-bounds: entry_end + sum(pfp) <= last_chunk's end
                self.chunks@[entry as int].offset as int + entry_cap as int
                    + seq_sum_from(phase1b_free_prefixes, 0)
                    <= self.chunks@[last_chunk as int].offset as int
                        + self.chunks@[last_chunk as int].bitmap@.num_bits,
                // Tight sum-bounds: when need > 0, sum is exact
                need > 0 ==>
                    self.chunks@[entry as int].offset as int + entry_cap as int
                        + seq_sum_from(phase1b_free_prefixes, 0)
                        == self.chunks@[last_chunk as int].offset as int
                            + self.chunks@[last_chunk as int].bitmap@.num_bits,
                // initial_need tracks count - trailing_free (when loop is active)
                need > 0 ==> count as int > trailing_free as int,
                count as int > trailing_free as int ==>
                    initial_need == count as int - trailing_free as int,
                // entry_cap and trailing_free relate to count
                entry_cap as int == self.chunks@[entry as int].bitmap@.num_bits,
                // trailing_free loop ensures: if trailing_free < entry_cap, bit below is set
                trailing_free < entry_cap ==>
                    self.chunks@[entry as int].bitmap@.set_bits.contains(
                        (entry_cap as int - trailing_free as int - 1)),
            decreases need,
        {
            let n_chunks: usize = self.chunks.len();
            // n_chunks is usize, so last_chunk < n_chunks implies last_chunk + 1 <= n_chunks <= usize::MAX
            let next: usize = last_chunk + 1;
            if next >= n_chunks {
                proof {
                    lemma_walk_fail_gap_case(
                        old(self)@, self.chunks@, entry as int, count as int,
                        trailing_free as int, entry_cap as int, need as int,
                        seq_sum_from(phase1b_free_prefixes, 0), last_chunk as int);
                }
                return Ok(None);
            }
            let last_end = self.chunks[last_chunk].end();
            let next_off = self.chunks[next].offset;
            if last_end != next_off {
                proof {
                    lemma_walk_fail_gap_case(
                        old(self)@, self.chunks@, entry as int, count as int,
                        trailing_free as int, entry_cap as int, need as int,
                        seq_sum_from(phase1b_free_prefixes, 0), last_chunk as int);
                }
                return Ok(None);
            }
            let cap_next = self.chunks[next].bitmap.number_of_bits();
            let take: usize = if need < cap_next { need } else { cap_next };

            // Check that bits [0, take) in chunk[next] are free
            let mut check_bit: usize = 0;
            while check_bit < take
                invariant
                    self.inv(),
                    self@ =~= old_view,
                    old_view =~= old(self)@,
                    self.chunks@ =~= old_chunks_seq,
                    next < self.chunks@.len(),
                    take <= cap_next,
                    cap_next as int == self.chunks@[next as int].bitmap@.num_bits,
                    0 <= check_bit <= take,
                    // All bits [0, check_bit) in chunk[next] are free
                    forall|b: int| 0 <= b < check_bit as int
                        ==> !self.chunks@[next as int].bitmap@.set_bits.contains(b),
                    // Outer loop state (not modified by inner loop)
                    entry < self.chunks@.len(),
                    last_chunk >= entry,
                    last_chunk < self.chunks@.len(),
                    need > 0,
                    take <= need,
                    count > 0,
                    trailing_free > 0,
                    trailing_free <= entry_cap,
                    entry_cap as int == self.chunks@[entry as int].bitmap@.num_bits,
                    initial_need == count as int - trailing_free as int,
                    need as int + seq_sum_from(phase1b_free_prefixes, 0) == initial_need,
                    seq_sum_from(phase1b_free_prefixes, 0) >= 0,
                    count as int > trailing_free as int,
                    // Tight sum-bounds
                    self.chunks@[entry as int].offset as int + entry_cap as int
                        + seq_sum_from(phase1b_free_prefixes, 0)
                        == self.chunks@[last_chunk as int].offset as int
                            + self.chunks@[last_chunk as int].bitmap@.num_bits,
                    // Touching: chunks[next].offset == chunk_end(last_chunk)
                    self.chunks@[last_chunk as int].offset as int
                        + self.chunks@[last_chunk as int].bitmap@.num_bits
                        == self.chunks@[next as int].offset as int,
                    // Trailing-free loop ensures
                    trailing_free < entry_cap ==>
                        self.chunks@[entry as int].bitmap@.set_bits.contains(
                            (entry_cap as int - trailing_free as int - 1)),
                decreases take - check_bit,
            {
                let test_result = self.chunks[next].bitmap.test(check_bit);
                match test_result {
                    Ok(is_set) => {
                        if is_set {
                            // Case (c): set bit in follow chunk blocks cross-chunk range
                            proof {
                                lemma_walk_fail_set_bit_case(
                                    old(self)@, self.chunks@, entry as int, count as int,
                                    trailing_free as int, entry_cap as int, need as int,
                                    seq_sum_from(phase1b_free_prefixes, 0), last_chunk as int,
                                    next as int, check_bit as int);
                            }
                            return Ok(None);
                        }
                    },
                    Err(e) => {
                        proof {
                            assert(check_bit < cap_next);
                            assert(false);
                        }
                        return Err(e);
                    },
                }
                check_bit = check_bit + 1;
            }

            proof {
                let ghost old_pfp = phase1b_free_prefixes;
                phase1b_free_prefixes = phase1b_free_prefixes.push(take as int);
                lemma_seq_sum_from_push(old_pfp, take as int, 0);
            }

            need = need - take;
            last_chunk = next;
        }

        // ── Phase 2: commit — set bits in entry chunk ──
        let _take_from_entry: usize = if count < trailing_free { count } else { trailing_free };
        let _start_bit_in_entry: usize = entry_cap - trailing_free;

        // ── Prove has_contiguous_free_range (Item 1) BEFORE mutations ──
        // At this point self@ =~= old(self)@ (no mutations yet).
        proof {
            lemma_cross_chunk_range_is_free(
                old(self)@, self.chunks@, phase1b_free_prefixes,
                entry as int, last_chunk as int, entry_cap as int,
                trailing_free as int, count as int);
        }


        // ── Call commit helper ──
        let global_start = self.commit_cross_chunk_alloc(
            entry, count, trailing_free, entry_cap, last_chunk,
            Ghost(phase1b_free_prefixes), Ghost(old_chunks_seq));

        Ok(Some(global_start))
    }


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
    {
        if count == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "count must be non-zero"));
        }

        let n = self.chunks.len();
        let start_hint = self.next_chunk_hint;
        let ghost old_view = self@;

        // Pass 1: single-chunk allocation via circular iteration
        let mut idx: usize = start_hint;
        let mut visited: usize = 0;
        let ghost mut has_wrapped: bool = false;
        while visited < n
            invariant
                self.inv(),
                self@ =~= old_view,
                old_view =~= old(self)@,
                n == self.chunks@.len(),
                n > 0,
                count > 0,
                0 <= visited <= n,
                0 <= idx < n,
                0 <= start_hint < n,
                // idx position tracking
                !has_wrapped ==> idx as int == start_hint as int + visited as int,
                has_wrapped ==> idx as int == start_hint as int + visited as int - n as int,
                !has_wrapped ==> idx >= start_hint,
                has_wrapped ==> idx <= start_hint,
                // Negative single-chunk facts for visited chunks
                !has_wrapped ==>
                    forall|k: int| start_hint as int <= k < idx as int
                        ==> !old_view.has_single_chunk_free_range(k, count as int),
                has_wrapped ==> {
                    &&& forall|k: int| start_hint as int <= k < n as int
                        ==> !old_view.has_single_chunk_free_range(k, count as int)
                    &&& forall|k: int| 0 <= k < idx as int
                        ==> !old_view.has_single_chunk_free_range(k, count as int)
                },
            decreases n - visited,
        {
            let ghost old_chunks_seq = self.chunks@;

            let mut chunk = self.chunks.remove(idx);
            let ghost pre_alloc_bitmap_view = chunk.bitmap@;

            let num_bits = chunk.bitmap.number_of_bits();

            if count <= num_bits {
                match chunk.bitmap.alloc_range(count) {
                    Ok(local) => {
                        let result_offset = chunk.offset + local;
                        let ghost new_chunk = chunk;
                        self.chunks.insert(idx, chunk);
                        self.next_chunk_hint = idx;

                        proof {
                            lemma_seq_remove_insert_is_update(old_chunks_seq, idx as int, new_chunk);
                            lemma_single_chunk_alloc_ok(
                                &*self, old_chunks_seq, old_view, pre_alloc_bitmap_view,
                                idx as int, local as int, count as int);
                        }

                        return Ok(result_offset);
                    },
                    Err(_e) => {
                        self.chunks.insert(idx, chunk);
                        proof {
                            lemma_seq_remove_insert_identity(old_chunks_seq, idx as int);
                            lemma_bitmap_no_range_implies_no_single_chunk(
                                old_view, self.chunks@, idx as int, count as int);
                        }
                    },
                }
            } else {
                self.chunks.insert(idx, chunk);
                proof {
                    lemma_seq_remove_insert_identity(old_chunks_seq, idx as int);
                    lemma_too_large_implies_no_single_chunk(
                        old_view, idx as int, count as int);
                }
            }

            proof {
                if idx as int + 1 >= n as int { has_wrapped = true; }
            }
            idx = if idx + 1 >= n { 0 } else { idx + 1 };
            visited = visited + 1;
        }

        proof {
            lemma_circular_scan_exhausts(
                has_wrapped, idx as int, start_hint as int,
                visited as int, n as int);
        }

        // Pass 2: cross-chunk allocation
        let mut entry: usize = 0;
        while entry < n
            invariant
                self.inv(),
                self@ =~= old_view,
                old_view =~= old(self)@,
                n == self.chunks@.len(),
                n > 0,
                count > 0,
                0 <= entry <= n,
                // Negative cross-chunk facts for visited entries
                forall|k: int| 0 <= k < entry as int
                    ==> !old_view.has_cross_chunk_free_range_from(k, count as int),
                // Carry forward: all chunks have no single-chunk range
                forall|k: int| 0 <= k < n as int
                    ==> !old_view.has_single_chunk_free_range(k, count as int),
            decreases n - entry,
        {
            match self.try_alloc_cross_chunk_from(entry, count) {
                Ok(opt) => {
                    match opt {
                        Some(global_start) => {
                            self.next_chunk_hint = entry;

                            proof {
                                assert((self.next_chunk_hint as int) < self.chunks@.len());
                                Self::lemma_new_establishes_inv(&*self);
                            }

                            return Ok(global_start);
                        },
                        None => {
                            // Ok(None) ensures: !has_cross_chunk_free_range_from(entry, count)
                            // This extends the invariant for the next iteration.
                        },
                    }
                },
                Err(e) => {
                    proof {
                        // try_alloc_cross_chunk_from ensures !(result matches Err(_)),
                        // so this branch is unreachable.
                        assert(false);
                    }
                    return Err(e);
                },
            }
            entry = entry + 1;
        }

        proof {
            lemma_exhaustive_search_no_free_range(old_view, count as int);
        }
        Err(Error::new(
            ErrorCode::OutOfMemory,
            "no contiguous free range of the requested size (tried single-chunk and cross-chunk \
             passes)",
        ))
    }

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
    {
        match self.find_chunk_index(index) {
            Some(ci) => {
                // Read offset before mutation; compute local index
                let chunk_offset = self.chunks[ci].offset;
                let local = index - chunk_offset;

                // Snapshot old state while self.inv() still holds
                let ghost old_chunks_seq = self.chunks@;
                proof {
                    assert(old_chunks_seq[ci as int].bitmap.inv());
                    assert((local as int) < old_chunks_seq[ci as int].bitmap@.num_bits);
                }

                // Remove, mutate, re-insert
                let mut chunk = self.chunks.remove(ci);
                let result = chunk.bitmap.set(local);
                let ghost new_chunk = chunk;
                self.chunks.insert(ci, chunk);

                proof {
                    lemma_seq_remove_insert_is_update(old_chunks_seq, ci as int, new_chunk);
                }

                match result {
                    Ok(()) => {
                        proof {
                            lemma_lifted_set_bits_set(old_chunks_seq, self.chunks@, ci as int, local as int);
                            lemma_lifted_set_bits_not_contains_chunk(old_chunks_seq, 0, ci as int, local as int);
                            lemma_inv_after_chunk_update(&*self, old_chunks_seq, old(self)@, ci as int);
                        }
                        Ok(())
                    },
                    Err(e) => {
                        proof {
                            assert(self.chunks@ =~= old_chunks_seq);
                            Self::lemma_new_establishes_inv(&*self);
                            lemma_lifted_set_bits_contains_chunk(old_chunks_seq, 0, ci as int, local as int);
                        }
                        Err(e)
                    },
                }
            },
            None => Err(Error::new(ErrorCode::InvalidArgument, "no chunk covers index")),
        }
    }

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
    {
        match self.find_chunk_index(index) {
            Some(ci) => {
                let chunk_offset = self.chunks[ci].offset;
                let local = index - chunk_offset;

                let ghost old_chunks_seq = self.chunks@;
                proof {
                    assert(old_chunks_seq[ci as int].bitmap.inv());
                    assert((local as int) < old_chunks_seq[ci as int].bitmap@.num_bits);
                }

                let mut chunk = self.chunks.remove(ci);
                let result = chunk.bitmap.clear(local);
                let ghost new_chunk = chunk;
                self.chunks.insert(ci, chunk);

                proof {
                    lemma_seq_remove_insert_is_update(old_chunks_seq, ci as int, new_chunk);
                }

                match result {
                    Ok(()) => {
                        proof {
                            lemma_lifted_set_bits_clear(old_chunks_seq, self.chunks@, ci as int, local as int);
                            lemma_lifted_set_bits_contains_chunk(old_chunks_seq, 0, ci as int, local as int);
                            lemma_inv_after_chunk_update(&*self, old_chunks_seq, old(self)@, ci as int);
                        }
                        Ok(())
                    },
                    Err(e) => {
                        proof {
                            assert(self.chunks@ =~= old_chunks_seq);
                            Self::lemma_new_establishes_inv(&*self);
                            lemma_lifted_set_bits_not_contains_chunk(old_chunks_seq, 0, ci as int, local as int);
                        }
                        Err(e)
                    },
                }
            },
            None => Err(Error::new(ErrorCode::BadAddress, "bit is not set")),
        }
    }

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
    {
        let n = self.chunks.len();
        let mut lo: usize = 0;
        let mut hi: usize = n;
        while lo < hi
            invariant
                0 <= lo <= hi <= n,
                n == self.chunks@.len(),
                self.inv(),
                forall|k: int| 0 <= k < lo as int
                    ==> (#[trigger] self.chunks@[k]).offset as int <= index as int,
                forall|k: int| hi as int <= k < n as int
                    ==> (#[trigger] self.chunks@[k]).offset as int > index as int,
            decreases hi - lo,
        {
            let mid = lo + (hi - lo) / 2;
            if self.chunks[mid].offset <= index {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let idx = lo;
        if idx == 0 {
            proof {
                lemma_no_chunk_covers_all_above(
                    self.chunks@, self@.chunks, index as int);
            }
            return None;
        }
        let candidate = idx - 1;
        if self.chunks[candidate].covers(index) {
            Some(candidate)
        } else {
            proof {
                lemma_no_chunk_covers_with_gap(
                    self.chunks@, self@.chunks, candidate as int, index as int);
            }
            None
        }
    }
}

} // verus!

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
    fn alloc_range_stitches_across_touching_chunks() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(64)), (64, make_bitmap(64))]).unwrap();
        let start = s.alloc_range(65).expect("touching chunks should stitch");
        assert_eq!(start, 0);
        for i in 0..65 {
            assert!(s.test(i).unwrap());
        }
        assert!(!s.test(65).unwrap());
    }

    #[test]
    fn alloc_range_refuses_to_stitch_across_gap() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(64)), (128, make_bitmap(64))]).unwrap();
        let err = s.alloc_range(65).expect_err("gap breaks contiguity");
        assert_eq!(err.code, ErrorCode::OutOfMemory);
    }

    #[test]
    fn alloc_range_stitches_across_three_chunks() {
        let mut s = SparseBitmap::new(vec![
            (0, make_bitmap(8)),
            (8, make_bitmap(8)),
            (16, make_bitmap(8)),
        ])
        .unwrap();
        let start = s.alloc_range(20).expect("three-chunk stitch");
        assert_eq!(start, 0);
        for i in 0..20 {
            assert!(s.test(i).unwrap());
        }
    }

    #[test]
    fn alloc_range_stitch_requires_free_tail_of_entry() {
        let mut s = SparseBitmap::new(vec![(0, make_bitmap(8)), (8, make_bitmap(8))]).unwrap();
        s.set(7).unwrap();
        let err = s.alloc_range(9).expect_err("no valid entry tail");
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
