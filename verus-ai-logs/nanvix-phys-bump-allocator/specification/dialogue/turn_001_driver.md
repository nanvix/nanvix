## Turn 1: Tautological ensures (`Err(_) => true`)

### Progress
- Done (PASS):
  1. In-scope exec coverage — `align_up`, `alloc`, `alloc_as`, `as_mut_ptr` all carry
     `requires`/`ensures`. `fn_coverage` reports 3/6 with the 3 unverified being
     `fmt`, `new`, `default`, all out of the verification scope declared in
     `caller_analysis.md`.
  2. Caller coverage — every expectation in `caller_analysis.md` maps to spec/proof:
     size/align gating + alignment + in-bounds in the `alloc`/`alloc_as` `ensures`;
     uniqueness/in-bounds/alignment over the whole pool in `geometry_ok` +
     `lemma_geometry`; exhaustion boundary in `lemma_exhausted_boundary`; monotone
     advance in `lemma_alloc_transition`. (Cross-call uniqueness is intentionally
     deferred to the proving phase per the documented atomic-ghost-token limitation.)
  3. View consistency — `alloc`/`alloc_as` require `bump_view(self).inv()` and state
     guarantees over `v.base`, `v.unit_align`, `v.storage_size`; matches `view_design.md`.
  5. No subsumed ensures — geometry guarantees are over the uninterpreted
     `slot_ref_addr(slot)`, not derivable from `inv()` alone.
  7. No workspace-internal `assume_specification` — the only one is on
     `<usize>::div_ceil` (std), documented as not-yet-in-vstd.
  8. vstd searched — confirmed no `div_ceil` spec in the vstd tree before assuming.
  9. Specs written for the caller — addresses, size/align match are directly usable.
  10. Trait obligations — `BssStorage::as_mut_ptr` ensures `result as int == base_of`.
  12. Loop invariants — the only loop (CAS retry) is inside an `external_body`
      function whose body Verus does not verify; no `invariant` clause required.
  13. Cheating counts — `assume=0 admit=0 trusted=0`; `external_body=2`
      (`alloc` L286, `alloc_as` L367), BOTH individually registered in
      `verus-ai-logs/tcb-allowed.md` with the raw-pointer-materialization rationale
      (mirrors `raw-array`). No admit/assume in the proof bodies (the L6 comment is
      stale). Accepted as TCB-registered external dependencies.
  14. No spec weakened — `caller_analysis.md` confirms the prior specs were empty
      (clean slate); nothing to weaken.
  15. Bug awareness — `bugs.md` present; no code bugs (confirmed: under `inv()`,
      `alloc`'s overflow/bounds/misalign paths are unreachable — see below).
  16. Cross-module regression — `make verify` ran all crates: bitmap, sys,
      nanvix-slab, bump-allocator, kernel — every one Exit code 0.
  17. Verification + build — `make verify-bump-allocator` exit 0; `cargo test`
      compiles and passes 3 unit tests + 1 doc-test.

- Current: **Item 4 — No tautological ensures** (also drives Item 6).
- Remaining: 11 (spec-completeness advisory).

### Verification
`grep` of `ensures` on the in-scope functions shows a tautological catch-all in
both trusted (`external_body`) functions:

`lib.rs:283` (`alloc`):
```
Err(_) => true,
```
`lib.rs:364` (`alloc_as`):
```
Err(_) => true,
```

Item 4 explicitly names `Err(_) => true` as the canonical tautology to reject, and
item 6 requires meaningful error ensures. For `alloc` ALL four error variants
(`Exhausted`, `Overflow`, `OutOfBounds`, `Misaligned`) collapse to `true`. For
`alloc_as` the `SizeMismatch`/`AlignmentMismatch` arms are meaningful, but the
residual `Err(_) => true` is still tautological.

This is fixable with a true, caller-relevant statement. Under `bump_view(self).inv()`
the only reachable error of `alloc` is `Exhausted`:
- `stride = align_up(N,A) = align_up_spec(unit_size,unit_align) = Some(stride)` ⇒ no
  `Overflow` from the stride step.
- `idx < NUM_UNITS = capacity`, and `inv()` gives `capacity*stride <= storage_size`
  and `base + storage_size <= usize::MAX+1` ⇒ `base+offset+N <= base+capacity*stride
  <= base+storage_size`: no add `Overflow`, and `end <= storage_end` ⇒ no `OutOfBounds`.
- `base % unit_align == 0` and `stride % unit_align == 0` ⇒ `ptr % A == 0`: no
  `Misaligned`.

So `Exhausted` is the only error a caller can observe under the invariant — which is
exactly the "graceful exhaustion" guarantee callers depend on. Because both
functions are `external_body` (trusted), tightening their `ensures` to this true
statement strengthens (does not weaken) the contract and is sound to assert.

### Fix Request
Make the error arms meaningful (do NOT justify in prose — change the spec):

1. In `lib.rs`, `alloc` `#[verus_spec]` (around L283), replace:
   ```
   Err(_) => true,
   ```
   with:
   ```
   Err(e) => e == BumpAllocError::Exhausted,
   ```

2. In `lib.rs`, `alloc_as` `#[verus_spec]` (around L364), replace the catch-all:
   ```
   Err(_) => true,
   ```
   with the set of reachable propagated/guard errors:
   ```
   Err(e) => e == BumpAllocError::SizeMismatch
       || e == BumpAllocError::AlignmentMismatch
       || e == BumpAllocError::Exhausted,
   ```
   (Keep the existing `Err(BumpAllocError::SizeMismatch) => ...` and
   `Err(BumpAllocError::AlignmentMismatch) => ...` arms; only the trailing
   `Err(_) => true` changes.)

Note: the `match` must remain exhaustive. If the compiler complains about
overlapping/unreachable arms once `Err(_)` is replaced by `Err(e)`, fold the two
specific `Err(BumpAllocError::SizeMismatch|AlignmentMismatch)` arms into the single
`Err(e) => ...` arm using `&&&` implications, e.g.:
```
Err(e) => {
    &&& e == BumpAllocError::SizeMismatch
        || e == BumpAllocError::AlignmentMismatch
        || e == BumpAllocError::Exhausted
    &&& (e == BumpAllocError::SizeMismatch ==> vstd::layout::size_of::<T>() != N as nat)
    &&& (e == BumpAllocError::AlignmentMismatch ==> vstd::layout::align_of::<T>() > A as nat)
},
```

Verify with:
- `make verify-bump-allocator` → expect Exit code 0, coverage unchanged (3/6 with
  only `fmt`/`new`/`default` unverified), no new errors.
- `cargo test` in `src/libs/bump_allocator` → 3 unit tests + doc-test still pass.

Do not create the STOP file: item 4 is an open FAIL and item 11 (spec-completeness,
advisory) has not been run.
