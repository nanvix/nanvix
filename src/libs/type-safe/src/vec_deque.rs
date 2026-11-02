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
    /// Maps a function over the elements of a non-empty vector dequeue.
    ///
    /// # Parameters
    ///
    /// - `vec_deque`: Non-empty vector dequeue whose elements are consumed.
    /// - `f`: Stateful mapping function to apply to each element in order.
    ///
    /// # Returns
    ///
    /// A new non-empty vector dequeue containing the results of applying `f` to each element.
    ///
    pub fn map<U, F>(mut vec_deque: NonEmptyVecDeque<U>, mut f: F) -> NonEmptyVecDeque<T>
    where
        F: FnMut(U) -> T,
    {
        let new_head: T = f(vec_deque.head);
        let new_tail: Option<VecDeque<T>> = vec_deque
            .tail
            .take()
            .map(|tail| tail.into_iter().map(&mut f).collect());
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
mod test;

//==================================================================================================
// Benchmarks
//==================================================================================================

#[cfg(feature = "std")]
#[cfg(test)]
#[path = "vec_deque/tests/benchmarks.rs"]
mod benchmarks;
