verus! {

//==================================================================================================
// Helper spec functions
//==================================================================================================

/// Sum of elements in `s` from index `from` (inclusive) to the end.
pub open spec fn seq_sum_from(s: Seq<int>, from: int) -> int
    decreases (if from < s.len() { s.len() - from } else { 0 }),
{
    if from >= s.len() {
        0
    } else {
        s[from] + seq_sum_from(s, from + 1)
    }
}

//==================================================================================================
// SparseBitmapView - Abstract Specification Model
//==================================================================================================

/// Abstract view of a SparseBitmap.
///
/// Two fields capture the full caller-observable state:
/// - `chunks`: the fixed shape — which global index ranges are covered.
/// - `set_bits`: the mutable state — which covered indices have their bit set.
#[verifier::ext_equal]
pub struct SparseBitmapView {
    /// Sorted, non-overlapping chunk descriptors.
    /// Each element is (offset, num_bits) describing the half-open range
    /// [offset, offset + num_bits).
    pub chunks: Seq<(int, int)>,

    /// Set of global indices where bits are set.
    /// Always a subset of covered_indices().
    pub set_bits: Set<int>,
}

impl SparseBitmapView {
    /// Well-formedness invariant for the abstract view.
    pub open spec fn wf(&self) -> bool {
        // At least one chunk
        &&& self.chunks.len() > 0

        // All chunk offsets are non-negative
        &&& forall|i: int| #![auto] 0 <= i < self.chunks.len()
            ==> self.chunks[i].0 >= 0

        // All chunk sizes are positive
        &&& forall|i: int| #![auto] 0 <= i < self.chunks.len()
            ==> self.chunks[i].1 > 0

        // Chunk ranges do not overflow usize (representability)
        &&& forall|i: int| #![auto] 0 <= i < self.chunks.len()
            ==> self.chunks[i].0 + self.chunks[i].1 <= usize::MAX as int

        // Chunks are sorted and non-overlapping (touching permitted)
        &&& forall|i: int, j: int| #![auto] 0 <= i < j < self.chunks.len()
            ==> self.chunks[i].0 + self.chunks[i].1 <= self.chunks[j].0

        // set_bits only contains covered indices
        &&& forall|idx: int| #![auto] self.set_bits.contains(idx)
            ==> self.is_covered(idx)

        // set_bits is finite (required for len()/cardinality)
        &&& self.set_bits.finite()
    }

    /// Whether a global index is covered by some chunk.
    pub open spec fn is_covered(&self, index: int) -> bool {
        exists|i: int|
            #![trigger self.chunks[i]]
            0 <= i < self.chunks.len()
            && self.chunks[i].0 <= index
            && index < self.chunks[i].0 + self.chunks[i].1
    }

    /// The set of all globally covered indices.
    pub open spec fn covered_set(&self) -> Set<int> {
        Set::new(|idx: int| self.is_covered(idx))
    }

    /// Total number of bits across all chunks.
    pub open spec fn capacity(&self) -> int {
        self.capacity_from(0)
    }

    /// Capacity starting from chunk index `from` (recursive helper).
    pub open spec fn capacity_from(&self, from: int) -> int
        decreases self.chunks.len() - from
    {
        if from >= self.chunks.len() {
            0
        } else {
            self.chunks[from].1 + self.capacity_from(from + 1)
        }
    }

    /// Number of chunks.
    pub open spec fn chunk_count(&self) -> int {
        self.chunks.len() as int
    }

    /// Whether a specific bit is set.
    pub open spec fn is_bit_set(&self, index: int) -> bool {
        self.set_bits.contains(index)
    }

    /// Whether two adjacent chunks touch (chunk i's end == chunk j's start).
    pub open spec fn chunks_touch(&self, i: int, j: int) -> bool {
        &&& 0 <= i < self.chunks.len()
        &&& 0 <= j < self.chunks.len()
        &&& self.chunks[i].0 + self.chunks[i].1 == self.chunks[j].0
    }

    /// Whether the range [start, start + count) is entirely covered and free.
    pub open spec fn has_contiguous_free_range(&self, start: int, count: int) -> bool {
        &&& count > 0
        &&& forall|k: int| start <= k < start + count ==> self.is_covered(k)
        &&& forall|k: int| start <= k < start + count ==> !self.set_bits.contains(k)
    }

    /// Whether there exists some contiguous free range of the given size.
    pub open spec fn exists_contiguous_free_range(&self, count: int) -> bool {
        exists|start: int|
            #![trigger self.has_contiguous_free_range(start, count)]
            self.has_contiguous_free_range(start, count)
    }

    /// Whether chunk `ci` contains a single-chunk contiguous free range of `count` bits.
    /// The range [start, start+count) fits entirely within chunk ci.
    pub open spec fn has_single_chunk_free_range(&self, ci: int, count: int) -> bool {
        &&& 0 <= ci < self.chunks.len()
        &&& count > 0
        &&& exists|start: int| {
            &&& self.chunks[ci].0 <= start
            &&& start + count <= self.chunks[ci].0 + self.chunks[ci].1
            &&& self.has_contiguous_free_range(start, count)
        }
    }

    /// Whether there is a cross-chunk contiguous free range of `count` bits
    /// starting in chunk `ci` (the range extends past chunk ci's end).
    pub open spec fn has_cross_chunk_free_range_from(&self, ci: int, count: int) -> bool {
        &&& 0 <= ci < self.chunks.len()
        &&& count > 0
        &&& exists|start: int| {
            &&& self.chunks[ci].0 <= start
            &&& start < self.chunks[ci].0 + self.chunks[ci].1
            &&& start + count > self.chunks[ci].0 + self.chunks[ci].1
            &&& self.has_contiguous_free_range(start, count)
        }
    }

    /// Usage: total number of set bits.
    pub open spec fn usage(&self) -> int {
        self.set_bits.len() as int
    }

    /// Whether all bits are set (full).
    pub open spec fn is_full(&self) -> bool {
        forall|idx: int| #![auto] self.is_covered(idx) ==> self.set_bits.contains(idx)
    }

    /// Whether no bits are set (empty).
    pub open spec fn is_empty(&self) -> bool {
        self.set_bits =~= Set::<int>::empty()
    }

    /// Chunk offset accessor.
    pub open spec fn chunk_offset(&self, i: int) -> int {
        self.chunks[i].0
    }

    /// Chunk size accessor.
    pub open spec fn chunk_size(&self, i: int) -> int {
        self.chunks[i].1
    }

    /// Chunk end (exclusive upper bound).
    pub open spec fn chunk_end(&self, i: int) -> int {
        self.chunks[i].0 + self.chunks[i].1
    }
}

//==================================================================================================
// Chunk Specification Functions
//==================================================================================================

impl Chunk {
    /// Spec-level offset accessor.
    pub closed spec fn spec_offset(&self) -> int {
        self.offset as int
    }

    /// Spec-level num_bits accessor.
    pub closed spec fn spec_num_bits(&self) -> int {
        self.bitmap@.num_bits
    }

    /// Spec-level end accessor.
    pub open spec fn spec_end(&self) -> int {
        self.spec_offset() + self.spec_num_bits()
    }

    /// Whether the chunk covers `index` at spec level.
    pub open spec fn spec_covers(&self, index: int) -> bool {
        self.spec_offset() <= index && index < self.spec_end()
    }

    /// Chunk-level invariant: the underlying bitmap is valid and range doesn't overflow.
    pub closed spec fn inv(&self) -> bool {
        &&& self.bitmap.inv()
        &&& self.spec_offset() + self.spec_num_bits() <= usize::MAX as int
    }
}

//==================================================================================================
// View Implementation for SparseBitmap
//==================================================================================================

/// Lift per-chunk set_bits into the global index space, unioning
/// from chunk index `idx` onwards.
pub closed spec fn lifted_set_bits(chunks: Seq<Chunk>, idx: int) -> Set<int>
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
        Set::empty()
    } else {
        let offset = chunks[idx].offset as int;
        let bv = chunks[idx].bitmap@;
        let shifted = Set::new(|g: int|
            bv.set_bits.contains(g - offset)
        );
        shifted.union(lifted_set_bits(chunks, idx + 1))
    }
}

impl View for SparseBitmap {
    type V = SparseBitmapView;

    closed spec fn view(&self) -> SparseBitmapView {
        SparseBitmapView {
            chunks: Seq::new(self.chunks@.len(), |i: int| (
                self.chunks@[i].offset as int,
                self.chunks@[i].bitmap@.num_bits,
            )),
            set_bits: lifted_set_bits(self.chunks@, 0),
        }
    }
}

//==================================================================================================
// SparseBitmap Invariant
//==================================================================================================

impl SparseBitmap {
    /// Public invariant: view is well-formed and concrete state is consistent.
    pub open spec fn inv(&self) -> bool {
        &&& self@.wf()
        &&& self.internal_inv()
    }

    /// Internal invariant connecting concrete state to abstract view.
    pub closed spec fn internal_inv(&self) -> bool {
        // The chunks vector is non-empty
        &&& self.chunks@.len() > 0

        // All per-chunk bitmaps satisfy their own invariant
        &&& forall|i: int| 0 <= i < self.chunks@.len()
            ==> (#[trigger] self.chunks@[i]).bitmap.inv()

        // Per-chunk bitmap sizes are positive (from Bitmap::internal_inv, opaque cross-crate)
        &&& forall|i: int| #![auto] 0 <= i < self.chunks@.len()
            ==> self.chunks@[i].bitmap@.num_bits > 0

        // Per-chunk set_bits are finite (from Bitmap::internal_inv, opaque cross-crate)
        &&& forall|i: int| #![auto] 0 <= i < self.chunks@.len()
            ==> self.chunks@[i].bitmap@.set_bits.finite()

        // Chunk ranges do not overflow usize (from constructor validation)
        &&& forall|i: int| #![auto] 0 <= i < self.chunks@.len()
            ==> self.chunks@[i].offset as int + self.chunks@[i].bitmap@.num_bits
                <= usize::MAX as int

        // Chunks are sorted and non-overlapping (from constructor sort + overlap check)
        &&& forall|i: int, j: int| #![auto] 0 <= i < j < self.chunks@.len()
            ==> self.chunks@[i].offset as int + self.chunks@[i].bitmap@.num_bits
                <= self.chunks@[j].offset as int

        // Cached capacity matches the sum of chunk sizes
        &&& self.capacity_bits as int == self@.capacity()

        // next_chunk_hint is a valid index into the chunks vector
        &&& (self.next_chunk_hint as int) < self.chunks@.len()

        // The view's chunks sequence matches the concrete chunks
        &&& self@.chunks.len() == self.chunks@.len()
        &&& forall|i: int| #![auto] 0 <= i < self.chunks@.len() ==> {
            &&& self@.chunks[i].0 == self.chunks@[i].offset as int
            &&& self@.chunks[i].1 == self.chunks@[i].bitmap@.num_bits
        }
    }
}

//==================================================================================================
// Input Sequence Helpers (for new() specification)
//==================================================================================================

/// Total capacity from an input sequence of (offset, Bitmap) pairs.
/// Sort-invariant: the sum does not depend on element order.
pub open spec fn input_capacity(input: Seq<(usize, Bitmap)>, idx: int) -> int
    decreases input.len() - idx
{
    if idx >= input.len() {
        0
    } else {
        input[idx].1@.num_bits + input_capacity(input, idx + 1)
    }
}

/// Lifted set_bits from an input sequence of (offset, Bitmap) pairs.
/// Each bitmap's set_bits are shifted by its associated offset.
/// Sort-invariant: the union does not depend on element order.
pub open spec fn input_lifted_set_bits(input: Seq<(usize, Bitmap)>, idx: int) -> Set<int>
    decreases input.len() - idx
{
    if idx >= input.len() {
        Set::empty()
    } else {
        let offset = input[idx].0 as int;
        let bv = input[idx].1@;
        let shifted = Set::new(|g: int| bv.set_bits.contains(g - offset));
        shifted.union(input_lifted_set_bits(input, idx + 1))
    }
}

/// Total capacity from a sequence of Chunk structs (sum of bitmap sizes).
pub closed spec fn chunk_seq_capacity(chunks: Seq<Chunk>, idx: int) -> int
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
        0
    } else {
        chunks[idx].bitmap@.num_bits + chunk_seq_capacity(chunks, idx + 1)
    }
}

} // verus!
