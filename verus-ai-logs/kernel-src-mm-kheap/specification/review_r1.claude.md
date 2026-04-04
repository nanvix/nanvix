# Specification Review: kheap (Claude Opus 4.6)

**Module**: `src/kernel/src/mm/kheap.rs`
**Spec file**: `src/kernel/src/mm/kheap.spec.rs`
**Proof file**: `src/kernel/src/mm/kheap.proof.rs`
**Dependency**: `src/libs/slab/src/lib.spec.rs` (SlabView, Slab contracts)
**Reviewer**: Claude Opus 4.6
**Date**: 2025-07-22

---

## Property Mapping

| Property ID | Status | Spec Location | Notes |
|-------------|--------|---------------|-------|
| **TYPE-1** | OK | `KheapView::inv()` spec.rs:169–173 | Slab count + per-slab `inv()`. Correctly mapped. |
| **TYPE-2** | OK | `KheapView::inv()` spec.rs:175–176 | `end_addr <= start_addr[i+1]` captures non-overlapping. Allows gaps between slabs, which is correct (Slab may not use the full partition). |
| **TYPE-3** | OK | `KheapView::inv()` spec.rs:178–179 | `block_size == block_sizes()[i]` with concrete `block_sizes()` sequence. The monotonicity property from the analysis is not stated explicitly but is trivially derivable from the concrete values in `block_sizes()`. |
| **TYPE-4** | UNMAPPED | — | **Heap storage containment is not an invariant.** Partially covered by `from_raw_parts` ensures (FN-2e) at construction time, but not maintained as a `KheapView` invariant. The `KheapView` struct does not contain a heap base address or bound, making it impossible to express this. |
| **TYPE-5** | UNMAPPED | — | SlabSize enum discriminant correctness has no Verus spec. Rust guarantees discriminant values, so this is compiler-enforced and low-risk. However, the bridge between `SlabSize::Slab8` and spec index 0 is never formally established. |
| **TYPE-6** | UNMAPPED | — | HeapStorage alignment is enforced by `#[repr(align(4096))]` and `static_assert::assert_eq_align!`, but not expressed as a Verus spec element. Adequate via static assertion. |
| **FN-1a** | OK | kheap.rs:335–336 | `Ok(_) => spec_slab_for_size(...).is_some()`. Forward direction. Combined with FN-1d's reverse, bidirectionality holds. |
| **FN-1b** | UNMAPPED | — | **The spec does not state that the returned slab tier is large enough for the request.** `Ok(_)` discards the variant entirely. A caller cannot derive `slab_size as usize >= layout.size()` from the current ensures. The ensures only says "some slab exists," not "the returned slab is sufficient." |
| **FN-1c** | UNMAPPED | — | **Tightest-fit is not specified.** No ensures clause states the returned slab is the *smallest* sufficient tier. While derivable from `spec_slab_for_size` definition (which maps to exact ranges), the spec doesn't expose this. Low priority — callers don't usually need this. |
| **FN-1d** | OK | kheap.rs:338 | `Err(_) => spec_slab_for_size(...).is_none()`. Correctly bidirectional with FN-1a. |
| **FN-2a** | UNMAPPED | — | Memory safety precondition — not expressible in Verus. Documented as acknowledged exclusion. Acceptable. |
| **FN-2b** | OK | kheap.rs:128 | `heap.inv()` on success. |
| **FN-2c** | OK | kheap.rs:130–131 | All slabs start with `allocated_addrs == Set::empty()`. |
| **FN-2d** | SUBSUMED | — | Block sizes match expected sequence. Already implied by `heap.inv()` (FN-2b), which includes TYPE-3 (`block_size == block_sizes()[i]`). No separate ensures needed. |
| **FN-2e** | OK | kheap.rs:133–136 | Slab containment within partitions. Uses `addr + i * slab_size` bounds. |
| **FN-2f** | OK | kheap.rs:144 | `e.code == ErrorCode::InvalidArgument`. |
| **FN-2g** | WRONG | kheap.rs:138–140, 146–148 | **Err ensures is too restrictive.** The spec claims: error implies at least one kheap-level check failed (`addr % PAGE_SIZE != 0 || size < MIN_HEAP_SIZE || size % MIN_HEAP_SIZE != 0`). But the function body propagates errors from inner `Slab::from_raw_parts` calls via `?`. If an inner slab construction fails while kheap-level checks pass, the Err ensures is violated. Correctness depends on LIVE-1 (all inner slab calls succeed when kheap checks pass), which is unproven. The spec should either: (a) prove LIVE-1 and document the dependency, or (b) weaken the Err ensures to `e.code == ErrorCode::InvalidArgument` only. |
| **FN-3a** | OK | kheap.rs:253 | `old(self).inv()` requires. |
| **FN-3b** | OK | kheap.rs:261–262 | Address was free in correct slab, gated by `opt_idx.is_some()`. |
| **FN-3c** | SUBSUMED | kheap.rs:264 | Block-alignment is implied by FN-3b (`ptr` was in `free_addrs`) + `SlabView::inv()` (free addresses are block-aligned). However, this is **useful for caller convenience** — a caller doesn't need to unfold `SlabView::inv()` to get alignment. Acceptable as a convenience ensures. |
| **FN-3d** | OK | kheap.rs:266 | Exact state transition via `spec_allocate`. Good frame condition — only target slab changes, others are preserved by `Seq::update`. |
| **FN-3e** | OK | kheap.rs:256 | Invariant preservation. |
| **FN-3f** | UNMAPPED | — | **Bidirectional error condition missing.** The spec says `Err(_) => self@ == old(self)@` but does not specify *when* errors occur: unsupported size (`spec_slab_for_size` returns None) or slab exhausted (free set is empty). A caller cannot reason about liveness without this. |
| **FN-3g** | OK | kheap.rs:269 | State preserved on error. |
| **FN-4a** | OK | kheap.rs:295 | `old(self).inv()` requires. |
| **FN-4b** | OK | kheap.rs:302–303 | Pointer was in allocated set of correct slab. |
| **FN-4c** | OK | kheap.rs:306 | Exact state transition via `spec_deallocate`. |
| **FN-4d** | OK | kheap.rs:298 | Invariant preservation. |
| **FN-4e** | UNMAPPED | — | **Bidirectional error condition missing.** Same issue as FN-3f: the Err ensures only says state preserved, not when errors occur (unsupported size OR pointer not in allocated set). |
| **FN-4f** | OK | kheap.rs:309 | State preserved on error. |
| **FN-5a** | UNMAPPED | — | `GlobalAlloc::alloc` is outside the `verus!{}` block. No contracts. |
| **FN-5b** | UNMAPPED | — | Same — HEAP-is-None path unspecified. |
| **FN-5c** | UNMAPPED | — | Same — allocation failure path unspecified. |
| **FN-6a** | UNMAPPED | — | `GlobalAlloc::dealloc` is outside the `verus!{}` block. No contracts. |
| **FN-6b** | UNMAPPED | — | Same — HEAP-is-None path unspecified. |
| **FN-6c** | UNMAPPED | — | Same — deallocation failure path unspecified. |
| **FN-7a** | UNMAPPED | — | `init()` is outside the `verus!{}` block. No contracts at all. |
| **FN-7b** | UNMAPPED | — | Same. |
| **FN-7c** | UNMAPPED | — | Same. |
| **FN-7d** | UNMAPPED | — | Same. |
| **MOD-1** | OK (stub) | proof.rs:13–14 | In `lemma_kheap_inv_implies_cross_slab_disjointness` ensures. Body is `admit()`. **Floating** — not connected to any exec contract. |
| **MOD-2** | OK (stub) | proof.rs:16–17 | Same lemma, same caveats. |
| **MOD-3** | OK (stub) | proof.rs:19–20 | Same lemma. Subsumes MOD-1 and MOD-2. |
| **MOD-4** | UNMAPPED | — | **No spec captures "no allocation at address zero."** Would require knowing `HEAP_STORAGE` has non-zero address (linker-dependent). Not expressible within the module. |
| **MOD-5** | OK (stub) | proof.rs:47–76 | Two lemmas (`lemma_allocate_conserves`, `lemma_deallocate_conserves`). Bodies are `admit()`. **Floating** — not connected to exec contracts. |
| **MOD-6** | TAUTOLOGICAL | — | Routing consistency: `layout_to_allocator` and `spec_slab_for_size` are both pure deterministic functions. Determinism is inherent. No verification value. |
| **MOD-7** | UNMAPPED | — | **Memory-region containment not an invariant.** Same issue as TYPE-4: `KheapView` lacks heap bounds, so this can't be expressed as a maintained invariant. |
| **LIVE-1** | UNMAPPED | — | Slab construction feasibility — no lemma or spec. Critical for FN-2g correctness. |
| **LIVE-2** | UNMAPPED | — | `init()` infallibility — no spec. |
| **LIVE-3** | UNMAPPED | — | Allocation succeeds when free blocks exist — not an explicit ensures. Partially implied by the contrapositive of a bidirectional error condition on `allocate`, but that condition is itself missing (FN-3f). |
| **LIVE-4** | UNMAPPED | — | Deallocation succeeds for allocated pointer — not an explicit ensures. Same issue. |
| **LIVE-5** | OK (stub) | proof.rs:36–45 | `lemma_alloc_dealloc_round_trip`. Body is `admit()`. **Floating** — not connected to exec contracts. |
| **LIVE-6** | SUBSUMED | — | Failure recoverability is implied by state preservation (FN-3g/FN-4f) + invariant preservation (FN-3e/FN-4d). If inv holds and state is unchanged, subsequent valid operations remain possible. No separate spec needed. |

### Property Mapping Summary

| Status | Count | Property IDs |
|--------|------:|-------------|
| OK | 20 | TYPE-1, TYPE-2, TYPE-3, FN-1a, FN-1d, FN-2b, FN-2c, FN-2e, FN-2f, FN-3a, FN-3b, FN-3d, FN-3e, FN-3g, FN-4a, FN-4b, FN-4c, FN-4d, FN-4f, FN-3c (convenience) |
| OK (stub) | 5 | MOD-1, MOD-2, MOD-3, MOD-5, LIVE-5 |
| UNMAPPED | 20 | TYPE-4, TYPE-5, TYPE-6, FN-1b, FN-1c, FN-2a, FN-3f, FN-4e, FN-5a–c, FN-6a–c, FN-7a–d, MOD-4, MOD-7, LIVE-1–4 |
| WRONG | 1 | FN-2g |
| SUBSUMED | 3 | FN-2d, LIVE-6, FN-3c |
| TAUTOLOGICAL | 1 | MOD-6 |

---

## Missing Properties

Properties the property analysis missed or that the spec should capture:

### MP-1: `layout_to_allocator` returns a slab tier sufficient for the request

`layout_to_allocator` ensures only says `spec_slab_for_size(...).is_some()`. It
discards the returned `SlabSize` variant entirely (`Ok(_)`). A caller cannot
prove that the returned tier's block size is ≥ `layout.size()`. The ensures
should name the return value and connect it to the spec index:

```
Ok(slab_size) => {
    &&& spec_slab_for_size(spec_layout_size(*layout) as int).is_some()
    &&& slab_size as usize >= spec_layout_size(*layout)
}
```

### MP-2: `allocate` / `deallocate` missing bidirectional error conditions

Both functions specify error-path state preservation but not error-path
*conditions*. A caller can't distinguish "unsupported size" from "slab
exhausted" (for allocate) or "unsupported size" from "not allocated" (for
deallocate). Add:

```rust
// allocate Err branch:
Err(_) => {
    &&& self@ == old(self)@
    &&& (spec_slab_for_size(spec_layout_size(layout) as int).is_none()
        || {
            let idx = spec_slab_for_size(spec_layout_size(layout) as int).unwrap();
            old(self)@.slabs[idx].free_addrs =~= Set::<usize>::empty()
        })
}
```

### MP-3: `KheapView` should track heap bounds

Adding `base_addr: int` and `bound_addr: int` to `KheapView` would enable
TYPE-4 (heap storage containment) and MOD-7 (all pointers within HEAP_STORAGE)
as maintained invariants. Without them, these properties are only asserted at
construction time and cannot be referenced by callers of allocate/deallocate.

### MP-4: LIVE-1 (slab construction feasibility) is critical for FN-2g soundness

The `from_raw_parts` Err ensures claims error implies kheap-level checks
failed. But inner `Slab::from_raw_parts` calls could fail independently. LIVE-1
argues they don't (given the constants), but this is unproven. Either:
- Add a lemma proving LIVE-1 (that the kheap-level checks imply all Slab
  preconditions hold), or
- Weaken FN-2g's Err ensures to not assert which checks failed.

### MP-5: Alignment is not verified

BUG-2 from the property analysis (alignment not checked in `layout_to_allocator`)
is not addressed in the spec. The allocate ensures says the returned pointer is
block-aligned (FN-3c), but doesn't relate this to `layout.align()`. A `Layout`
with `size=4, align=16` would get an 8-byte-aligned pointer, violating the
alignment requirement. Either:
- Add `requires layout.align() <= spec_layout_size(layout)` to allocate, or
- Route based on `max(size, align)`.

This requires an `assume_specification` for `Layout::align()` to express.

---

## Specs to Remove

### SR-1: MOD-6 (routing consistency) — TAUTOLOGICAL

`spec_slab_for_size` is a pure spec function. `layout_to_allocator` is a pure
exec function (no `&mut self`). Determinism is inherent to pure functions. No
spec or lemma is needed. Remove from the property list.

### SR-2: FN-2d — SUBSUMED by FN-2b

Block sizes matching the expected sequence is already part of `KheapView::inv()`
(TYPE-3), which is ensured by `heap.inv()` (FN-2b). No separate ensures needed.

### SR-3: LIVE-6 — SUBSUMED by FN-3e+FN-3g / FN-4d+FN-4f

Failure recoverability follows directly from invariant preservation + state
preservation on error. Not a separate property.

---

## Spec Quality Issues (highest priority first)

### SQ-1: CRITICAL — All exec bodies contain `proof { admit(); }` — nothing is verified

**Location**: kheap.rs lines 191, 272, 312, 341

All four exec functions inside `verus!{}` (`from_raw_parts`, `allocate`,
`deallocate`, `layout_to_allocator`) contain `proof { admit(); }` in their
bodies. This discharges ALL proof obligations, meaning every ensures clause is
**trusted, not verified**. The spec looks complete on paper but proves nothing
about the actual implementation.

**Impact**: A completely broken implementation would satisfy these specs. The
`admit()` in exec bodies is fundamentally different from `admit()` in proof
lemma stubs — proof stubs are internal helpers, but exec `admit()` means the
API contracts are unverified.

**Recommendation**: These must be removed during the proving phase. If they
remain in the final artifact, this is a desk reject. For a spec-phase review,
they are acceptable as placeholders, but should be tracked explicitly.

### SQ-2: HIGH — One-sided error specs on `allocate` and `deallocate`

**Location**: kheap.rs:269, kheap.rs:309

Both functions specify error paths as only `self@ == old(self)@`. This violates
the "bidirectional failure condition" principle: a caller can't reason about
*when* errors occur. Without error conditions:
- LIVE-3 (allocation succeeds when slab has free blocks) is unprovable
- LIVE-4 (deallocation succeeds for allocated pointer) is unprovable
- Callers can't prove their operations will succeed

**Fix**: Add bidirectional error conditions per MP-2 above.

### SQ-3: HIGH — FN-2g Err ensures is incorrect

**Location**: kheap.rs:146–148

The Err ensures claims error implies at least one kheap-level check failed. But
the function body propagates inner `Slab::from_raw_parts` errors via `?`. If an
inner slab call fails while kheap checks pass, the ensures is violated. This
would be caught during proving (the `admit()` currently hides it), but it's a
spec design error.

**Fix**: Either prove LIVE-1 as a prerequisite lemma and document the
dependency, or weaken the Err ensures.

### SQ-4: HIGH — `init()`, `GlobalAlloc::alloc`, `GlobalAlloc::dealloc` completely unspecified

**Location**: kheap.rs:363–403

Three functions are entirely outside the `verus!{}` block with no contracts:
- `init()` — the entry point that constructs the global heap
- `GlobalAlloc::alloc` — the actual allocator interface called by Rust runtime
- `GlobalAlloc::dealloc` — the actual deallocator interface

The verified `Kheap` methods are internal; the actual public interface is
unverified. This means the verification covers the core logic but not the
integration layer. The property analysis acknowledged this (FN-5/6/7 and the
"excluded properties" section), but the gap should be explicitly tracked.

**Recommendation**: At minimum, `init()` should be brought into a `verus!{}`
block with contracts. The `GlobalAlloc` trait impls are harder (trait method
constraints), but documenting the verification boundary is essential.

### SQ-5: MEDIUM — Floating proof lemmas not connected to exec contracts

**Location**: kheap.proof.rs:9–76

Five proof lemmas exist (`lemma_kheap_inv_implies_cross_slab_disjointness`,
`lemma_slab_for_size_valid`, `lemma_alloc_dealloc_round_trip`,
`lemma_allocate_conserves`, `lemma_deallocate_conserves`) but none is called
from any exec function's proof block. These are "floating specs" — they state
important properties but don't connect to the verified exec code path.

**Impact**: Even when `admit()` is removed from exec bodies, these lemmas would
not be automatically used. They'd need to be explicitly invoked in proof blocks.

**Recommendation**: During proving, ensure these lemmas are invoked where needed.
For the spec review, note that MOD-1/2/3/5 and LIVE-5 are only proven as
standalone lemmas, not as ensures on exec functions. Consider whether any should
be lifted into exec ensures (e.g., MOD-5 could be part of `allocate`'s ensures).

### SQ-6: MEDIUM — `spec_layout_size` is uninterpreted

**Location**: spec.rs:80

`spec_layout_size` is declared `uninterp spec fn`, making it opaque. This is
the correct approach for an external type (`Layout` is `external_body`), but it
has consequences:
- No caller can reason about specific size values (e.g., "Layout::from_size_align(8, 1) has size 8")
- `spec_slab_for_size(spec_layout_size(layout) as int)` is fully opaque to
  external callers who don't have a handle on the Layout's size

This is an inherent limitation of the opaque Layout type, not a spec error.
The mitigation is that callers would thread through the `spec_layout_size`
equality from `Layout::size()`'s ensures. Acceptable but worth documenting.

### SQ-7: LOW — `layout_to_allocator` discards the return value in its ensures

**Location**: kheap.rs:335 — `Ok(_) =>`

The `Ok` branch uses wildcard `_`, losing all information about the returned
`SlabSize` variant. While the current internal callers (`allocate`/`deallocate`)
match on the variant and can connect it to specific slabs within their own
proofs, the ensures is weaker than necessary.

**Fix**: Name the return value and add `slab_size as usize >= spec_layout_size(*layout)`:
```rust
Ok(slab_size) => {
    &&& spec_slab_for_size(spec_layout_size(*layout) as int).is_some()
    &&& slab_size as usize >= spec_layout_size(*layout)
}
```

---

## View Abstraction Assessment

### Strengths

1. **Clean separation**: `view()` is `pub closed spec fn` — callers can't see
   the field-to-index mapping. ✓
2. **Mathematical abstraction**: `Seq<SlabView>` enables quantified reasoning
   over all slabs without enumerating fields. ✓
3. **Spec transitions**: `spec_allocate` and `spec_deallocate` use
   `Seq::update` with `..self.slabs[idx]` for clean frame conditions. Only
   the target slab changes; all others are preserved implicitly. ✓
4. **Convenience functions**: `all_allocated()` and `all_free()` provide
   heap-wide aggregates using `Set::new` with existential quantifiers. ✓
5. **`ext_equal`**: Both `KheapView` and `SlabView` are marked
   `verifier::ext_equal`, enabling extensional equality reasoning. ✓

### Weaknesses

1. **Missing heap bounds**: `KheapView` lacks `base_addr` and `bound_addr`
   fields. This prevents TYPE-4 and MOD-7 from being invariant properties.
   Without these, a caller can't prove that returned pointers lie within the
   heap storage region.

2. **`all_allocated()` / `all_free()` are defined but never used**: These
   convenience specs appear in spec.rs but are not referenced by any ensures
   clause or lemma. They are dead spec code unless used in future proofs.

3. **No `spec_slab_for_layout` on KheapView**: The routing function
   `spec_slab_for_size` is a standalone spec, not a method on `KheapView`.
   This is acceptable (it depends only on the size, not the heap state), but
   means callers must import the free function separately.

### Transition Correctness

`spec_allocate(idx, addr)` correctly:
- Inserts `addr` into `slabs[idx].allocated_addrs`
- Removes `addr` from `slabs[idx].free_addrs`
- Preserves all other fields via `..self.slabs[idx]`
- Preserves all other slabs via `Seq::update` (only index `idx` changes)

`spec_deallocate(idx, addr)` correctly:
- Removes `addr` from `slabs[idx].allocated_addrs`
- Inserts `addr` into `slabs[idx].free_addrs`
- Same frame as above

Both transitions are **correct and clean**.

---

## assume_specification / Trust Boundary Assessment

### AS-1: `spec_layout_size` (uninterpreted) + `Layout::size()` spec

```rust
pub uninterp spec fn spec_layout_size(layout: Layout) -> usize;
pub assume_specification[ Layout::size ](layout: &Layout) -> (result: usize)
    ensures result == spec_layout_size(*layout);
```

**Verdict**: Sound. This is the standard pattern for opaque external types.
The uninterpreted function simply names the return value. No incorrect
assumptions are introduced. The ensures is a tautology by construction
(result equals itself under a name).

### AS-2: `Error::new` constructor

```rust
pub assume_specification[ Error::new ](code: ErrorCode, reason: &'static str) -> (result: Error)
    ensures result.code == code;
```

**Verdict**: Sound. Assumes that `Error::new` stores the provided error code.
This is a trivial property of a constructor. The `reason` field is not
constrained (logging-only), which is appropriate.

### AS-3: `usize_to_mut_ptr` (external_body)

```rust
#[verifier::external_body]
fn usize_to_mut_ptr(addr: usize) -> (result: *mut u8)
    ensures result as usize == addr;
```

**Verdict**: Sound but fragile. This is a workaround for Verus not supporting
`addr as *mut u8` casts. The ensures is correct for the cast semantics. The
`external_body` is justified because Verus cannot verify raw pointer casts.

**Risk**: This doesn't establish pointer provenance. The returned pointer has
no associated allocation in Verus's model. This is an inherent limitation
when working with raw pointers to static memory.

### AS-4: External type specifications

```rust
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLayout(Layout);
```

**Verdict**: Sound. `Layout` is treated as fully opaque (`external_body`),
which is conservative. `AllocError`, `Error`, `ErrorCode` are transparent,
which is correct for their simple structures.

### AS-5: Slab dependency assume_specifications (in lib.spec.rs)

The slab crate's spec file contains several `assume_specification` items for
pointer operations (`is_null`, `wrapping_add`, `add`, `offset_from_unsigned`,
pointer comparison operators) and one `axiom` (`align_of::<u8>() == 1`).

**Verdict**: All appear sound. The pointer arithmetic specs follow standard
semantics. The `wrapping_add` spec correctly uses modular arithmetic. The
`add` spec has appropriate overflow preconditions (`requires`). The axiom
about `u8` alignment is a language-level fact.

### Trust Boundary Summary

The trust boundary is **small and well-justified**:
- 3 assume_specifications in kheap (Layout::size, Error::new, usize_to_mut_ptr)
- 1 uninterpreted function (spec_layout_size)
- 4 external type specifications (Layout, AllocError, Error, ErrorCode)
- ~7 assume_specifications inherited from slab crate (pointer operations)
- 1 axiom inherited from slab crate (align_of::<u8>)

No assume_specification makes an incorrect or overly strong assumption.

---

## Anti-Pattern Flags

### AP-1: `proof { admit(); }` in exec function bodies (CRITICAL)

**Location**: kheap.rs:191, 272, 312, 341

Four exec functions contain `proof { admit(); }`. This is the most severe
anti-pattern — it makes ALL ensures clauses unverified trust assumptions.
Currently acceptable as spec-phase placeholders, but MUST be removed during
proving.

**Count**: 4 instances in exec code, 5 instances in proof stubs = 9 total.

### AP-2: No `admit()` / `assume()` outside placeholder positions

All `admit()` calls are either in exec proof blocks (placeholder) or proof
lemma bodies (stub). No `assume()` appears anywhere. No `admit()` is used
to paper over a specific difficult proof obligation within a larger proof.
This is clean — the admits are structural placeholders, not targeted cheats.

### AP-3: No unjustified `external_body`

The only `external_body` is `usize_to_mut_ptr` (justified: raw pointer cast)
and `ExLayout` (justified: opaque external type). No exec functions are marked
`external_body`.

### AP-4: No `trusted` functions

No functions are marked `trusted`. Good.

### AP-5: No loops in kheap, so no missing loop invariants

The kheap module has no loops. All iteration is done via match arms over
the fixed set of slab variants. N/A.

---

## Overall Assessment

- **Grade: C+**

### Key Strengths

1. **Well-designed View abstraction**: `KheapView` with `Seq<SlabView>` is
   clean, properly `closed`, and enables quantified reasoning. The spec
   transition functions (`spec_allocate`/`spec_deallocate`) have correct
   frame conditions.

2. **Strong specs on core methods**: `from_raw_parts`, `allocate`, and
   `deallocate` have detailed ensures clauses covering success transitions,
   invariant preservation, state preservation on error, and (for
   `from_raw_parts`) bidirectional error conditions.

3. **Small, well-justified trust boundary**: Only 3 kheap-specific
   `assume_specification` items, all sound and minimal. No unjustified
   verification escapes.

4. **Good property analysis**: The property analysis is thorough, identifying
   42 properties across 6 categories, plus 5 suspected bugs. The analysis
   correctly categorizes cross-module properties and excluded properties.

5. **Clean proof structure**: Proof stubs are clearly separated in proof.rs
   with descriptive names and correct requires/ensures. Ready for the
   proving phase.

### Key Weaknesses

1. **Nothing is actually verified**: All 4 exec functions have `proof { admit(); }`
   in their bodies. The ensures clauses are trusted, not proven. A buggy
   implementation would pass all checks. (Expected for spec phase, but must be
   resolved.)

2. **One-sided error specs**: `allocate` and `deallocate` specify error paths
   as only "state preserved" without stating *when* errors occur. This blocks
   caller-side liveness reasoning (LIVE-3, LIVE-4). The most impactful single
   improvement would be adding bidirectional error conditions.

3. **`from_raw_parts` Err ensures is incorrect**: Claims error implies
   kheap-level checks failed, but inner `Slab::from_raw_parts` errors can
   propagate when kheap checks pass. Depends on unproven LIVE-1.

4. **Public interface unspecified**: `init()`, `GlobalAlloc::alloc`, and
   `GlobalAlloc::dealloc` have no contracts. The actual allocator interface
   used by the rest of the kernel is unverified.

5. **20 of 49 properties unmapped**: Significant coverage gap. Most unmapped
   properties are either in the unspecified functions (FN-5/6/7) or are
   properties that `KheapView` can't express (TYPE-4, MOD-4, MOD-7) due to
   missing heap-bounds fields.

6. **Floating proof lemmas**: All 5 proof lemmas are standalone — not connected
   to any exec contract. They prove important properties (MOD-1/2/3/5, LIVE-5)
   but only as abstract theorems about `KheapView`, not about the actual
   implementation.

### Priority Fixes for Next Iteration

1. Add bidirectional error conditions to `allocate` and `deallocate` (SQ-2)
2. Fix `from_raw_parts` Err ensures or prove LIVE-1 (SQ-3)
3. Add heap bounds to `KheapView` for TYPE-4/MOD-7 (MP-3)
4. Strengthen `layout_to_allocator` ensures to name return value (SQ-7)
5. Bring `init()` into `verus!{}` with contracts (SQ-4)
6. Remove all exec `proof { admit(); }` during proving phase (SQ-1)
