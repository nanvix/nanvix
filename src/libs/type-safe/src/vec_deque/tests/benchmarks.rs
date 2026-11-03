// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
