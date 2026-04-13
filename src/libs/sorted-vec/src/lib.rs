// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

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

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A sorted vector that maintains its elements in ascending order and provides efficient
/// lookup via binary search.
///
/// Elements must implement [`Ord`] for sorting and searching. The vector does not allow
/// duplicate elements; inserting a value that already exists replaces the old entry.
///
#[derive(Debug, Clone)]
pub struct SortedVec<T: Ord> {
    /// Underlying storage.
    inner: Vec<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

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
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    ///
    /// # Description
    ///
    /// Clears all elements from the sorted vector.
    ///
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
    #[must_use = "ignoring the return value may hide an unintended replacement"]
    pub fn insert(&mut self, value: T) -> Option<T> {
        match self.inner.binary_search(&value) {
            Ok(index) => {
                let old: T = ::core::mem::replace(&mut self.inner[index], value);
                Some(old)
            },
            Err(index) => {
                self.inner.insert(index, value);
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
    #[must_use = "ignoring the return value may hide a failed removal"]
    pub fn remove(&mut self, value: &T) -> Option<T> {
        match self.inner.binary_search(value) {
            Ok(index) => Some(self.inner.remove(index)),
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
    #[must_use = "ignoring the return value may hide a failed removal"]
    pub fn remove_by<K, F>(&mut self, key: &K, f: F) -> Option<T>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        match self.inner.binary_search_by_key(key, f) {
            Ok(index) => Some(self.inner.remove(index)),
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
    pub fn get(&self, value: &T) -> Option<&T> {
        match self.inner.binary_search(value) {
            Ok(index) => Some(&self.inner[index]),
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
    pub fn lookup_by<K, F>(&self, key: &K, f: F) -> Option<&T>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        match self.inner.binary_search_by_key(key, f) {
            Ok(index) => Some(&self.inner[index]),
            Err(_) => None,
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
    pub fn as_slice(&self) -> &[T] {
        self.inner.as_slice()
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl<T: Ord> Default for SortedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> From<Vec<T>> for SortedVec<T> {
    ///
    /// # Description
    ///
    /// Creates a [`SortedVec`] from an unsorted [`Vec`]. The vector is sorted and duplicates
    /// are removed.
    ///
    fn from(mut vec: Vec<T>) -> Self {
        vec.sort_unstable();
        vec.dedup();
        Self { inner: vec }
    }
}

impl<T: Ord> IntoIterator for SortedVec<T> {
    type Item = T;
    type IntoIter = ::alloc::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T: Ord> IntoIterator for &'a SortedVec<T> {
    type Item = &'a T;
    type IntoIter = ::core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}
