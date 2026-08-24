use vstd::prelude::*;

// External (not verus-processed) trait with an associated const, mirroring
// bump_allocator::BssStorage.
trait Bss {
    const N_UNITS: usize;
    const STORAGE_SIZE: usize;
    fn as_mut_ptr() -> *mut u8;
}

struct S;

// The probe injects `#[verus_verify]` on this impl. Verus generates
// `VERUS_UNERASED_PROXY__*` proxy members for the associated consts, which the
// external trait does not declare -> rustc E0407.
#[verus_verify]
unsafe impl Bss for S {
    const N_UNITS: usize = 4;
    const STORAGE_SIZE: usize = 4096;
    fn as_mut_ptr() -> *mut u8 { core::ptr::null_mut() }
}

fn main() {}
