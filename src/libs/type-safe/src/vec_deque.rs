// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::{
    VecDeque,
    vec_deque,
};

//==================================================================================================
// Non Empty Vector Dequeue
//==================================================================================================

///
/// # Description
///
/// A vector that is guaranteed to have at least one element.
///
#[derive(Debug, PartialEq)]
pub struct NonEmptyVecDeque<T> {
    /// The first element of the vector.
    head: T,
    /// The remaining elements of the vector.
    tail: Option<VecDeque<T>>,
}

impl<T> NonEmptyVecDeque<T> {
    ///
    /// # Description
    ///
    /// Creates a new type-safe vector.
    ///
    pub fn new(first: T) -> Self {
        Self {
            head: first,
            tail: None,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new type-safe vector from a vector.
    ///
    /// # Parameters
    ///
    /// - `vec` - The vector.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns a type-safe vector. Otherwise, it returns `None`.
    ///
    pub fn from(mut vec: VecDeque<T>) -> Option<Self> {
        vec.pop_front().map(|head| Self {
            head,
            tail: Some(vec),
        })
    }

    ///
    /// # Description
    ///
    /// Pushes a element to the back of `self`.
    ///
    /// # Parameters
    ///
    /// - `value` - The element to push.
    ///
    pub fn push_back(&mut self, value: T) {
        if let Some(ref mut tail) = self.tail {
            tail.push_back(value);
        } else {
            self.tail = Some(VecDeque::from([value]));
        }
    }

    ///
    /// # Description
    ///
    /// Move all elements from `other` to the end of `self`, leaving `other` empty.
    ///
    pub fn append(&mut self, mut other: Self) {
        self.push_back(other.head);
        if let Some(mut tail) = other.tail.take() {
            if let Some(ref mut self_tail) = self.tail {
                self_tail.append(&mut tail);
            } else {
                self.tail = Some(tail);
            }
        }
    }

    ///
    /// # Description
    ///
    /// Pops the first element in `self`.
    ///
    /// # Returns
    ///
    /// Consumes `self` and returns a tuple containing the first element and the remaining values
    /// (if any).
    ///
    pub fn pop_front(mut self) -> (VecDeque<T>, T) {
        match self.tail.take() {
            Some(tail) => (tail, self.head),
            None => (VecDeque::new(), self.head),
        }
    }

    ///
    /// # Description
    ///
    /// Removes the first element in `self` that satisfies a condition.
    ///
    /// # Parameters
    ///
    /// - `condition` - The condition to satisfy.
    ///
    /// # Returns
    ///
    /// If any element satisfies the condition, the function returns a tuple containing the
    /// remaining elements and the removed element. Otherwise, it returns `Err(Self)`.
    ///
    pub fn remove_if(
        mut self,
        condition: impl Fn(&mut T) -> bool,
    ) -> Result<(VecDeque<T>, T), Self> {
        if condition(&mut self.head) {
            match self.tail.take() {
                Some(tail) => Ok((tail, self.head)),
                None => Ok((VecDeque::new(), self.head)),
            }
        } else {
            match self.tail.take() {
                Some(mut tail) => match tail.iter_mut().position(condition) {
                    Some(pos) => match tail.remove(pos) {
                        Some(extracted) => {
                            tail.push_front(self.head);
                            Ok((tail, extracted))
                        },
                        None => Err(Self {
                            head: self.head,
                            tail: Some(tail),
                        }),
                    },
                    None => Err(Self {
                        head: self.head,
                        tail: Some(tail),
                    }),
                },
                None => Err(Self {
                    head: self.head,
                    tail: None,
                }),
            }
        }
    }

    ///
    /// # Description
    ///
    /// Maps a function over the elements of `self`.
    ///
    /// # Parameters
    ///
    /// - `f` - Mapping function.
    ///
    /// # Returns
    ///
    /// The function returns a new type-safe vector with the results of applying `f` to each element
    /// of `self`.
    ///
    pub fn map<U, F>(mut vec_deque: NonEmptyVecDeque<U>, f: F) -> NonEmptyVecDeque<T>
    where
        F: Fn(U) -> T,
    {
        let new_head: T = f(vec_deque.head);
        let new_tail: Option<VecDeque<T>> = vec_deque
            .tail
            .take()
            .map(|tail| tail.into_iter().map(&f).collect());
        NonEmptyVecDeque {
            head: new_head,
            tail: new_tail,
        }
    }

    ///
    /// # Description
    ///
    /// Removes the element with the minimum key from `self`.
    ///
    /// Since the deque is guaranteed to be non-empty, a minimum always exists and this operation
    /// cannot fail.
    ///
    /// # Parameters
    ///
    /// - `key_fn` - Function that extracts a comparable key from each element.
    ///
    /// # Returns
    ///
    /// A tuple containing the remaining elements and the removed minimum element.
    ///
    pub fn remove_min_by_key<K: Ord>(mut self, key_fn: impl Fn(&T) -> K) -> (VecDeque<T>, T) {
        match self.tail.take() {
            None => (VecDeque::new(), self.head),
            Some(mut tail) => {
                let mut min_key: K = key_fn(&self.head);
                let mut min_pos: Option<usize> = None;
                for (i, elem) in tail.iter().enumerate() {
                    let k: K = key_fn(elem);
                    if k < min_key {
                        min_key = k;
                        min_pos = Some(i);
                    }
                }
                match min_pos {
                    None => (tail, self.head),
                    Some(pos) => match tail.remove(pos) {
                        Some(extracted) => {
                            tail.push_front(self.head);
                            (tail, extracted)
                        },
                        // pos was found by enumerate on tail, so remove cannot fail.
                        // Fall back to head removal to stay panic-free.
                        None => (tail, self.head),
                    },
                }
            },
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            passed_head: false,
            head: Some(&self.head),
            tail: self.tail.as_ref().map(|tail| tail.iter()),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            passed_head: false,
            head: Some(&mut self.head),
            tail: self.tail.as_mut().map(|tail| tail.iter_mut()),
        }
    }
}

impl<T> From<NonEmptyVecDeque<T>> for VecDeque<T> {
    fn from(vec_deque: NonEmptyVecDeque<T>) -> Self {
        match vec_deque.tail {
            Some(mut tail) => {
                tail.push_front(vec_deque.head);
                tail
            },
            None => VecDeque::from([vec_deque.head]),
        }
    }
}

//==================================================================================================
// Iterator
//==================================================================================================

///
/// # Description
///
/// An iterator over the elements of a non-empty vector.
///
pub struct Iter<'a, T> {
    /// Indicates if the head element has been passed.
    passed_head: bool,
    /// The head element.
    head: Option<&'a T>,
    /// The remaining elements.
    tail: Option<vec_deque::Iter<'a, T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.passed_head {
            self.passed_head = true;
            self.head.take()
        } else {
            match self.tail {
                Some(ref mut tail) => tail.next(),
                None => None,
            }
        }
    }
}

//==================================================================================================
// Mutable Iterator
//==================================================================================================

///
/// # Description
///
/// A mutable iterator over the elements of a non-empty vector.
///
pub struct IterMut<'a, T> {
    /// Indicates if the head element has been passed.
    passed_head: bool,
    /// The head element.
    head: Option<&'a mut T>,
    /// The remaining elements.
    tail: Option<vec_deque::IterMut<'a, T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.passed_head {
            self.passed_head = true;
            self.head.take()
        } else {
            match self.tail {
                Some(ref mut tail) => tail.next(),
                None => None,
            }
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(feature = "std")]
#[cfg(test)]
mod test {
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
        let new_vec_deque: NonEmptyVecDeque<i32> =
            NonEmptyVecDeque::map(vec_deque, |value| value + 1);
        assert_eq!(new_vec_deque.head, 1);
        assert_eq!(new_vec_deque.tail, None);
    }

    #[test]
    fn test_map_with_tail() {
        let mut vec_deque = NonEmptyVecDeque::new(0);
        vec_deque.push_back(1);
        let new_vec_deque: NonEmptyVecDeque<i32> =
            NonEmptyVecDeque::map(vec_deque, |value| value + 1);
        assert_eq!(new_vec_deque.head, 1);
        assert_eq!(new_vec_deque.tail, Some(VecDeque::from([2])));
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
}

//==================================================================================================
// Benchmarks
//==================================================================================================

#[cfg(feature = "std")]
#[cfg(test)]
mod benchmarks {
    #![allow(clippy::unwrap_used)]

    extern crate test;

    use super::*;
    use test::{
        Bencher,
        black_box,
    };

    #[bench]
    fn bench_new(b: &mut Bencher) {
        b.iter(|| black_box(NonEmptyVecDeque::new(0)));
    }

    #[bench]
    fn bench_from(b: &mut Bencher) {
        let vec = VecDeque::from([0, 1]);
        b.iter(|| black_box(NonEmptyVecDeque::from(vec.clone())));
    }

    #[bench]
    fn bench_push_back(b: &mut Bencher) {
        b.iter(|| {
            let mut vec = NonEmptyVecDeque::new(0);
            vec.push_back(1);
            black_box(vec);
        });
    }

    #[bench]
    fn bench_append(b: &mut Bencher) {
        b.iter(|| {
            let mut vec = NonEmptyVecDeque::from(VecDeque::from([0, 1])).unwrap();
            let other = NonEmptyVecDeque::from(VecDeque::from([2, 3])).unwrap();
            vec.append(other);
            black_box(vec);
        });
    }

    #[bench]
    fn bench_pop_front(b: &mut Bencher) {
        b.iter(|| {
            let vec = NonEmptyVecDeque::from(VecDeque::from([0, 1])).unwrap();
            black_box(vec.pop_front());
        });
    }

    #[bench]
    fn bench_remove_if(b: &mut Bencher) {
        b.iter(|| {
            let vec = NonEmptyVecDeque::from(VecDeque::from([0, 1])).unwrap();
            let _ = black_box(vec.remove_if(|value| *value == 1));
        });
    }

    #[bench]
    fn bench_remove_min_by_key(b: &mut Bencher) {
        b.iter(|| {
            let vec = NonEmptyVecDeque::from(VecDeque::from([3, 1, 2])).unwrap();
            black_box(vec.remove_min_by_key(|v| *v));
        });
    }

    #[bench]
    fn bench_remove_min_by_key_large(b: &mut Bencher) {
        b.iter(|| {
            let data: VecDeque<i32> = (0..64).collect();
            let vec = NonEmptyVecDeque::from(data).unwrap();
            black_box(vec.remove_min_by_key(|v| *v));
        });
    }

    #[bench]
    fn bench_map(b: &mut Bencher) {
        b.iter(|| {
            let vec = NonEmptyVecDeque::from(VecDeque::from([0, 1])).unwrap();
            black_box(NonEmptyVecDeque::map(vec, |value| value + 1));
        });
    }

    #[bench]
    fn bench_iter(b: &mut Bencher) {
        b.iter(|| {
            let vec = NonEmptyVecDeque::from(VecDeque::from([0, 1])).unwrap();
            for value in vec.iter() {
                black_box(value);
            }
        });
    }

    #[bench]
    fn bench_iter_mut(b: &mut Bencher) {
        b.iter(|| {
            let mut vec = NonEmptyVecDeque::from(VecDeque::from([0, 1])).unwrap();
            for value in vec.iter_mut() {
                black_box(value);
            }
        });
    }
}
