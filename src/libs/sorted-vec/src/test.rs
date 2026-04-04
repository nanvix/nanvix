// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::SortedVec;
use alloc::{
    vec,
    vec::Vec,
};
use core::cmp::Ordering;

/// Helper type whose `Ord`/`Eq` compare only by `key`, carrying a separate `payload`.
#[derive(Debug, Clone)]
struct KeyValue {
    key: i32,
    payload: &'static str,
}

impl PartialEq for KeyValue {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for KeyValue {}

impl PartialOrd for KeyValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KeyValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

//==================================================================================================
// Creation Tests
//==================================================================================================

#[test]
fn test_new_is_empty() {
    let sv: SortedVec<i32> = SortedVec::new();
    assert!(sv.is_empty());
    assert_eq!(sv.len(), 0);
}

#[test]
fn test_with_capacity() {
    let sv: SortedVec<i32> = SortedVec::with_capacity(16);
    assert!(sv.is_empty());
    assert!(sv.capacity() >= 16);
}

#[test]
fn test_default() {
    let sv: SortedVec<i32> = SortedVec::default();
    assert!(sv.is_empty());
}

//==================================================================================================
// Insertion Tests
//==================================================================================================

#[test]
fn test_insert_single() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    let result: Option<i32> = sv.insert(42);
    assert!(result.is_none());
    assert_eq!(sv.len(), 1);
    assert_eq!(sv.get(&42), Some(&42));
}

#[test]
fn test_insert_maintains_order() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(30);
    sv.insert(10);
    sv.insert(20);
    assert_eq!(sv.as_slice(), &[10, 20, 30]);
}

#[test]
fn test_insert_duplicate_replaces() {
    let mut sv: SortedVec<KeyValue> = SortedVec::new();
    sv.insert(KeyValue {
        key: 10,
        payload: "first",
    });
    let old: Option<KeyValue> = sv.insert(KeyValue {
        key: 10,
        payload: "second",
    });
    assert_eq!(sv.len(), 1);
    let old = old.expect("duplicate insert should return the old element");
    assert_eq!(old.payload, "first");
    assert_eq!(
        sv.get(&KeyValue {
            key: 10,
            payload: ""
        })
        .expect("element should exist after replacement")
        .payload,
        "second"
    );
}

#[test]
fn test_insert_ascending() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    for i in 0..10 {
        sv.insert(i);
    }
    let expected: Vec<i32> = (0..10).collect();
    assert_eq!(sv.as_slice(), expected.as_slice());
}

#[test]
fn test_insert_descending() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    for i in (0..10).rev() {
        sv.insert(i);
    }
    let expected: Vec<i32> = (0..10).collect();
    assert_eq!(sv.as_slice(), expected.as_slice());
}

//==================================================================================================
// Removal Tests
//==================================================================================================

#[test]
fn test_remove_existing() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(10);
    sv.insert(20);
    sv.insert(30);
    let removed: Option<i32> = sv.remove(&20);
    assert_eq!(removed, Some(20));
    assert_eq!(sv.len(), 2);
    assert_eq!(sv.as_slice(), &[10, 30]);
}

#[test]
fn test_remove_nonexistent() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(10);
    let removed: Option<i32> = sv.remove(&99);
    assert!(removed.is_none());
    assert_eq!(sv.len(), 1);
}

#[test]
fn test_remove_from_empty() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    let removed: Option<i32> = sv.remove(&1);
    assert!(removed.is_none());
}

#[test]
fn test_remove_by_existing() {
    let mut sv: SortedVec<(i32, &str)> = SortedVec::new();
    sv.insert((1, "one"));
    sv.insert((2, "two"));
    sv.insert((3, "three"));
    let removed: Option<(i32, &str)> = sv.remove_by(&2, |&(k, _)| k);
    assert_eq!(removed, Some((2, "two")));
    assert_eq!(sv.len(), 2);
    assert_eq!(sv.as_slice(), &[(1, "one"), (3, "three")]);
}

#[test]
fn test_remove_by_not_found() {
    let mut sv: SortedVec<(i32, &str)> = SortedVec::new();
    sv.insert((1, "one"));
    sv.insert((3, "three"));
    let removed: Option<(i32, &str)> = sv.remove_by(&2, |&(k, _)| k);
    assert!(removed.is_none());
    assert_eq!(sv.len(), 2);
}

//==================================================================================================
// Lookup Tests
//==================================================================================================

#[test]
fn test_contains() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(5);
    sv.insert(15);
    assert!(sv.contains(&5));
    assert!(sv.contains(&15));
    assert!(!sv.contains(&10));
}

#[test]
fn test_get_existing() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(42);
    assert_eq!(sv.get(&42), Some(&42));
}

#[test]
fn test_get_nonexistent() {
    let sv: SortedVec<i32> = SortedVec::new();
    assert_eq!(sv.get(&42), None);
}

//==================================================================================================
// Lookup Tests (Custom Comparator)
//==================================================================================================

#[test]
fn test_lookup() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(10);
    sv.insert(20);
    sv.insert(30);
    let result: Option<&i32> = sv.lookup_by(&20, |x| *x);
    assert_eq!(result, Some(&20));
}

#[test]
fn test_lookup_not_found() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(10);
    sv.insert(30);
    let result: Option<&i32> = sv.lookup_by(&20, |x| *x);
    assert!(result.is_none());
}

#[test]
fn test_lookup_by_key() {
    let mut sv: SortedVec<(i32, &str)> = SortedVec::new();
    sv.insert((1, "one"));
    sv.insert((2, "two"));
    sv.insert((3, "three"));
    let result: Option<&(i32, &str)> = sv.lookup_by(&2, |&(k, _)| k);
    assert_eq!(result, Some(&(2, "two")));
}

#[test]
fn test_lookup_by_key_not_found() {
    let mut sv: SortedVec<(i32, &str)> = SortedVec::new();
    sv.insert((1, "one"));
    sv.insert((3, "three"));
    let result: Option<&(i32, &str)> = sv.lookup_by(&2, |&(k, _)| k);
    assert!(result.is_none());
}

//==================================================================================================
// First/Last Tests
//==================================================================================================

#[test]
fn test_first_last() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(30);
    sv.insert(10);
    sv.insert(20);
    assert_eq!(sv.first(), Some(&10));
    assert_eq!(sv.last(), Some(&30));
}

#[test]
fn test_first_last_empty() {
    let sv: SortedVec<i32> = SortedVec::new();
    assert_eq!(sv.first(), None);
    assert_eq!(sv.last(), None);
}

//==================================================================================================
// Clear Tests
//==================================================================================================

#[test]
fn test_clear() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(1);
    sv.insert(2);
    sv.insert(3);
    sv.clear();
    assert!(sv.is_empty());
    assert_eq!(sv.len(), 0);
}

//==================================================================================================
// Conversion Tests
//==================================================================================================

#[test]
fn test_from_vec_sorts_and_dedups() {
    let sv: SortedVec<i32> = SortedVec::from(vec![3, 1, 2, 1, 3]);
    assert_eq!(sv.as_slice(), &[1, 2, 3]);
    assert_eq!(sv.len(), 3);
}

#[test]
fn test_from_empty_vec() {
    let sv: SortedVec<i32> = SortedVec::from(vec![]);
    assert!(sv.is_empty());
}

//==================================================================================================
// Iterator Tests
//==================================================================================================

#[test]
fn test_iter() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(30);
    sv.insert(10);
    sv.insert(20);
    let collected: Vec<&i32> = sv.iter().collect();
    assert_eq!(collected, vec![&10, &20, &30]);
}

#[test]
fn test_into_iter() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(3);
    sv.insert(1);
    sv.insert(2);
    let collected: Vec<i32> = sv.into_iter().collect();
    assert_eq!(collected, vec![1, 2, 3]);
}

#[test]
fn test_ref_into_iter() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(3);
    sv.insert(1);
    sv.insert(2);
    let collected: Vec<&i32> = (&sv).into_iter().collect();
    assert_eq!(collected, vec![&1, &2, &3]);
}

//==================================================================================================
// Edge Case Tests
//==================================================================================================

#[test]
fn test_single_element() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(42);
    assert_eq!(sv.first(), Some(&42));
    assert_eq!(sv.last(), Some(&42));
    assert!(sv.contains(&42));
}

#[test]
fn test_insert_remove_reinsert() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(10);
    sv.remove(&10);
    assert!(sv.is_empty());
    sv.insert(10);
    assert_eq!(sv.len(), 1);
    assert!(sv.contains(&10));
}

#[test]
fn test_large_insertion() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    for i in (0..100).rev() {
        sv.insert(i);
    }
    assert_eq!(sv.len(), 100);
    for i in 0..100 {
        assert!(sv.contains(&i));
    }
}

#[test]
fn test_clone() {
    let mut sv: SortedVec<i32> = SortedVec::new();
    sv.insert(1);
    sv.insert(2);
    let cloned: SortedVec<i32> = sv.clone();
    assert_eq!(sv.as_slice(), cloned.as_slice());
}
