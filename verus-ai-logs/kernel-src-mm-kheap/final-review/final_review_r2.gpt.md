# Independent Review: kheap (GPT-5.3-Codex)

## Spec Quality Assessment
- `layout_to_allocator` (kheap.rs:366-401): strong external-top spec. `Err(_) <=> unsupported size` is explicit and readable; tightest-fit clause avoids over-weak routing specs.
- `allocate` (266-314) and `deallocate` (317-363): strong contracts with invariant preservation, explicit error-state frame (`self@ == old(self)@`), and exact abstract transition (`spec_allocate`/`spec_deallocate`).
- `from_raw_parts` (122-263): success branch is good (inv, empty allocated sets, partition containment). Error branch is still one-sided (`Err => InvalidArgument`) and does **not** encode bidirectional kheap-level failure (FN-2g). Fix report’s claim that this is recovered by contrapositive is logically incorrect for that direction.
- Readability: generally good; properties are named in comments and map to abstractions (`KheapView`, `spec_slab_for_size`, `block_sizes`).
- Anti-pattern check (spec-design):
  - No code-as-spec or tautologies in the 4 verified functions.
  - No missing error-frame in `allocate`/`deallocate`.
  - Remaining weakness: one-sided error information in `from_raw_parts`.

## Property Coverage
Mapping each property ID to concrete spec/proof elements:

### TYPE
- **TYPE-1**: Covered by `KheapView::inv()` (kheap.spec.rs:198-204), used by `heap.inv()` postconditions.
- **TYPE-2**: Covered by `KheapView::inv()` (205-207).
- **TYPE-3**: Covered by `KheapView::inv()` (208-210) + `block_sizes()`; strengthened by `lemma_block_sizes_strictly_increasing` (kheap.proof.rs:178-184).
- **TYPE-4**: **Partial**. Constructor partition containment in `from_raw_parts` ensures (kheap.rs:138-141), but no verified `init` contract tying heap to `HEAP_STORAGE`.
- **TYPE-5**: Uncovered in Verus specs (compiler/layout guarantee only).
- **TYPE-6**: Uncovered in Verus specs (repr/alignment + static assert, outside Verus model).

### FN
- **FN-1a**: Covered (`Ok => opt_idx.is_some()`, kheap.rs:371-373).
- **FN-1b**: Covered (`block_sizes[idx] >= size`, 373-375).
- **FN-1c**: Covered (`idx == spec_slab_size_to_index(ss)`, 375-377; tightest fit 377-380).
- **FN-1d**: Covered (`Err <=> unsupported`, 381-382).

- **FN-2a**: Uncovered (documented safety requirement only in property_analysis, not formalized in code).
- **FN-2b**: Covered (`heap.inv()`, 132-134).
- **FN-2c**: Covered (empty allocated sets, 135-137).
- **FN-2d**: Covered indirectly via `heap.inv()` + TYPE-3 (`KheapView::inv` block-size equality).
- **FN-2e**: Covered (partition containment, 138-141).
- **FN-2f**: Covered (`Err => e.code == InvalidArgument`, 147-150).
- **FN-2g**: **Partial**: only `Ok => checks` (142-145), not full bidirectional error condition.

- **FN-3a**: Covered (requires `old(self).inv()`, 267-270).
- **FN-3b**: Covered (returned ptr was free in routed slab, 277-279).
- **FN-3c**: Covered (alignment, 279-281).
- **FN-3d**: Covered (exact transition, 281-283).
- **FN-3e**: Covered (`self.inv()`, 271-273).
- **FN-3f**: Covered (`Err => unsupported or exhausted`, 288-292).
- **FN-3g**: Covered (`Err => self@ unchanged`, 286-288).

- **FN-4a**: Covered (requires `old(self).inv()`, 318-321).
- **FN-4b**: Covered (ptr was allocated in routed slab, 327-330).
- **FN-4c**: Covered (exact transition, 330-332).
- **FN-4d**: Covered (`self.inv()`, 322-324).
- **FN-4e**: Covered (`Err => unsupported or ptr not allocated`, 337-341).
- **FN-4f**: Covered (`Err => self@ unchanged`, 335-337).

- **FN-5a/FN-5b/FN-5c**: Uncovered (no Verus contract on `GlobalAlloc::alloc`, kheap.rs:406-421).
- **FN-6a/FN-6b/FN-6c**: Uncovered (no Verus contract on `GlobalAlloc::dealloc`, 423-431).
- **FN-7a/FN-7b/FN-7c/FN-7d**: Uncovered (no Verus contract on `init`, 437-446).

### MOD
- **MOD-1**: Covered by lemma `lemma_kheap_inv_implies_cross_slab_disjointness` (kheap.proof.rs:33-39).
- **MOD-2**: Covered by same lemma (40-41).
- **MOD-3**: Covered by same lemma (43-45).
- **MOD-4**: **Partial** via conditional `lemma_no_null_address` (199-223); not linked to exec contracts and depends on extra precondition `base_addr > 0`.
- **MOD-5**: Covered by `lemma_allocate_conserves` (104-130) and `lemma_deallocate_conserves` (133-158).
- **MOD-6**: Covered by deterministic routing contract in `layout_to_allocator` and both wrappers using it.
- **MOD-7**: **Partial**. Region bounds come from constructor/Slab invariants, but no verified `init` contract tying allocator state to `HEAP_STORAGE` globally.

### LIVE
- **LIVE-1**: Uncovered (no proof each inner `Slab::from_raw_parts` must succeed under kheap checks).
- **LIVE-2**: Uncovered (init wrapper unverified; relies on static-mut/global reasoning).
- **LIVE-3**: Covered (from FN-3f + Result exhaustiveness: supported+nonempty implies not Err, hence Ok).
- **LIVE-4**: Covered (from FN-4e + Result exhaustiveness).
- **LIVE-5**: Covered (`lemma_alloc_dealloc_round_trip`, 78-100).
- **LIVE-6**: Covered (FN-3g + FN-4f state preservation on failure).

### GLOBAL
- **GLOBAL-1..GLOBAL-5**: Uncovered in-module (architectural/cross-module assumptions; documented in property_analysis, not machine-checked here).

## Proof Completeness
- `admit()` count in reviewed files (`kheap.rs`, `kheap.spec.rs`, `kheap.proof.rs`): **0**.
- No remaining `admit()` locations.

## Trust Boundary Audit
### assume_specification
1. `Layout::size` (kheap.spec.rs:83-85)
   - Assumes: `result == spec_layout_size(*layout)`.
   - Needed Assumptions status: **Approved** (`[x]`, property_analysis.md:720).
   - Minimality/correctness: minimal accessor model; acceptable.
2. `Error::new` (88-91)
   - Assumes: constructed error preserves `code`.
   - Needed Assumptions status: **Approved** (`[x]`, 719).
   - Minimality/correctness: minimal (does not over-constrain `reason`); acceptable.

### external_body
1. `ExLayout` external type spec body (59-61)
   - Assumes opacity of foreign `Layout` representation.
   - Needed Assumptions status: not listed explicitly as a checklist item; standard external type pattern.
   - Minimality/correctness: acceptable for opaque std type.
2. `usize_to_mut_ptr` (95-100)
   - Assumes: `result as usize == addr` for cast helper.
   - Needed Assumptions status: **Approved** (`[x]`, 727).
   - Minimality/correctness: minimal and matches helper body.

### axiom
- No module-defined `axiom` declarations in reviewed files.
- Note: module uses `group_control_flow_axioms` from vstd in proof blocks (kheap.rs:193,296,345); this is library trust, not a new local axiom.

## Previous Issue Resolution
1. **Issue 1 (unverified wrappers)**: **Confirmed unresolved**. `alloc`/`dealloc`/`init` remain uncontracted and unverified; classification as Verus limitation is plausible.
2. **Issue 2 (MOD-4 no null)**: **Partially addressed**. `lemma_no_null_address` exists and verifies, but is conditional (`base_addr > 0`) and not connected to any exec function contract. MOD-4 remains not fully established at API level.
3. **Issue 3 (LIVE-1/LIVE-2)**: **Confirmed unresolved**. Still no machine-checked proofs/contracts for these liveness claims.
4. **Issue 4 (Err underspecified)**: **Not resolved**. `from_raw_parts` Err branch still only specifies error code. Fix report’s “contrapositive recovers Err => bad checks” argument is incorrect; contrapositive of `Ok => P` yields `!P => !Ok`, not `Err => !P`.
5. **Issue 5 (usize_to_mut_ptr doc)**: **Resolved**. Added to Needed Assumptions as approved `[x]`.
6. **Issue 6 (TYPE-5/TYPE-6 no change)**: **Confirmed unchanged**. Still not formalized in Verus; treated as compiler/runtime guarantees.

## Issues Found
1. **Medium**: `GlobalAlloc::alloc`, `GlobalAlloc::dealloc`, and `init` still lack Verus contracts (coverage gap for FN-5*, FN-6*, FN-7*).
2. **Low**: `from_raw_parts` error contract remains one-sided (FN-2g only partial), reducing caller-usable failure reasoning.
3. **Low**: MOD-4 remains conditional and unbound to external-top contracts (lemma exists but does not close the property).
4. **Low**: LIVE-1/LIVE-2 remain non-machine-checked.

## Overall Assessment
**Grade: B.** Core verified functions have strong, mostly clean contracts and zero `admit()`. However, this re-review still finds unresolved external-top coverage gaps (unverified wrappers), partially specified constructor failure behavior, and unresolved MOD-4/LIVE-1/LIVE-2 proof obligations. Fixes improved documentation and added a useful conditional lemma, but did not fully close the previously identified specification/completeness gaps.
