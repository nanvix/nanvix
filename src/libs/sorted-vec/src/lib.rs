// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]
#![cfg_attr(verus_keep_ghost, feature(allocator_api))]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::alloc::vec::Vec;
use ::vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use ::vstd::{
    laws_cmp::{
        obeys_cmp_ord,
        obeys_cmp_partial_ord,
        obeys_cmp_spec,
        obeys_partial_cmp_spec_properties,
    },
    laws_eq::obeys_eq_spec_properties,
    std_specs::{
        cmp::*,
        convert::FromSpecImpl,
    },
};

//==================================================================================================
// Include specifications.
#[cfg(verus_keep_ghost)]
include!("lib.spec.rs");

// Include proofs.
#[cfg(verus_keep_ghost)]
include!("lib.proof.rs");

// Structures
//==================================================================================================

/// Wraps `mem::replace`, hiding the reborrowing (`&mut Vec<T>` → `&mut T`)
/// that Verus cannot verify.
#[inline]
#[allow(clippy::ptr_arg)]
#[verus_verify(external_body)]
#[verus_spec(prev =>
    requires
        index < old(v)@.len()
    ensures
        prev == old(v)@[index as int],
        v@ == old(v)@.update(index as int, value),
        v@.len() == old(v)@.len()
)]
fn vec_replace<T>(v: &mut Vec<T>, index: usize, value: T) -> T {
    ::core::mem::replace(&mut v[index], value)
}

/// Wraps `sort_unstable`, hiding the `DerefMut` coercion
/// (`&mut Vec<T>` → `&mut [T]`) that Verus cannot verify.
#[inline]
#[allow(clippy::ptr_arg)]
#[verus_verify(external_body)]
#[verus_spec(ensures
        v@.len() == old(v)@.len(),
        forall|val: T| v@.contains(val) <==> old(v)@.contains(val),
        forall|i: int, j: int| #![trigger v@[i], v@[j]]
            0 <= i < j < v@.len() ==> !(v@[j].cmp_spec(&v@[i]) is Less)
)]
fn vec_sort_unstable<T: Ord>(v: &mut Vec<T>) {
    v.sort_unstable();
}

///
/// # Description
///
/// A sorted vector that maintains its elements in ascending order and provides efficient
/// lookup via binary search.
///
/// Elements must implement [`Ord`] for sorting and searching. The vector does not allow
/// duplicate elements; inserting a value that already exists replaces the old entry.
///
#[cfg_attr(not(verus_keep_ghost), derive(Debug, Clone))]
#[verus_verify]
pub struct SortedVec<T: Ord> {
    /// Underlying storage.
    inner: Vec<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
impl<T: Ord> SortedVec<T> {
    ///
    /// # Description
    ///
    /// Creates an empty [`SortedVec`].
    ///
    /// # Returns
    ///
    /// An empty sorted vector.
    ///
    #[verus_spec(result =>
        ensures
            result.inv(),
            result@ == Seq::<T>::empty()
    )]
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    ///
    /// # Description
    ///
    /// Creates an empty [`SortedVec`] with the specified capacity.
    ///
    /// # Parameters
    ///
    /// - `capacity`: The number of elements the vector can hold without reallocating.
    ///
    /// # Returns
    ///
    /// An empty sorted vector with the given capacity.
    ///
    #[verus_spec(result =>
        ensures
            result.inv(),
            result@ == Seq::<T>::empty()
    )]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the number of elements in the sorted vector.
    ///
    /// # Returns
    ///
    /// The number of elements.
    ///
    #[verus_spec(result =>
        ensures
            result as int == self@.len()
    )]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    ///
    /// # Description
    ///
    /// Returns `true` if the sorted vector contains no elements.
    ///
    /// # Returns
    ///
    /// `true` if empty, `false` otherwise.
    ///
    #[verus_spec(result =>
        ensures
            result <==> self@.len() == 0
    )]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    ///
    /// # Description
    ///
    /// Returns the capacity of the sorted vector.
    ///
    /// # Returns
    ///
    /// The number of elements the vector can hold without reallocating.
    ///
    #[verus_spec(result =>
        ensures
            result as int >= self@.len()
    )]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    ///
    /// # Description
    ///
    /// Clears all elements from the sorted vector.
    ///
    #[verus_spec(requires
            old(self).inv()
        ensures
            self.inv(),
            self@ == Seq::<T>::empty()
    )]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    ///
    /// # Description
    ///
    /// Inserts a value into the sorted vector, maintaining sorted order. If the value already
    /// exists, the old value is replaced and returned.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to insert.
    ///
    /// # Returns
    ///
    /// `Some(old_value)` if the value was already present, `None` otherwise.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            obeys_cmp_spec::<T>()
        ensures
            self.inv(),
            // Biconditional: replacement iff value was Ord-present
            result.is_some() <==> spec_contains(old(self)@, value),
            // Replacement case: positional update at the Ord-equal index
            result.is_some() ==> {
                &&& old(self)@.contains(result.unwrap())
                &&& result.unwrap().cmp_spec(&value) is Equal
                &&& self@.len() == old(self)@.len()
                &&& spec_contains(self@, value)
                // POSITIONAL frame: self@ equals old@ with exactly one position updated
                &&& exists|idx: int| #![auto] 0 <= idx < old(self)@.len()
                    && old(self)@[idx].cmp_spec(&value) is Equal
                    && self@ == old(self)@.update(idx, value)
            },
            // New insertion case: positional insert at sorted position
            result.is_none() ==> {
                &&& self@.len() == old(self)@.len() + 1
                &&& spec_contains(self@, value)
                // POSITIONAL frame: self@ equals old@ with value inserted at sorted position
                &&& exists|idx: int| #![auto] 0 <= idx <= old(self)@.len()
                    && self@ == old(self)@.insert(idx, value)
            },
            self@.contains(value),
            // Bidirectional frame: no spurious elements introduced
            forall|v: T| self@.contains(v) ==> (old(self)@.contains(v) || v == value)
    )]
    pub fn insert(&mut self, value: T) -> Option<T> {
        match self.inner.binary_search(&value) {
            Ok(index) => {
                let old_val: T = vec_replace(&mut self.inner, index, value);
                proof! {
                    lemma_insert_replace_maintains_inv(old(self)@, index as int, value);
                }
                Some(old_val)
            },
            Err(index) => {
                self.inner.insert(index, value);
                proof! {
                    let ghost _vec_len: usize = vstd::std_specs::vec::spec_vec_len(&self.inner);
                    lemma_insert_new_maintains_inv(old(self)@, index as int, value);
                }
                None
            },
        }
    }

    ///
    /// # Description
    ///
    /// Removes a value from the sorted vector.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to remove.
    ///
    /// # Returns
    ///
    /// `Some(removed_value)` if found, `None` otherwise.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            obeys_cmp_spec::<T>()
        ensures
            self.inv(),
            result.is_some() <==> spec_contains(old(self)@, *value),
            // Found: positional removal at the Ord-equal index
            result.is_some() ==> {
                &&& result.unwrap().cmp_spec(value) is Equal
                &&& self@.len() == old(self)@.len() - 1
                &&& !spec_contains(self@, *value)
                &&& old(self)@.contains(result.unwrap())
                // POSITIONAL frame: self@ equals old@ with exactly one position removed
                &&& exists|idx: int| #![auto] 0 <= idx < old(self)@.len()
                    && old(self)@[idx] == result.unwrap()
                    && self@ == old(self)@.remove(idx)
            },
            // Not found: state completely unchanged
            result.is_none() ==> {
                &&& !spec_contains(old(self)@, *value)
                &&& self@ == old(self)@
            },
            // Bidirectional frame: no spurious elements
            forall|v: T| self@.contains(v) ==> old(self)@.contains(v)
    )]
    pub fn remove(&mut self, value: &T) -> Option<T> {
        match self.inner.binary_search(value) {
            Ok(index) => {
                proof! { lemma_remove_maintains_inv(self@, index as int, *value); }
                Some(self.inner.remove(index))
            },
            Err(_) => None,
        }
    }

    ///
    /// # Description
    ///
    /// Removes an element by extracting a comparable key from each entry.
    ///
    /// The key function must extract a value whose natural ordering is consistent with the
    /// ascending [`Ord`] order of the underlying sorted vector. The library performs the
    /// comparison internally, so ordering correctness is enforced by construction.
    ///
    /// # Parameters
    ///
    /// - `key`: The key value to search for.
    /// - `f`: A function that extracts a comparable key of type `K` from each element.
    ///
    /// # Returns
    ///
    /// `Some(removed_value)` if found, `None` otherwise.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            obeys_cmp_spec::<K>(),
            // Key extraction is consistent with sorted order
            forall|i: int, j: int, x: K, y: K| {
                &&& 0 <= i < j < old(self)@.len()
                &&& f.ensures((&old(self)@[i], ), x)
                &&& f.ensures((&old(self)@[j], ), y)
            } ==> !(x.cmp_spec(&y) is Greater)
        ensures
            self.inv(),
            // Found: element removed, key matched
            result.is_some() ==> {
                &&& old(self)@.contains(result.unwrap())
                &&& maps_to_key(f, &result.unwrap(), key)
                &&& self@.len() == old(self)@.len() - 1
                // Positional frame
                &&& exists|idx: int| #![auto] 0 <= idx < old(self)@.len()
                    && old(self)@[idx] == result.unwrap()
                    && self@ == old(self)@.remove(idx)
            },
            // Not found: state unchanged
            result.is_none() ==> self@ == old(self)@,
            // Bidirectional frame
            forall|v: T| self@.contains(v) ==> old(self)@.contains(v),
            // Forward frame: non-removed elements preserved
            result.is_some() ==> forall|v: T|
                old(self)@.contains(v) && v != result.unwrap() ==> self@.contains(v)
    )]
    pub fn remove_by<K, F>(&mut self, key: &K, f: F) -> Option<T>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        match self.inner.binary_search_by_key(key, f) {
            Ok(index) => {
                proof_decl! { let ghost old_seq = old(self)@; }
                let removed = self.inner.remove(index);
                proof! {
                    lemma_remove_forward_frame(old_seq, index as int, removed);
                }
                Some(removed)
            },
            Err(_) => None,
        }
    }

    ///
    /// # Description
    ///
    /// Returns `true` if the sorted vector contains the given value.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to search for.
    ///
    /// # Returns
    ///
    /// `true` if the value is found, `false` otherwise.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
            obeys_cmp_spec::<T>()
        ensures
            result <==> spec_contains(self@, *value)
    )]
    pub fn contains(&self, value: &T) -> bool {
        self.inner.binary_search(value).is_ok()
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the element matching the given value, using binary search.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to search for.
    ///
    /// # Returns
    ///
    /// `Some(&element)` if found, `None` otherwise.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
            obeys_cmp_spec::<T>()
        ensures
            result.is_some() <==> spec_contains(self@, *value),
            result.is_some() ==> {
                &&& (*result.unwrap()).cmp_spec(value) is Equal
                // Exact position: the returned element is at binary_search's index
                &&& exists|i: int| #![auto] 0 <= i < self@.len()
                    && self@[i] == *result.unwrap()
                    && self@[i].cmp_spec(value) is Equal
                    // Uniqueness: this is the ONLY Ord-equal element
                    && forall|j: int| #![auto] 0 <= j < self@.len() && j != i ==> !(self@[j].cmp_spec(value) is Equal)
            }
    )]
    pub fn get(&self, value: &T) -> Option<&T> {
        match self.inner.binary_search(value) {
            Ok(index) => {
                proof! {
                    lemma_get_uniqueness(self@, index as int, *value);
                }
                Some(&self.inner[index])
            },
            Err(_) => None,
        }
    }

    ///
    /// # Description
    ///
    /// Searches the sorted vector by extracting a comparable key from each entry.
    ///
    /// The key function must extract a value whose natural ordering is consistent with the
    /// ascending [`Ord`] order of the underlying sorted vector. The library performs the
    /// comparison internally, so ordering correctness is enforced by construction.
    ///
    /// # Parameters
    ///
    /// - `key`: The key value to search for.
    /// - `f`: A function that extracts a comparable key of type `K` from each element.
    ///
    /// # Returns
    ///
    /// `Some(&element)` if found, `None` otherwise.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
            obeys_cmp_spec::<K>(),
            // Key extraction is deterministic (one output per input)
            forall|i: int, x1: K, x2: K|
                0 <= i < self@.len()
                && f.ensures((&self@[i], ), x1)
                && f.ensures((&self@[i], ), x2)
                ==> x1 == x2,
            // Key extraction is consistent with sorted order
            forall|i: int, j: int, x: K, y: K| {
                &&& 0 <= i < j < self@.len()
                &&& f.ensures((&self@[i], ), x)
                &&& f.ensures((&self@[j], ), y)
            } ==> !(x.cmp_spec(&y) is Greater)
        ensures
            result.is_some() ==> {
                &&& exists|i: int| #![auto] 0 <= i < self@.len() && self@[i] == *result.unwrap()
                &&& maps_to_key(f, result.unwrap(), key)
            },
            result.is_none() ==> forall|i: int| #![auto]
                0 <= i < self@.len() ==> !maps_to_key(f, &self@[i], key)
    )]
    pub fn lookup_by<K, F>(&self, key: &K, f: F) -> Option<&T>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        match self.inner.binary_search_by_key(key, f) {
            Ok(index) => Some(&self.inner[index]),
            Err(_) => {
                proof! {
                    reveal_cmp_laws!();
                }
                None
            },
        }
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the smallest element.
    ///
    /// # Returns
    ///
    /// `Some(&element)` if non-empty, `None` otherwise.
    ///
    #[verus_spec(result =>
        requires
            self.inv()
        ensures
            result.is_some() <==> self@.len() > 0,
            result.is_some() ==> *result.unwrap() == self@[0],
            result.is_some() ==> forall|i: int| 0 < i < self@.len()
                ==> (#[trigger] self@[0]).cmp_spec(&self@[i]) is Less
    )]
    pub fn first(&self) -> Option<&T> {
        self.inner.first()
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the largest element.
    ///
    /// # Returns
    ///
    /// `Some(&element)` if non-empty, `None` otherwise.
    ///
    #[verus_spec(result =>
        requires
            self.inv()
        ensures
            result.is_some() <==> self@.len() > 0,
            result.is_some() ==> *result.unwrap() == self@[self@.len() - 1],
            result.is_some() ==> forall|i: int| 0 <= i < self@.len() - 1
                ==> (#[trigger] self@[i]).cmp_spec(&self@[self@.len() - 1]) is Less
    )]
    pub fn last(&self) -> Option<&T> {
        self.inner.last()
    }

    ///
    /// # Description
    ///
    /// Returns an iterator over the elements in sorted order.
    ///
    /// # Returns
    ///
    /// An iterator yielding references to elements in ascending order.
    ///
    #[verus_spec(iter =>
        ensures
            ({
            let (index, seq) = iter@;
            &&& index == 0
            &&& seq == self@
        })
    )]
    pub fn iter(&self) -> ::core::slice::Iter<'_, T> {
        self.inner.iter()
    }

    ///
    /// # Description
    ///
    /// Returns a slice of the underlying sorted elements.
    ///
    /// # Returns
    ///
    /// A slice of all elements in sorted order.
    ///
    #[verus_spec(result =>
        requires
            self.inv()
        ensures
            result@ == self@
    )]
    pub fn as_slice(&self) -> &[T] {
        self.inner.as_slice()
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

#[verus_verify]
impl<T: Ord> Default for SortedVec<T> {
    #[verus_spec(result =>
        ensures
            result.inv(),
            result@ == Seq::<T>::empty()
    )]
    fn default() -> Self {
        Self::new()
    }
}

#[verus_verify]
impl<T: Ord> From<Vec<T>> for SortedVec<T> {
    ///
    /// # Description
    ///
    /// Creates a [`SortedVec`] from an unsorted [`Vec`]. The vector is sorted and duplicates
    /// are removed.
    ///
    #[verus_spec(result =>
        ensures
            obeys_cmp_spec::<T>() ==> result.inv(),
            result@.len() <= vec@.len(),
            forall|v: T| result@.contains(v) ==> vec@.contains(v)
    )]
    fn from(vec: Vec<T>) -> Self {
        let mut vec = vec;
        proof! { let ghost _len: usize = vstd::std_specs::vec::spec_vec_len(&vec); }
        vec_sort_unstable(&mut vec);
        vec.dedup();
        proof! {
            reveal_cmp_laws!();
        }
        Self { inner: vec }
    }
}

#[verus_verify]
impl<T: Ord> IntoIterator for SortedVec<T> {
    type Item = T;
    type IntoIter = ::alloc::vec::IntoIter<T>;

    #[verus_spec(iter =>
        ensures
            iter@ == (0int, self@)
    )]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[verus_verify]
impl<'a, T: Ord> IntoIterator for &'a SortedVec<T> {
    type Item = &'a T;
    type IntoIter = ::core::slice::Iter<'a, T>;

    #[verus_spec(iter =>
        ensures
            iter@ == (0int, self@)
    )]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}
