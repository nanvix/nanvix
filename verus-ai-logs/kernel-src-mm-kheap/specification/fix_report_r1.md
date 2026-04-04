# Fix Report: kheap Specification Review R1

## Summary

Addressed 7 issues from `review_r1.md`. Three spec changes were made (Issues 2, 3, 6);
four issues required no code change (Issues 1, 4, 5, 7). All verification passes:
15 verified, 0 errors across bitmap, slab, and kernel crates. Full build (`./z build -- all`) clean.

---

## Issue 1: CRITICAL — All exec bodies contain `proof { admit(); }`

**Classification: (D) Reviewer wrong**

The review acknowledges these are "acceptable only as a spec-phase placeholder".
This IS the spec phase — the task is to write and fix specifications, not prove them.
The admits are expected placeholders that will be eliminated during the proof phase.
No change needed.

---

## Issue 2: HIGH — One-sided error specs on `allocate` and `deallocate`

**Classification: (B) Spec missing — fixed**

**Problem**: Error paths only specified `self@ == old(self)@` (state preservation),
with no information about *when* errors occur. Callers couldn't reason about
liveness (LIVE-3, LIVE-4).

**Fix**: Added bidirectional error conditions:

- **`allocate` Err**: `opt_idx.is_none() || old(self)@.slabs[opt_idx.unwrap()].free_addrs == Set::<usize>::empty()`
  — error iff size is unsupported or the matching slab is exhausted.

- **`deallocate` Err**: `opt_idx.is_none() || !old(self)@.slabs[opt_idx.unwrap()].allocated_addrs.contains(ptr as usize)`
  — error iff size is unsupported or pointer was not allocated in that slab.

These match the Slab-level error conditions (Slab::allocate errors when free_addrs
is empty; Slab::deallocate errors when ptr is not in allocated_addrs) composed with
layout_to_allocator routing. Expressed at the abstract level using KheapView state.

---

## Issue 3: HIGH — `from_raw_parts` Err ensures is incorrect (FN-2g)

**Classification: (A) Spec too strong — weakened**

**Problem**: The Err ensures claimed error ↔ kheap-level check failed (bidirectional).
But the function body propagates inner `Slab::from_raw_parts` errors via `?`. If
kheap-level checks pass but an inner slab constructor fails, the bidirectional
ensures would be violated.

**Fix**: Removed the reverse direction of FN-2g from the Err branch. The Err
ensures now only specifies `e.code == ErrorCode::InvalidArgument` (FN-2f). The
forward direction (success implies preconditions held) remains on the Ok branch.

**Justification**: Without proving LIVE-1 (slab construction feasibility — which
requires reasoning about concrete constant arithmetic), we cannot guarantee that
inner slab calls succeed when kheap checks pass. The weakened spec is still useful:
callers know the error code, and the Ok-branch's forward direction establishes that
success implies valid inputs.

---

## Issue 4: HIGH — `init()`, `GlobalAlloc::alloc/dealloc` unspecified

**Classification: (E) Verus limitation**

**Problem**: Three public functions have no contracts.

**Analysis**: These functions access `static mut HEAP` and `HEAP_STORAGE` through
raw pointer operations (`ptr::addr_of_mut!`), and use macros (`info!`, `error!`)
that Verus cannot parse. Verus does not support reasoning about global mutable
state (`static mut`) — there is no permission/ownership model for it.

The core verified methods (`Kheap::allocate`, `Kheap::deallocate`,
`Kheap::from_raw_parts`, `Kheap::layout_to_allocator`) are the semantically
meaningful functions. The `GlobalAlloc` impl and `init()` are thin wrappers
that delegate to these verified methods. Verification of the core layer provides
the essential guarantees; the wrapper layer would only add that `HEAP` is
`Some`/`None` at the right times (GLOBAL-2), which is a cross-module boot ordering
property outside kheap's scope.

---

## Issue 5: MEDIUM — Floating proof lemmas

**Classification: (D) Reviewer wrong**

The proof lemmas (`lemma_kheap_inv_implies_cross_slab_disjointness`,
`lemma_slab_for_size_valid`, `lemma_alloc_dealloc_round_trip`,
`lemma_allocate_conserves`, `lemma_deallocate_conserves`) are a proof library
for the proof phase. They state properties about `KheapView` that will be
invoked from exec proof blocks once admits are replaced. They are not dead code —
they are the proof infrastructure that the proving phase will connect.

---

## Issue 6: MEDIUM — `layout_to_allocator` discards return value

**Classification: (B) Spec missing — fixed**

**Problem**: `Ok(_)` lost information about the returned `SlabSize`. Callers
couldn't prove the returned tier is sufficient for the request.

**Fix**: Added FN-1b as an ensures clause on the Ok branch, expressed through
the abstract model:
```
block_sizes()[opt_idx.unwrap()] >= spec_layout_size(*layout) as int
```
where `opt_idx = spec_slab_for_size(spec_layout_size(*layout) as int)`.

Note: We kept `Ok(_)` rather than `Ok(slab_size)` because `SlabSize` is
module-private and Verus requires ensures patterns in public functions to use
only publicly visible types. The sufficiency guarantee is expressed through
`block_sizes()` and `spec_slab_for_size()` which are public spec functions —
this is actually a better spec design (abstract level, not concrete type).

---

## Issue 7: LOW — `all_allocated()`/`all_free()` dead spec code

**Classification: (D) Reviewer wrong**

These are spec convenience functions on `KheapView` that compute the union of
allocated/free addresses across all slabs. They are referenced by the proof
lemmas (MOD-3 cross-slab disjointness), will be used in future specs (MOD-4
no-null-allocation, MOD-7 region containment), and serve as the caller-facing
abstraction for global heap state. They are part of the View design, not dead code.

---

## Verification Results

```
bitmap:  clean (10/10 exec functions)
slab:    35 verified, 0 errors (3/3 exec functions)
kernel:  15 verified, 0 errors (4/7 exec functions with contracts)
build:   ./z build -- all — clean
```

Unverified kernel functions (`alloc`, `dealloc`, `init`) are outside `verus!{}`
due to Verus limitations with `static mut` and macros (Issue 4).
