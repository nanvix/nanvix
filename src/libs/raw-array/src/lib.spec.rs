// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// RawArray - Specifications
//
// This file contains specification functions, invariants, and View trait for RawArray.

verus! {

//==================================================================================================
// Zero Initialization Specification
//==================================================================================================

/// Predicate: Specifies that a value is zero/default for its type.
pub uninterp spec fn is_zero<T>(value: T) -> bool;

/// Axiom: For u8, zero means the value is 0.
pub axiom fn axiom_u8_zero_is_0(t: u8)
    requires is_zero(t)
    ensures t == 0;

/// Axiom: For usize, zero means the value is 0.
pub axiom fn axiom_usize_zero_is_0(t: usize)
    requires is_zero(t)
    ensures t == 0;

//==================================================================================================
// RawArrayView - Abstract Specification Model
//==================================================================================================

/// Abstract view of a RawArray as a sequence (ghost/spec-level representation).
pub struct RawArrayView<T> {
    pub contents: Seq<T>,
}

impl<T> RawArrayView<T> {
    /// Returns the length of the array view.
    pub open spec fn len(&self) -> nat {
        self.contents.len()
    }

    /// Returns the element at index i.
    pub open spec fn index(&self, i: int) -> T
        recommends
            0 <= i < self.len() as int,
    {
        self.contents[i]
    }

    /// Returns a new view with the element at index i updated to value.
    pub open spec fn update(&self, i: int, value: T) -> RawArrayView<T>
        recommends
            0 <= i < self.len() as int,
    {
        RawArrayView { contents: self.contents.update(i, value) }
    }

    /// Two views are equal if they have the same contents.
    pub open spec fn spec_eq(&self, other: &RawArrayView<T>) -> bool {
        self.contents =~= other.contents
    }
}

//==================================================================================================
// RawArray Invariant
//==================================================================================================

/// The invariant that all RawArray instances must satisfy.
pub open spec fn raw_array_inv<T>(len: nat) -> bool {
    len > 0 && len < i32::MAX as nat
}

} // verus!

verus! {

//==================================================================================================
// RawArray View Implementation
//==================================================================================================

impl<T> View for RawArray<T> {
    type V = Seq<T>;

    /// Abstract view of the array as a sequence (uninterpreted).
    #[verifier::external_body]
    open spec fn view(&self) -> Seq<T> {
        unimplemented!()
    }
}

//==================================================================================================
// RawArray Specification Functions
//==================================================================================================

impl<T> RawArray<T> {
    /// Returns the spec-level length of the array.
    pub open spec fn spec_len(&self) -> nat {
        self@.len()
    }

    /// Returns true if index i is within bounds.
    pub open spec fn in_bounds(&self, i: int) -> bool {
        0 <= i < self.spec_len() as int
    }

    /// Invariant: length is positive and bounded.
    pub open spec fn inv(&self) -> bool {
        self@.len() > 0 && self@.len() < i32::MAX as nat
    }
}

} // verus!
