# Minimized latest-Verus reproducers for the two virt-MM diagnostic categories

Verus: 0.2026.08.23.fbbbbcf (binary: /home/ruize/verus-exp/verus-main-20260824/source/target-verus/release/verus)
Date: 2026-08-24T11:52:28+08:00

Each reproducer is minimized legal Rust, run standalone (no Nanvix context).
Both reproduce the EXACT diagnostic observed in the probe run, confirming
genuine Verus frontend limitations rather than Nanvix-specific artifacts.

## CAT-static-mut  ->  genuine Verus frontend limitation
```rust
use vstd::prelude::*;

// Minimized legal Rust: a plain global `static mut`, annotated the way the probe
// injects it. Verus rejects `static mut` in its frontend.
#[verus_verify]
static mut COUNTER: u32 = 0;

fn main() {}
```
```
error: Verus does not support 'static mut'
 --> repro_static_mut_verusverify.rs:6:1
  |
6 | static mut COUNTER: u32 = 0;
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: aborting due to 1 previous error

```

## CAT-assoc-const-trait-impl  ->  genuine Verus frontend limitation
```rust
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
```
```
error[E0407]: method `VERUS_UNERASED_PROXY__N_UNITS` is not a member of trait `Bss`
  --> repro_assoc_const_impl.rs:18:5
   |
18 |     const N_UNITS: usize = 4;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^ not a member of trait `Bss`

error[E0407]: method `VERUS_UNERASED_PROXY__STORAGE_SIZE` is not a member of trait `Bss`
  --> repro_assoc_const_impl.rs:19:5
   |
19 |     const STORAGE_SIZE: usize = 4096;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ not a member of trait `Bss`

error: aborting due to 2 previous errors

For more information about this error, try `rustc --explain E0407`.
```

## Root-cause in Verus source (authoritative)
`builtin_macros/src/unerased_proxies.rs`:
- L79: "TODO: when we can generate unerased_proxy for associated const impls, we won't need this"
- L146: "allow associated const impls only if they are external_body"
