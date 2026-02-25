// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Bitmap - Specifications
//
// This file contains specification functions, BitmapView, and View trait for Bitmap.
// Uses Set<int> as the primary abstraction (no Seq<bool>).

verus! {

//==================================================================================================
// BitmapView - Abstract Specification Model (Set-based)
//==================================================================================================

/// A view of the Bitmap as a set of indices where bits are set.
/// This avoids expensive `Seq<bool>` which triggers `axiom_seq_new_index`.
#[verifier::ext_equal]
pub struct BitmapView {
    /// Number of bits in the bitmap.
    pub num_bits: int,
    /// Set of indices where bits are set (0-indexed).
    /// Invariant: all elements are in [0, num_bits).
    pub set_bits: Set<int>,
}

impl BitmapView {
    /// Returns the number of bits in the bitmap view.
    pub open spec fn number_of_bits(&self) -> int {
        self.num_bits
    }

    /// Returns the usage (count of set bits) in the bitmap view.
    /// Requires set_bits to be finite (enforced by Bitmap::inv()).
    pub open spec fn usage(&self) -> int {
        self.set_bits.len() as int
    }

    /// Returns the count of free (unset) bits.
    pub open spec fn count_free(&self) -> int {
        self.number_of_bits() - self.usage()
    }

    /// Returns true if there exists at least one unset bit.
    pub open spec fn has_free_bit(&self) -> bool {
        exists|i: int| 0 <= i < self.number_of_bits() && !self.set_bits.contains(i)
    }

    /// Returns true if the bitmap is full (all bits set).
    pub open spec fn is_full(&self) -> bool {
        forall|i: int| 0 <= i < self.num_bits ==> self.set_bits.contains(i)
    }

    /// Returns true if the bitmap is empty (no bits set).
    pub open spec fn is_empty(&self) -> bool {
        self.set_bits =~= Set::empty()
    }

    /// Returns true if a specific bit is set.
    pub open spec fn is_bit_set(&self, index: int) -> bool {
        self.set_bits.contains(index)
    }

    /// Helper: Create a set of indices in range [start, end).
    pub open spec fn range_set(start: int, end: int) -> Set<int> {
        Set::new(|i: int| start <= i < end)
    }

    /// Well-formedness: set_bits only contains valid indices.
    pub open spec fn wf(&self) -> bool {
        forall|i: int| self.set_bits.contains(i) ==> 0 <= i < self.num_bits
    }
}

//==================================================================================================
// View Implementation for Bitmap
//==================================================================================================

impl View for Bitmap {
    type V = BitmapView;

    closed spec fn view(&self) -> BitmapView {
        BitmapView {
            num_bits: self.number_of_bits as int,
            set_bits: Set::new(|i: int| 0 <= i < self.number_of_bits as int && Self::bit_at(self.bits@, i)),
        }
    }
}

//==================================================================================================
// Bitmap Specification Functions
//==================================================================================================

impl Bitmap {
    /// Helper spec function: get the bit value at a specific index from raw bytes.
    pub open spec fn bit_at(bytes: Seq<u8>, bit_index: int) -> bool {
        let word: int = bit_index / (u8::BITS as int);
        let bit: int = bit_index % (u8::BITS as int);
        if 0 <= bit_index && word < bytes.len() {
            (bytes[word] & (1u8 << bit)) != 0
        } else {
            false
        }
    }

    /// Helper spec function: check if a bit at the given bit index is set.
    pub open spec fn is_bit_set(&self, bit_index: int) -> bool {
        &&& 0 <= bit_index < self@.number_of_bits()
        &&& self@.set_bits.contains(bit_index)
    }

    /// Helper spec function: check if all bits in range [start, end) are set.
    pub open spec fn all_bits_set_in_range(&self, start: int, end: int) -> bool {
        forall|i: int| start <= i < end ==> self.is_bit_set(i)
    }

    /// Helper spec function: check if all bits in range [start, end) are not set.
    pub open spec fn all_bits_unset_in_range(&self, start: int, end: int) -> bool {
        forall|i: int| start <= i < end ==> !self.is_bit_set(i)
    }

    /// Helper spec function: check if there exists a contiguous range of n free bits starting at start.
    pub open spec fn has_free_range_at(&self, start: int, n: int) -> bool {
        &&& 0 <= start
        &&& start + n <= self@.number_of_bits()
        &&& self.all_bits_unset_in_range(start, start + n)
    }

    /// Helper spec function: check if there exists a contiguous range of n free bits.
    pub open spec fn exists_contiguous_free_range(&self, n: int) -> bool {
        exists|start: int| #![trigger self.has_free_range_at(start, n)]
            self.has_free_range_at(start, n)
    }

    /// Invariant: the bitmap's state is well-formed.
    pub closed spec fn inv(&self) -> bool {
        &&& self@.number_of_bits() > 0
        &&& self@.number_of_bits() == self.bits@.len() * (u8::BITS as int)
        &&& self@.number_of_bits() < u32::MAX as int
        &&& self@.wf()  // set_bits only contains valid indices
        &&& self@.set_bits.finite()  // set_bits is finite (required for len())
        &&& self@.usage() <= self@.number_of_bits()
        &&& self.number_of_bits as int == self@.number_of_bits()
        &&& self.usage as int == self@.usage()
    }
}

} // verus!
