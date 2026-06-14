## Response to Turn 1

### What I Changed

Replaced the tautological `Err(_) => true` catch-all arms in both trusted
(`external_body`) functions with meaningful, caller-relevant error postconditions.
File: `src/libs/bump_allocator/src/lib.rs`.

1. **`FixedSizeBumpAllocator::alloc`** (`#[verus_spec]` error arm, ~L283):
   ```diff
   -                Err(_) => true,
   +                Err(e) => e == BumpAllocError::Exhausted,
   ```
   Under `bump_view(self).inv()`, `Exhausted` is the only reachable error:
   `stride == align_up_spec(unit_size, unit_align)` rules out the stride
   `Overflow`; `idx < capacity` with `capacity*stride <= storage_size` and
   `base + storage_size <= usize::MAX + 1` rule out the address-arithmetic
   `Overflow` and `OutOfBounds`; `base % unit_align == 0 && stride % unit_align == 0`
   rule out `Misaligned`. This is exactly the "graceful exhaustion" guarantee
   callers depend on.

2. **`FixedSizeBumpAllocator::alloc_as<T>`** (`#[verus_spec]` error arms, ~L362):
   Folded the two specific `Err(BumpAllocError::SizeMismatch|AlignmentMismatch)`
   arms plus the trailing `Err(_) => true` into a single exhaustive `Err(e)` arm
   (the reviewer's suggested `&&&`-implication form) to keep the match exhaustive
   without overlapping arms:
   ```diff
   -                Err(BumpAllocError::SizeMismatch) => vstd::layout::size_of::<T>() != N as nat,
   -                Err(BumpAllocError::AlignmentMismatch) => vstd::layout::align_of::<T>() > A as nat,
   -                Err(_) => true,
   +                Err(e) => {
   +                    &&& e == BumpAllocError::SizeMismatch
   +                        || e == BumpAllocError::AlignmentMismatch
   +                        || e == BumpAllocError::Exhausted
   +                    &&& (e == BumpAllocError::SizeMismatch ==> vstd::layout::size_of::<T>()
   +                        != N as nat)
   +                    &&& (e == BumpAllocError::AlignmentMismatch ==> vstd::layout::align_of::<T>()
   +                        > A as nat)
   +                },
   ```
   The reachable errors of `alloc_as` are its two type-gate guards
   (`SizeMismatch`, `AlignmentMismatch`) plus the propagated `Exhausted` from
   `alloc`; the bidirectional guard facts are preserved as implications.

No exec logic, signatures, or struct definitions were touched — only the
`ensures` text inside the two `#[verus_spec]` attributes.

### Verification

`make verify-bump-allocator`:
```
verification results:: 6 verified, 0 errors
  6 verified
  0 errors
  Exit code : 0
  cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 3/6 exec functions have contracts
```
- Exit code 0, 0 errors, coverage unchanged (3/6 — only `fmt`/`new`/`default`
  unverified, all out of scope). `external_body=2` are the TCB-registered
  `alloc`/`alloc_as`; `assume=admit=trusted=no_decreases=cfg_gate=0`.

`./z build -- check-guest-rlibs` (dual compilation — Verus attrs erase under
normal `cargo`):
```
Checking bump-allocator v0.16.17 (/home/ruize/nanvix-phy/src/libs/bump_allocator)
{"reason":"build-finished","success":true}
[OK]    Build complete.
```

`cargo test` in `src/libs/bump_allocator`:
```
running 3 tests
test result: ok. 3 passed; 0 failed; ...
running 1 test    (doc-test)
test result: ok. 1 passed; 0 failed; ...
```

### Result: FIXED

Both tautological `Err(_) => true` arms are gone. `alloc` now guarantees the only
observable error under the invariant is `Exhausted`; `alloc_as` constrains its
errors to `{SizeMismatch, AlignmentMismatch, Exhausted}` with the bidirectional
size/alignment guard facts retained. Verification passes (0 errors), dual build
compiles, and all unit + doc tests pass.
