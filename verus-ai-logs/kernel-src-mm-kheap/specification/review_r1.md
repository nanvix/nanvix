# Specification Review: kheap

## Grade: C

Consolidated from independent reviews by Claude Opus 4.6 (grade: C+) and
GPT-5.3-Codex (grade: D). The spec has a well-designed View abstraction and
meaningful contracts on the core methods, but nothing is actually verified
(all exec bodies contain `proof { admit(); }`), error paths are one-sided,
three public functions are completely unspecified, and 20 of 49 properties
are unmapped.

---

## Property Mapping Issues

| Property ID | Status | Notes |
|-------------|--------|-------|
| TYPE-4 | UNMAPPED | Heap storage containment not expressible — `KheapView` lacks base/bound fields. Only partially covered at construction (FN-2e). |
| TYPE-5 | UNMAPPED | `SlabSize` enum discriminant correctness has no Verus spec. Compiler-enforced, low risk, but bridge to spec index never established. |
| TYPE-6 | UNMAPPED | `HeapStorage` page-alignment enforced by `#[repr(align(4096))]` and `static_assert!`, not by Verus. Adequate via static assertion. |
| FN-1b | UNMAPPED | `layout_to_allocator` ensures discards return value (`Ok(_)`). Callers cannot derive "returned slab block_size ≥ layout.size()". |
| FN-1c | UNMAPPED | Tightest-fit not specified. Derivable from `spec_slab_for_size` definition but not exposed in ensures. Low priority. |
| FN-2a | UNMAPPED | Safety/ownership precondition — not expressible in Verus. Documented exclusion. Acceptable. |
| FN-2d | SUBSUMED | Block sizes match expected sequence — already implied by `heap.inv()` (FN-2b) which includes TYPE-3. |
| FN-2g | WRONG | Err ensures claims error implies kheap-level checks failed. But inner `Slab::from_raw_parts` errors propagate via `?`. If kheap checks pass but an inner slab call fails, the ensures is violated. Depends on unproven LIVE-1. |
| FN-3c | SUBSUMED | Block-alignment implied by FN-3b + `SlabView::inv()`. Kept as caller convenience — acceptable but technically redundant. |
| FN-3f | UNMAPPED | Error causes not specified. Only state preservation on Err, no bidirectional condition for when errors occur (unsupported size vs. slab exhausted). |
| FN-4e | UNMAPPED | Same as FN-3f — error causes not specified for deallocate (unsupported size vs. ptr not allocated). |
| FN-5a–c | UNMAPPED | `GlobalAlloc::alloc` entirely outside `verus!{}`. No contracts. |
| FN-6a–c | UNMAPPED | `GlobalAlloc::dealloc` entirely outside `verus!{}`. No contracts. |
| FN-7a–d | UNMAPPED | `init()` entirely outside `verus!{}`. No contracts. |
| MOD-1 | SUBSUMED | Subsumed by MOD-3 (stronger: cross-slab disjointness of all set pairs). |
| MOD-2 | SUBSUMED | Subsumed by MOD-3. |
| MOD-4 | UNMAPPED | No spec excludes address 0 from slab sets. Requires knowing HEAP_STORAGE has non-zero address (linker-dependent). |
| MOD-6 | TAUTOLOGICAL | Routing determinism is inherent to pure functions (`spec_slab_for_size`, `layout_to_allocator`). No verification value. |
| MOD-7 | UNMAPPED | Memory-region containment not an invariant. `KheapView` lacks heap bounds. |
| LIVE-1 | UNMAPPED | Slab construction feasibility — critical for FN-2g soundness. No lemma or spec. |
| LIVE-2 | UNMAPPED | `init()` infallibility from static configuration — no spec. |
| LIVE-3 | UNMAPPED | Allocation succeeds when slab has free blocks — no liveness ensures. |
| LIVE-4 | UNMAPPED | Deallocation succeeds for allocated pointer — no liveness ensures. |
| LIVE-6 | SUBSUMED | Failure recoverability follows from FN-3e+FN-3g / FN-4d+FN-4f. |

Summary: 20 OK, 5 OK (stub with admit), 1 WRONG, 5 SUBSUMED, 1 TAUTOLOGICAL, 17 UNMAPPED.

---

## Missing Properties

1. **`layout_to_allocator` should name return value and guarantee sufficiency.**
   Ensures discards `Ok(_)`. Add: `Ok(slab_size) => slab_size as usize >= spec_layout_size(*layout)`.
   Without this, callers can't prove the returned slab is large enough.

2. **Bidirectional error conditions on `allocate`/`deallocate`.** Both specify
   error-path state preservation but not error-path *causes*. A caller can't
   distinguish "unsupported size" from "slab exhausted" (allocate) or "ptr not
   in allocated set" (deallocate). Blocks liveness reasoning (LIVE-3, LIVE-4).

3. **`KheapView` should include heap base/bound fields.** Without them, TYPE-4
   (heap storage containment) and MOD-7 (all pointers within HEAP_STORAGE) cannot
   be maintained as invariants. Only asserted at construction time currently.

4. **LIVE-1 must be proven for FN-2g soundness.** The `from_raw_parts` Err ensures
   claims error ↔ kheap-level checks failed, but inner Slab construction errors
   propagate independently. Either prove LIVE-1 or weaken FN-2g.

5. **Alignment not verified (BUG-2 from property analysis).** The spec never relates
   returned pointer alignment to `layout.align()`. A `Layout{size=4, align=16}`
   would get an 8-byte-aligned pointer, violating the requirement. Needs
   `assume_specification` for `Layout::align()` to express.

---

## Specs to Remove

- **MOD-6 (routing consistency)**: Tautological — determinism is inherent to pure
  functions. Remove from property goals.
- **FN-2d**: Subsumed by FN-2b + TYPE-3 (`heap.inv()` already ensures block sizes).
- **MOD-1/MOD-2 as separate goals**: Subsumed by MOD-3. Keep only MOD-3.
- **LIVE-6**: Subsumed by invariant preservation + state preservation on error.

---

## Issues (highest priority first)

### 1. CRITICAL — All exec bodies contain `proof { admit(); }` (kheap.rs:191,272,312,341)

All four exec functions inside `verus!{}` have `proof { admit(); }`, discharging
ALL proof obligations. Every ensures clause is **trusted, not verified**. A
completely broken implementation would satisfy these specs. This is the most
severe anti-pattern — acceptable only as a spec-phase placeholder, but must be
eliminated before any verification claims hold.

### 2. HIGH — One-sided error specs on `allocate` and `deallocate` (kheap.rs:269,309)

Error paths only specify `self@ == old(self)@`. Callers cannot reason about
*when* errors occur. Fix: add `Err(_) => spec_slab_for_size(...).is_none() || free_addrs == empty()` for allocate, and analogous condition for deallocate. This
is the single most impactful spec improvement.

### 3. HIGH — `from_raw_parts` Err ensures is incorrect (kheap.rs:146–148)

The Err ensures asserts error implies a kheap-level check failed. But the function
body propagates inner `Slab::from_raw_parts` errors via `?`. If inner slab
construction fails while kheap checks pass, the ensures is violated. Hidden by
`admit()`. Fix: prove LIVE-1 or weaken the Err ensures.

### 4. HIGH — `init()`, `GlobalAlloc::alloc/dealloc` completely unspecified (kheap.rs:363–403)

Three public functions are entirely outside `verus!{}` with no contracts. The
verified `Kheap` methods are internal; the actual allocator interface is
unverified. At minimum, `init()` should be brought into `verus!{}` with
contracts (HEAP == None precondition, success postcondition).

### 5. MEDIUM — Floating proof lemmas (kheap.proof.rs:9–76)

Five proof lemmas exist (MOD-1/2/3/5, LIVE-5) but none is called from any exec
function's proof block. They state important properties as standalone theorems
about `KheapView`, not about the actual implementation. Must be connected to exec
contracts during proving.

### 6. MEDIUM — `layout_to_allocator` discards return value (kheap.rs:335)

`Ok(_)` loses information about the returned `SlabSize`. A caller can't prove
the returned tier is sufficient for the request. Fix: name the return value and
add `slab_size as usize >= spec_layout_size(*layout)`.

### 7. LOW — `all_allocated()`/`all_free()` are dead spec code (kheap.spec.rs:183–194)

Defined but never referenced by any ensures clause or lemma. Either connect them
to contracts or remove to avoid confusion.

---

## Result: FAIL

The spec demonstrates competent View design and meaningful contracts on core
methods, but falls short of grade A due to: (a) nothing is verified — all exec
admits, (b) significant coverage gaps — 17 properties unmapped including all
public interface functions, (c) one-sided error specs block liveness reasoning,
and (d) one incorrect ensures clause (FN-2g). These must be addressed before
the specification can be considered complete.
