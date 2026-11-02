// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(clippy::unwrap_used)]

use super::*;
use ::alloc::collections::VecDeque;

#[test]
fn test_new() {
    let vec_deque = NonEmptyVecDeque::new(0);
    assert_eq!(vec_deque.head, 0);
    assert_eq!(vec_deque.tail, None);
}

#[test]
fn test_from() {
    let vec = VecDeque::from([0, 1, 2]);
    let vec_deque = NonEmptyVecDeque::from(vec.clone()).unwrap();
    assert_eq!(vec_deque.head, 0);
    assert_eq!(vec_deque.tail, Some(VecDeque::from([1, 2])));
}

#[test]
fn test_from_empty() {
    let vec = VecDeque::<usize>::new();
    let vec_deque = NonEmptyVecDeque::from(vec);
    assert_eq!(vec_deque, None);
}

#[test]
fn test_push_back() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    assert_eq!(vec_deque.head, 0);
    assert_eq!(vec_deque.tail, Some(VecDeque::from([1])));
}

#[test]
fn test_push_back_with_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    assert_eq!(vec_deque.head, 0);
    assert_eq!(vec_deque.tail, Some(VecDeque::from([1, 2])));
}

#[test]
fn test_append_some() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    let mut other = NonEmptyVecDeque::new(1);
    other.push_back(2);
    vec_deque.append(other);
    assert_eq!(vec_deque.head, 0);
    assert_eq!(vec_deque.tail, Some(VecDeque::from([1, 2])));
}

#[test]
fn test_pop_front() {
    let vec_deque = NonEmptyVecDeque::new(0);
    let (tail, head) = vec_deque.pop_front();
    assert_eq!(head, 0);
    assert_eq!(tail, VecDeque::new());
}

#[test]
fn test_pop_front_with_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    let (tail, head) = vec_deque.pop_front();
    assert_eq!(head, 0);
    assert_eq!(tail, VecDeque::from([1]));
}

#[test]
fn test_remove_if_exists_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let (tail, head) = vec_deque.remove_if(|value| *value == 1).unwrap();
    assert_eq!(head, 1);
    assert_eq!(tail, VecDeque::from([0, 2]));
}

#[test]
fn test_remove_if_first() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let (tail, head) = vec_deque.remove_if(|value| *value == 0).unwrap();
    assert_eq!(head, 0);
    assert_eq!(tail, VecDeque::from([1, 2]));
}

#[test]
fn test_remove_if_doest_not_exist() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let result = vec_deque.remove_if(|value| *value == 3);
    assert_eq!(
        result,
        Err(NonEmptyVecDeque {
            head: 0,
            tail: Some(VecDeque::from([1, 2])),
        })
    );
}

#[test]
fn test_map() {
    let vec_deque = NonEmptyVecDeque::new(0);
    let new_vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::map(vec_deque, |value| value + 1);
    assert_eq!(new_vec_deque.head, 1);
    assert_eq!(new_vec_deque.tail, None);
}

#[test]
fn test_map_with_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    let new_vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::map(vec_deque, |value| value + 1);
    assert_eq!(new_vec_deque.head, 1);
    assert_eq!(new_vec_deque.tail, Some(VecDeque::from([2])));
}

///
/// # Description
///
/// Verifies that [`NonEmptyVecDeque::map`] accepts a stateful closure and invokes it once for
/// every element in order.
///
#[test]
fn test_map_with_stateful_closure() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let mut calls: i32 = 0;
    let new_vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::map(vec_deque, |value| {
        calls += 1;
        value + calls
    });
    assert_eq!(new_vec_deque.head, 1);
    assert_eq!(new_vec_deque.tail, Some(VecDeque::from([3, 5])));
    assert_eq!(calls, 3);
}

#[test]
fn test_iter() {
    let vec_deque = NonEmptyVecDeque::new(0);
    let mut iter = vec_deque.iter();
    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_iter_with_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    let mut iter = vec_deque.iter();
    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_iter_mut() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    {
        let mut iter = vec_deque.iter_mut();
        if let Some(value) = iter.next() {
            *value += 1;
        }
    }
    assert_eq!(vec_deque.head, 1);
    assert_eq!(vec_deque.tail, None);
}

#[test]
fn test_iter_mut_with_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    {
        let mut iter = vec_deque.iter_mut();
        if let Some(value) = iter.next() {
            *value += 1;
        }
        if let Some(value) = iter.next() {
            *value += 1;
        }
    }
    assert_eq!(vec_deque.head, 1);
    assert_eq!(vec_deque.tail, Some(VecDeque::from([2])));
}

#[test]
fn test_into_vec_deque() {
    let mut vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let converted: VecDeque<i32> = VecDeque::from(vec_deque);
    assert_eq!(converted, VecDeque::from([0, 1, 2]));
}

#[test]
fn test_into_vec_deque_single() {
    let vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::new(42);
    let converted: VecDeque<i32> = VecDeque::from(vec_deque);
    assert_eq!(converted, VecDeque::from([42]));
}

#[test]
fn test_into_vec_deque_two() {
    let mut vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::new(10);
    vec_deque.push_back(20);
    let converted: VecDeque<i32> = VecDeque::from(vec_deque);
    assert_eq!(converted, VecDeque::from([10, 20]));
}

#[test]
fn test_into_vec_deque_order_preserved() {
    let mut vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::new(0);
    for i in 1..8 {
        vec_deque.push_back(i);
    }
    let converted: VecDeque<i32> = VecDeque::from(vec_deque);
    assert_eq!(converted, VecDeque::from([0, 1, 2, 3, 4, 5, 6, 7]));
}

#[test]
fn test_remove_if_head_single() {
    let vec_deque = NonEmptyVecDeque::new(5);
    let (tail, removed) = vec_deque.remove_if(|value| *value == 5).unwrap();
    assert_eq!(removed, 5);
    assert_eq!(tail, VecDeque::new());
}

#[test]
fn test_remove_if_no_match_single() {
    let vec_deque = NonEmptyVecDeque::new(5);
    let result = vec_deque.remove_if(|value| *value == 99);
    assert_eq!(
        result,
        Err(NonEmptyVecDeque {
            head: 5,
            tail: None,
        })
    );
}

#[test]
fn test_remove_if_head_two_elements() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    let (tail, removed) = vec_deque.remove_if(|value| *value == 0).unwrap();
    assert_eq!(removed, 0);
    assert_eq!(tail, VecDeque::from([1]));
}

#[test]
fn test_remove_if_tail_two_elements() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    let (tail, removed) = vec_deque.remove_if(|value| *value == 1).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(tail, VecDeque::from([0]));
}

#[test]
fn test_remove_if_last_in_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let (tail, removed) = vec_deque.remove_if(|value| *value == 2).unwrap();
    assert_eq!(removed, 2);
    assert_eq!(tail, VecDeque::from([0, 1]));
}

#[test]
fn test_remove_if_order_preserved() {
    let mut vec_deque = NonEmptyVecDeque::new(10);
    vec_deque.push_back(20);
    vec_deque.push_back(30);
    vec_deque.push_back(40);
    let (tail, removed) = vec_deque.remove_if(|value| *value == 20).unwrap();
    assert_eq!(removed, 20);
    assert_eq!(tail, VecDeque::from([10, 30, 40]));
}

#[test]
fn test_remove_if_first_match_only() {
    let mut vec_deque = NonEmptyVecDeque::new(1);
    vec_deque.push_back(2);
    vec_deque.push_back(2);
    vec_deque.push_back(3);
    let (tail, removed) = vec_deque.remove_if(|value| *value == 2).unwrap();
    assert_eq!(removed, 2);
    assert_eq!(tail, VecDeque::from([1, 2, 3]));
}

#[test]
fn test_remove_if_head_duplicate() {
    let mut vec_deque = NonEmptyVecDeque::new(1);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let (tail, removed) = vec_deque.remove_if(|value| *value == 1).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(tail, VecDeque::from([1, 2]));
}

#[test]
fn test_map_large_tail() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    for i in 1..6 {
        vec_deque.push_back(i);
    }
    let mapped: NonEmptyVecDeque<i32> = NonEmptyVecDeque::map(vec_deque, |v| v * 10);
    assert_eq!(mapped.head, 0);
    assert_eq!(mapped.tail, Some(VecDeque::from([10, 20, 30, 40, 50])));
}

#[test]
fn test_map_type_change() {
    let mut vec_deque: NonEmptyVecDeque<i32> = NonEmptyVecDeque::new(1);
    vec_deque.push_back(2);
    vec_deque.push_back(3);
    let mapped: NonEmptyVecDeque<bool> = NonEmptyVecDeque::map(vec_deque, |v| v > 1);
    assert!(!mapped.head);
    assert_eq!(mapped.tail, Some(VecDeque::from([true, true])));
}

#[test]
fn test_map_preserves_count() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    for i in 1..10 {
        vec_deque.push_back(i);
    }
    let mapped: NonEmptyVecDeque<i32> = NonEmptyVecDeque::map(vec_deque, |v| v + 1);
    let count = mapped.iter().count();
    assert_eq!(count, 10);
}

#[test]
fn test_pop_front_three_elements() {
    let mut vec_deque = NonEmptyVecDeque::new(0);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let (tail, head) = vec_deque.pop_front();
    assert_eq!(head, 0);
    assert_eq!(tail, VecDeque::from([1, 2]));
}

#[test]
fn test_roundtrip_from_into() {
    let original = VecDeque::from([10, 20, 30, 40]);
    let non_empty = NonEmptyVecDeque::from(original.clone()).unwrap();
    let converted: VecDeque<i32> = VecDeque::from(non_empty);
    assert_eq!(converted, original);
}

#[test]
fn test_remove_min_by_key_single() {
    let vec_deque = NonEmptyVecDeque::new(42);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 42);
    assert_eq!(remaining, VecDeque::new());
}

#[test]
fn test_remove_min_by_key_head_is_min() {
    let mut vec_deque = NonEmptyVecDeque::new(1);
    vec_deque.push_back(3);
    vec_deque.push_back(2);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 1);
    assert_eq!(remaining, VecDeque::from([3, 2]));
}

#[test]
fn test_remove_min_by_key_tail_is_min() {
    let mut vec_deque = NonEmptyVecDeque::new(5);
    vec_deque.push_back(2);
    vec_deque.push_back(8);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 2);
    assert_eq!(remaining, VecDeque::from([5, 8]));
}

#[test]
fn test_remove_min_by_key_last_is_min() {
    let mut vec_deque = NonEmptyVecDeque::new(5);
    vec_deque.push_back(3);
    vec_deque.push_back(1);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 1);
    assert_eq!(remaining, VecDeque::from([5, 3]));
}

#[test]
fn test_remove_min_by_key_duplicates_takes_first() {
    let mut vec_deque = NonEmptyVecDeque::new(3);
    vec_deque.push_back(1);
    vec_deque.push_back(1);
    vec_deque.push_back(5);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 1);
    assert_eq!(remaining, VecDeque::from([3, 1, 5]));
}

#[test]
fn test_remove_min_by_key_head_duplicate_min() {
    let mut vec_deque = NonEmptyVecDeque::new(1);
    vec_deque.push_back(1);
    vec_deque.push_back(2);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 1);
    assert_eq!(remaining, VecDeque::from([1, 2]));
}

#[test]
fn test_remove_min_by_key_order_preserved() {
    let mut vec_deque = NonEmptyVecDeque::new(40);
    vec_deque.push_back(10);
    vec_deque.push_back(30);
    vec_deque.push_back(20);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 10);
    assert_eq!(remaining, VecDeque::from([40, 30, 20]));
}

#[test]
fn test_remove_min_by_key_custom_key() {
    // Use a derived key: minimize (value % 10), so 30 has key 0, 15 has key 5, etc.
    let mut vec_deque = NonEmptyVecDeque::new(15);
    vec_deque.push_back(22);
    vec_deque.push_back(30);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| v % 10);
    assert_eq!(min, 30);
    assert_eq!(remaining, VecDeque::from([15, 22]));
}

#[test]
fn test_remove_min_by_key_two_elements() {
    let mut vec_deque = NonEmptyVecDeque::new(10);
    vec_deque.push_back(5);
    let (remaining, min) = vec_deque.remove_min_by_key(|v| *v);
    assert_eq!(min, 5);
    assert_eq!(remaining, VecDeque::from([10]));
}
