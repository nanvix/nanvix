# Fix Report R2: kheap Final Review Issues

**Module**: `mm::kheap`
**Baseline**: 20 verified, 0 errors
**After fixes**: 21 verified, 0 errors
**Regressions**: None (`make verify` passes clean)

---

## Issue 1 — [Medium] 3 unverified wrapper functions

**Classification: (E) Verus limitation**

`GlobalAlloc::alloc`, `GlobalAlloc::dealloc`, and `init()` access `static mut HEAP`
and `static mut HEAP_STORAGE`. Verus cannot verify code that accesses `static mut`
globals — this is a well-known limitation (no ownership model for mutable statics).

Per the verus-constraints skill, partial contracts are preferred over none.
However, these functions cannot be moved inside `verus! {}` because their bodies
consist primarily of `static mut` accesses (`ptr::addr_of_mut!(HEAP)`,
`HEAP = Some(...)`, `HEAP_STORAGE.memory.as_ptr()`). Cfg-gating sub-expressions
would require cfg-gating the entire function body, which defeats the purpose.
Adding `#[verifier::external]` is banned by policy.

The core allocator logic they call is fully verified (4 functions, 21 verified items).

**Action**: None possible without Verus `static mut` support.

---

## Issue 2 — [Low] LIVE-1/LIVE-2 not machine-checked

**Classification: (B) Conditional lemma added + (E) Verus limitation for LIVE-2**

**Correction**: The R1 fix report incorrectly stated that the Slab spec is not
bidirectional. In fact, the Slab `from_raw_parts` Err ensures clause lists ALL
error conditions as a disjunction:

```
Err(e) => {
    ||| addr == 0 ||| len == 0 ||| len >= i32::MAX ||| len > isize::MAX
    ||| addr + len > usize::MAX ||| block_size == 0 ||| block_size >= i32::MAX
    ||| block_size > (usize::MAX-1)/8 ||| len < block_size*2
    ||| addr % block_size != 0
}
```

By contrapositive: `¬(any error condition) ⟹ ¬Err ⟹ Ok`. This enables
proving LIVE-1 by showing all conditions are false for our parameters.

**Fix**: Added `lemma_slab_construction_feasible` to `kheap.proof.rs` — a
machine-checked proof that for the standard `init()` parameters
(`size = MIN_HEAP_SIZE`), every Slab error condition is negated for every slab
index. The lemma:

- Takes `base_addr` (positive, page-aligned) and `slab_idx` (0..NUM_OF_SLABS)
- Proves all 10 Slab error conditions are false for each slab
- Uses case-splitting on slab index for alignment (PAGE_SIZE divisibility)
- Uses vstd arithmetic lemmas (`lemma_mod_mod`, `lemma_mul_mod_noop_right`,
  `lemma_add_mod_noop`, `lemma_div_is_ordered`, `lemma_div_multiples_vanish`)
  for modular transitivity and integer division bounds

**Remaining architecture assumptions** (cannot be eliminated without axioms):
- `base_addr > 0`: HEAP_STORAGE is a static at linker-assigned non-zero address
- `MIN_HEAP_SIZE <= isize::MAX`: true on ≥32-bit platforms (917504 < 2^31-1)
- `usize::MAX >= 8 * max_slab_size() + 1`: true on ≥16-bit platforms

**LIVE-2** (`init()` infallibility) remains unverifiable: it accesses `static mut`
and the concrete `HEAP_STORAGE.memory.len() == MIN_HEAP_SIZE` relationship cannot
be expressed in Verus. Combined with LIVE-1 (now partially proven), LIVE-2 follows
by informal reasoning.

---

## Issue 3 — [Low] FN-2g Err branch partial

**Classification: (D) Reviewer acknowledges this is the strongest provable statement**

The `from_raw_parts` Err branch specifies `e.code == ErrorCode::InvalidArgument`.
The full bidirectional condition (`Err ⟹ ¬(kheap checks pass)`) is not provable
because:

1. **addr = 0 gap**: The kheap alignment check `addr % PAGE_SIZE == 0` passes
   for `addr = 0`, but Slab rejects null pointers. So `from_raw_parts` can
   return Err even when all three kheap-level checks pass.

2. **Large size gap**: On 64-bit platforms with `size` close to `isize::MAX`,
   `slab_size = size / NUM_OF_SLABS` could exceed `i32::MAX`, which Slab rejects.
   This is only avoidable if the kheap requires additionally constrained `size`.

The Ok branch's forward implications (FN-2g forward) already provide the useful
direction: `Ok ⟹ (aligned ∧ sufficient ∧ divisible)`. The contrapositive
`¬(aligned ∧ sufficient ∧ divisible) ⟹ Err` is recoverable by Result
exhaustiveness.

**Action**: No change. Current spec is correct and strongest provable.

---

## Issue 4 — [Info] TYPE-5, TYPE-6 not formalized

**Classification: (D) Reviewer acknowledges negligible risk**

- **TYPE-5** (enum discriminant correctness): Compiler guarantee for integer-repr
  enums. Not expressible in Verus without axioms.
- **TYPE-6** (struct alignment): Enforced by `#[repr(align(4096))]` and
  `static_assert::assert_eq_align!`. Compile-time guarantee.

**Action**: No change needed.

---

## Summary

| Issue | Severity | Classification | Action |
|-------|----------|---------------|--------|
| 1. Unverified wrappers | Medium | (E) Verus limitation | None possible |
| 2. LIVE-1/LIVE-2 | Low | (B) + (E) | Added conditional LIVE-1 lemma |
| 3. FN-2g Err partial | Low | (D) Acknowledged | None needed |
| 4. TYPE-5/TYPE-6 | Info | (D) Acknowledged | None needed |

**Verification**: 21 verified, 0 errors (was 20). +1 from `lemma_slab_construction_feasible`.
**Regressions**: None.
**Cheating**: assume=0, external_body=2, admit=0, trusted=0 (unchanged).
