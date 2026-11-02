// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

extern crate std;

use super::{
    BssStorage,
    BumpAllocError,
    FixedSizeBumpAllocator,
};
use std::vec::Vec;

#[repr(align(8))]
struct StorageA {
    bytes: [u8; 16],
}

static mut STORAGE_A: StorageA = StorageA { bytes: [0; 16] };

struct BackendA;

unsafe impl BssStorage for BackendA {
    const NUM_UNITS: usize = 2;
    const STORAGE_SIZE: usize = 16;

    fn as_mut_ptr() -> *mut u8 {
        unsafe { core::ptr::addr_of_mut!(STORAGE_A.bytes) as *mut u8 }
    }
}

static ALLOC_A: FixedSizeBumpAllocator<8, 8, BackendA> = unsafe { FixedSizeBumpAllocator::new() };

#[test]
fn alloc_returns_distinct_slots() {
    let first: *mut u8 = ALLOC_A.alloc().expect("first alloc failed").as_mut_ptr();
    let second: *mut u8 = ALLOC_A.alloc().expect("second alloc failed").as_mut_ptr();
    assert_ne!(first, second);
}

#[repr(align(4))]
struct StorageB {
    bytes: [u8; 8],
}

static mut STORAGE_B: StorageB = StorageB { bytes: [0; 8] };

struct BackendB;

unsafe impl BssStorage for BackendB {
    const NUM_UNITS: usize = 1;
    const STORAGE_SIZE: usize = 8;

    fn as_mut_ptr() -> *mut u8 {
        unsafe { core::ptr::addr_of_mut!(STORAGE_B.bytes) as *mut u8 }
    }
}

static ALLOC_B: FixedSizeBumpAllocator<8, 4, BackendB> = unsafe { FixedSizeBumpAllocator::new() };

#[test]
fn alloc_as_allows_typed_access() {
    let slot: &mut core::mem::MaybeUninit<[u32; 2]> =
        unsafe { ALLOC_B.alloc_as::<[u32; 2]>().expect("alloc_as failed") };
    slot.write([7, 11]);
    let typed: &mut [u32; 2] = unsafe { slot.assume_init_mut() };
    let values: Vec<u32> = Vec::from(*typed);
    assert_eq!(values, vec![7, 11]);
}

#[repr(align(8))]
struct StorageC {
    bytes: [u8; 8],
}

static mut STORAGE_C: StorageC = StorageC { bytes: [0; 8] };

struct BackendC;

unsafe impl BssStorage for BackendC {
    const NUM_UNITS: usize = 1;
    const STORAGE_SIZE: usize = 8;

    fn as_mut_ptr() -> *mut u8 {
        unsafe { core::ptr::addr_of_mut!(STORAGE_C.bytes) as *mut u8 }
    }
}

static ALLOC_C: FixedSizeBumpAllocator<8, 8, BackendC> = unsafe { FixedSizeBumpAllocator::new() };

#[test]
fn alloc_returns_exhausted_error() {
    let _ = ALLOC_C.alloc().expect("first alloc failed");
    let result = ALLOC_C.alloc();
    assert_eq!(result.unwrap_err(), BumpAllocError::Exhausted);
}
