# Specification Review: kheap (GPT-5.3-Codex)

## Property Mapping
| Property ID | Status | Notes |
| --- | --- | --- |
| TYPE-1 | OK | `KheapView::inv()` enforces slab count and per-slab `SlabView::inv()` (`kheap.spec.rs`). |
| TYPE-2 | OK | `KheapView::inv()` enforces ordered non-overlap (`end_addr <= next.start_addr`). |
| TYPE-3 | OK | `KheapView::inv()` enforces `block_size == block_sizes()[i]` sequence. |
| TYPE-4 | UNMAPPED | No kheap-level spec ties all slab addresses to `HEAP_STORAGE` bounds. |
| TYPE-5 | UNMAPPED | No Verus contract/lemma states enum discriminants are correct. |
| TYPE-6 | UNMAPPED | Alignment exists in exec (`repr(align)`, static assert), not bound into Verus contract. |
| FN-1a | OK | `layout_to_allocator` ensures `Ok => spec_slab_for_size(size).is_some()` and `Err => is_none()`, giving IFF. |
| FN-1b | WRONG | Contract does not constrain returned `SlabSize`; a non-minimal/larger tier could satisfy ensures. |
| FN-1c | WRONG | “Smallest fitting slab” is not specified. |
| FN-1d | OK | Error condition is bidirectional via dual ensures clauses. |
| FN-2a | UNMAPPED | Safety ownership/valid-region precondition is only documented text, not `requires`. |
| FN-2b | OK | `from_raw_parts` success ensures `heap.inv()`. |
| FN-2c | OK | Success ensures all `allocated_addrs` are empty. |
| FN-2d | SUBSUMED | Implied by FN-2b + TYPE-3 (`heap.inv()`). |
| FN-2e | OK | Success ensures each slab lies in its partition slice. |
| FN-2f | OK | Error ensures `e.code == InvalidArgument`. |
| FN-2g | OK | Contract encodes kheap-level checks as bidirectional error/success gate. |
| FN-3a | OK | `allocate` requires `old(self).inv()`. |
| FN-3b | OK | Success binds pointer to free set of selected slab index. |
| FN-3c | OK | Success guarantees block alignment. |
| FN-3d | OK | Success gives exact abstract transition via `spec_allocate`. |
| FN-3e | OK | Invariant preservation explicitly ensured. |
| FN-3f | WRONG | Error causes are not specified (only state preservation on error). |
| FN-3g | OK | Error branch ensures full state preservation. |
| FN-4a | OK | `deallocate` requires `old(self).inv()`. |
| FN-4b | OK | Success requires pointer was allocated in routed slab. |
| FN-4c | OK | Success gives exact abstract transition via `spec_deallocate`. |
| FN-4d | OK | Invariant preservation explicitly ensured. |
| FN-4e | WRONG | Error causes are not specified (missing unsupported-size / not-allocated characterization). |
| FN-4f | OK | Error branch ensures full state preservation. |
| FN-5a | UNMAPPED | `GlobalAlloc::alloc` has no Verus contract. |
| FN-5b | UNMAPPED | `GlobalAlloc::alloc` HEAP-None behavior not specified in Verus. |
| FN-5c | UNMAPPED | `GlobalAlloc::alloc` failure/null behavior not specified in Verus. |
| FN-6a | UNMAPPED | `GlobalAlloc::dealloc` has no Verus contract. |
| FN-6b | UNMAPPED | HEAP-None no-op behavior not specified in Verus. |
| FN-6c | UNMAPPED | Failure-preserves-state wrapper behavior not specified in Verus. |
| FN-7a | UNMAPPED | `init` precondition (`HEAP == None`) is not specified. |
| FN-7b | UNMAPPED | `init` success postcondition on `HEAP`/`kheap.inv()` is not specified. |
| FN-7c | UNMAPPED | `init` backing-by-`HEAP_STORAGE` postcondition is not specified. |
| FN-7d | UNMAPPED | `init` error/frame behavior is not specified. |
| MOD-1 | SUBSUMED | Included as part of stronger MOD-3 lemma guarantee; not needed separately. |
| MOD-2 | SUBSUMED | Included as part of stronger MOD-3 lemma guarantee; not needed separately. |
| MOD-3 | OK | Stated in `lemma_kheap_inv_implies_cross_slab_disjointness` (but currently `admit()`). |
| MOD-4 | WRONG | No spec excludes address 0 from slab sets; not derivable from current invariants. |
| MOD-5 | OK | Mapped to `lemma_allocate_conserves`/`lemma_deallocate_conserves` (both currently `admit()`). |
| MOD-6 | OK | Routing determinism captured operationally by `layout_to_allocator` + shared use in alloc/dealloc. |
| MOD-7 | UNMAPPED | No contract states all allocated addresses are within `HEAP_STORAGE` bounds. |
| LIVE-1 | UNMAPPED | No proved lemma/contract that inner slab constructors always succeed under kheap checks. |
| LIVE-2 | UNMAPPED | `init` infallibility is not specified/proved. |
| LIVE-3 | UNMAPPED | No liveness contract: free block implies `allocate` succeeds. |
| LIVE-4 | UNMAPPED | No liveness contract: allocated pointer implies `deallocate` succeeds. |
| LIVE-5 | OK | Mapped to `lemma_alloc_dealloc_round_trip` (currently `admit()`). |
| LIVE-6 | SUBSUMED | High-level recoverability follows from FN-3g/FN-4f frame-on-error clauses. |

## Missing Properties
- `layout.align()`-aware routing (currently only size-based; can violate alignment for exotic layouts).
- Explicit `init` contract (`HEAP == None` precondition, success/error frame, HEAP_STORAGE binding).
- `GlobalAlloc` wrapper contracts (null on failure/uninitialized, delegation/frame).
- Strong liveness (`free != ∅ => allocate Ok`, `allocated contains ptr => deallocate Ok`).
- Module-level containment (`all_allocated ⊆ HEAP_STORAGE range`).

## Specs to Remove
- Separate MOD-1/MOD-2 as top-level goals (keep MOD-3 only, mention derivation).
- Redundant FN-2d if FN-2b + TYPE-3 remain the canonical statement.

## Spec Quality Issues (highest priority first)
- `admit()` appears in exec bodies (`from_raw_parts`, `layout_to_allocator`, `allocate`, `deallocate`) and proof lemmas; this is a verification escape in trusted paths.
- `init`, `GlobalAlloc::alloc`, `GlobalAlloc::dealloc` are effectively unverified (outside `verus!` contracts).
- Error-path specs are incomplete for FN-3/FN-4 (missing bidirectional error causes).
- `layout_to_allocator` spec is too weak to guarantee minimal/tight slab choice (FN-1b/FN-1c).
- No explicit frame/spec for global mutable state (`HEAP`) transitions.

## View Abstraction Assessment
- `KheapView` cleanly abstracts implementation to slab sequence + set semantics.
- It still leaks structural layout (fixed ordering/count of concrete slab tiers), which is acceptable for this allocator design.
- `spec_allocate/spec_deallocate` transitions are appropriate and caller-usable.
- However, abstraction is underpowered for module-level guarantees (no explicit heap-base/bounds in view-level invariant).

## assume_specification / Trust Boundary Assessment
- `Layout::size` as uninterpreted (`spec_layout_size`) is acceptable for opacity, but too weak for alignment-sensitive obligations.
- `Error::new` assumption (code field) is narrow and reasonable.
- `usize_to_mut_ptr` with `external_body` is plausible as a cast shim, but remains trusted.
- Main trust risk is not these assumptions; it is the `admit()` in executable proof blocks.

## Anti-Pattern Flags
- `admit()` in proof stubs: acceptable only as temporary scaffolding.
- `admit()` in exec-function proof blocks: high-risk; should be eliminated before trusting verification claims.
- `external_body` present (`usize_to_mut_ptr`): justified only if unavoidable and minimal.
- No loop-invariant issues observed (no loops in verified portion).

## Overall Assessment
- Grade: D
- Key strengths: good abstract state model (`KheapView`), clean success-state transitions for allocate/deallocate, partial bidirectional routing check.
- Key weaknesses: major coverage gaps (FN-5/FN-6/FN-7, TYPE-4/5/6, liveness), weak routing precision (FN-1b/c), and heavy reliance on `admit()` in both proof and exec-adjacent verification.
