# Final Review: kheap (Claude Opus 4.6)

## 1. Spec Quality

### Overall Assessment: Strong

The external-top specs for the four verified functions (`layout_to_allocator`, `from_raw_parts`, `allocate`, `deallocate`) are well-written and follow spec-design principles closely.

### Detailed Analysis

**`Kheap::layout_to_allocator` (FN-1)**
- **Correctness**: ✅ Bidirectional — Ok iff `spec_slab_for_size` returns Some, Err iff None.
- **Completeness**: ✅ Covers size sufficiency (FN-1b), tightest-fit (FN-1c strengthened), and error iff unsupported (FN-1d).
- **Error path**: ✅ Bidirectional — `Err(_) => spec_slab_for_size(...).is_none()` is an iff condition.
- **Frame**: ✅ Pure function, no state mutation.
- **Declarative**: ✅ Uses `spec_slab_for_size` and `block_sizes()` mathematical abstractions.
- **Caller-oriented**: ✅ Caller learns the exact index and size guarantee.

**`Kheap::from_raw_parts` (FN-2)**
- **Correctness**: ✅ Ok ensures `heap.inv()`, all slabs empty, each slab contained within partition.
- **Completeness**: ✅ Covers invariant (FN-2b), empty start (FN-2c), containment (FN-2e), error code (FN-2f).
- **Error path**: ✅ Bidirectional via contrapositive — Ok ensures `addr % PAGE_SIZE == 0 ∧ size >= MIN_HEAP_SIZE ∧ size % MIN_HEAP_SIZE == 0`; if any fails, result cannot be Ok.
- **State preservation on error**: N/A (constructor, no pre-existing state).
- **Declarative**: ✅ Uses `KheapView` abstraction.
- **Note**: The Err branch only specifies `e.code == ErrorCode::InvalidArgument` without repeating the negated conditions. This is acceptable because the Ok branch's forward implications make the failure conditions logically deducible, but explicitly stating `Err ⟺ ¬preconditions` would be more readable.

**`Kheap::allocate` (FN-3)**
- **Correctness**: ✅ Ok ensures address was free, block-aligned, exact state transition via `spec_allocate`.
- **Completeness**: ✅ Full contract with invariant preservation (FN-3e).
- **Error path**: ✅ Bidirectional — `Err` iff size unsupported or slab exhausted (FN-3f). State preserved (FN-3g).
- **State preservation on error**: ✅ `self@ == old(self)@`.
- **Frame condition**: ✅ Exact state transition via `spec_allocate` — only the target slab changes, all others preserved by `Seq::update`.
- **Declarative**: ✅ Uses `KheapView::spec_allocate`, `spec_slab_for_size`.

**`Kheap::deallocate` (FN-4)**
- **Correctness**: ✅ Symmetric to allocate. Ok ensures ptr was allocated, exact transition via `spec_deallocate`.
- **Completeness**: ✅ Full contract with invariant preservation (FN-4d).
- **Error path**: ✅ Bidirectional — `Err` iff size unsupported or ptr not in allocated set (FN-4e). State preserved (FN-4f).
- **State preservation on error**: ✅ `self@ == old(self)@`.
- **Frame condition**: ✅ Exact via `spec_deallocate`.

### Anti-Pattern Check

| Anti-Pattern | Status |
|---|---|
| Exec Code Mutation | ✅ None — all deviations are cfg-gated, preserving original exec path |
| Verification Escape (admit/assume/external_body) | ⚠️ 2 assume_specification + 1 external_body helper — all justified (see §4) |
| Missing Loop Specs | ✅ N/A — no loops in kheap (slab construction is sequential) |
| Code-as-Spec | ✅ Specs use mathematical `KheapView` abstraction, not mirroring code |
| One-Sided Error Spec | ✅ All error paths are bidirectional |
| Missing Frame Condition | ✅ All state transitions are exact (`spec_allocate`/`spec_deallocate`) |
| Tautological Postconditions | ✅ None detected — all postconditions are substantive |
| Over-specification | ✅ Specs are appropriately detailed without constraining implementation unnecessarily |

### View Abstraction Quality

`KheapView` is well-designed:
- Uses `Seq<SlabView>` indexed by tier, enabling quantified properties.
- `spec_allocate`/`spec_deallocate` cleanly model state transitions.
- `all_allocated()`/`all_free()` provide global aggregation specs.
- `inv()` captures TYPE-1, TYPE-2, TYPE-3 compositionally.
- `KheapView` is appropriately `ext_equal`, enabling extensional equality reasoning in proofs.

### Weaknesses

1. **FN-2 error branch could be more explicit**: The Ok-branch forward conditions make Err conditions deducible, but stating `Err ⟹ ¬(addr aligned ∧ size sufficient ∧ size multiple)` directly in the Err branch would improve readability. Minor issue.
2. **`spec_layout_size` is uninterpreted**: Callers cannot reason about concrete Layout sizes (e.g., `Layout::from_size_align(64, 8)` → size = 64). This is inherent to the trust boundary — Layout is opaque. Acceptable.

---

## 2. Property Coverage

### Summary: Covered 33 / 59 total (33 / 40 in-scope verifiable)

### Per-Property Status

#### Type Invariants (TYPE-1 through TYPE-6)

| ID | Description | Status | Location |
|---|---|---|---|
| TYPE-1 | KheapView well-formedness | ✅ COVERED | `kheap.spec.rs:198-203` — `KheapView::inv()` |
| TYPE-2 | Slab region disjointness | ✅ COVERED | `kheap.spec.rs:205-206` — consecutive ordering |
| TYPE-3 | Block-size sequence | ✅ COVERED | `kheap.spec.rs:208-209` — `block_sizes()[i]` |
| TYPE-4 | Heap storage containment | ⚠️ PARTIAL | Via FN-2e (partition containment). End-to-end link to `HEAP_STORAGE` requires `init()` verification |
| TYPE-5 | SlabSize enum discriminants | ❌ NOT COVERED | Implicitly correct by Rust enum definition; not formalized in Verus |
| TYPE-6 | HeapStorage alignment | ❌ NOT COVERED | Established by `#[repr(align(4096))]` + `static_assert!`; not expressible in Verus |

#### Function Contracts

| ID | Description | Status | Location |
|---|---|---|---|
| FN-1a | Size supported → Ok | ✅ COVERED | `kheap.rs:371` + FN-1d bidirectional |
| FN-1b | Slab large enough | ✅ COVERED | `kheap.rs:374` |
| FN-1c | Tightest fit | ✅ COVERED | `kheap.rs:376-379` (strengthened with forall) |
| FN-1d | Error iff unsupported | ✅ COVERED | `kheap.rs:382` |
| FN-2b | heap.inv() on success | ✅ COVERED | `kheap.rs:133` |
| FN-2c | All slabs empty | ✅ COVERED | `kheap.rs:135-136` |
| FN-2d | Block sizes match | ✅ COVERED | Via FN-2b + TYPE-3 in `inv()` |
| FN-2e | Slab containment | ✅ COVERED | `kheap.rs:138-141` |
| FN-2f | Error code | ✅ COVERED | `kheap.rs:149` |
| FN-2g | Bidirectional failure | ✅ COVERED | `kheap.rs:143-145` (forward in Ok branch; contrapositive gives reverse) |
| FN-3b | Address was free | ✅ COVERED | `kheap.rs:278` |
| FN-3c | Block-aligned | ✅ COVERED | `kheap.rs:280` |
| FN-3d | Exact state transition | ✅ COVERED | `kheap.rs:282` |
| FN-3e | Invariant preserved | ✅ COVERED | `kheap.rs:272` |
| FN-3f | Error iff unsupported/exhausted | ✅ COVERED | `kheap.rs:289-291` |
| FN-3g | State preserved on error | ✅ COVERED | `kheap.rs:287` |
| FN-4b | Ptr was allocated | ✅ COVERED | `kheap.rs:329` |
| FN-4c | Exact state transition | ✅ COVERED | `kheap.rs:331` |
| FN-4d | Invariant preserved | ✅ COVERED | `kheap.rs:322` |
| FN-4e | Error iff unsupported/not-allocated | ✅ COVERED | `kheap.rs:338-340` |
| FN-4f | State preserved on error | ✅ COVERED | `kheap.rs:336` |
| FN-5a | alloc success | ❌ NOT COVERED | `ArenaAllocator::alloc` outside `verus!{}` block |
| FN-5b | alloc when HEAP None | ❌ NOT COVERED | Same — unverified wrapper |
| FN-5c | alloc failure | ❌ NOT COVERED | Same |
| FN-6a | dealloc success | ❌ NOT COVERED | `ArenaAllocator::dealloc` outside `verus!{}` block |
| FN-6b | dealloc when HEAP None | ❌ NOT COVERED | Same |
| FN-6c | dealloc failure | ❌ NOT COVERED | Same |
| FN-7b | init success | ❌ NOT COVERED | `init()` outside `verus!{}` block |
| FN-7c | init heap backing | ❌ NOT COVERED | Same |
| FN-7d | init error | ❌ NOT COVERED | Same |

#### Module-Level Safety (MOD-1 through MOD-7)

| ID | Description | Status | Location |
|---|---|---|---|
| MOD-1 | Cross-slab allocated disjoint | ✅ COVERED | `kheap.proof.rs:37-38` |
| MOD-2 | Cross-slab free disjoint | ✅ COVERED | `kheap.proof.rs:40-41` |
| MOD-3 | Full cross-disjointness | ✅ COVERED | `kheap.proof.rs:43-44` |
| MOD-4 | No allocation at addr 0 | ❌ NOT COVERED | No proof addresses this |
| MOD-5 | Allocation conservation | ✅ COVERED | `kheap.proof.rs:104-158` (both directions) |
| MOD-6 | Routing consistency | ✅ COVERED | `layout_to_allocator` is pure/deterministic |
| MOD-7 | Memory-region containment | ⚠️ PARTIAL | Via FN-2e + SlabView::inv() range bounds; end-to-end chain to HEAP_STORAGE incomplete |

#### Liveness (LIVE-1 through LIVE-6)

| ID | Description | Status | Location |
|---|---|---|---|
| LIVE-1 | Slab construction feasibility | ❌ NOT COVERED | Argued informally in property_analysis; no Verus proof |
| LIVE-2 | init() infallibility | ❌ NOT COVERED | Requires init() verification |
| LIVE-3 | Alloc succeeds when free | ✅ COVERED | Contrapositive of FN-3f |
| LIVE-4 | Dealloc succeeds when allocated | ✅ COVERED | Contrapositive of FN-4e |
| LIVE-5 | Alloc-dealloc round trip | ✅ COVERED | `kheap.proof.rs:78-100` |
| LIVE-6 | Failure recoverability | ✅ COVERED | Via FN-3g + FN-4f |

#### Cross-Module (GLOBAL-1 through GLOBAL-5)

| ID | Description | Status | Reason |
|---|---|---|---|
| GLOBAL-1 | Heap memory exclusivity | ❌ NOT COVERED | Cross-module architectural — by design |
| GLOBAL-2 | Single initialization | ❌ NOT COVERED | Caller responsibility — by design |
| GLOBAL-3 | No concurrent access | ❌ NOT COVERED | Architectural — by design |
| GLOBAL-4 | Layout consistency | ❌ NOT COVERED | Caller responsibility — by design |
| GLOBAL-5 | Architecture-constant coupling | ❌ NOT COVERED | Build-system — by design |

#### Suspected Bugs (BUG-1 through BUG-5)

| ID | Description | Status |
|---|---|---|
| BUG-1 | Double initialization leak | ⚠️ DOCUMENTED | Acknowledged; mitigated by single call site |
| BUG-2 | Alignment not checked | ⚠️ DOCUMENTED | Acknowledged; practical impact low |
| BUG-3 | dealloc silently ignores errors | ⚠️ DOCUMENTED | Inherent to GlobalAlloc trait |
| BUG-4 | Zero-sized layout handling | ⚠️ DOCUMENTED | Implementation-defined per GlobalAlloc |
| BUG-5 | Data-race risk on static mut | ⚠️ DOCUMENTED | Architectural concern (GLOBAL-3) |

### Coverage Breakdown

- **Fully covered**: 33 properties
- **Partially covered**: 2 (TYPE-4, MOD-7)
- **Not covered, in-scope**: 5 (TYPE-5, TYPE-6, MOD-4, LIVE-1, LIVE-2)
- **Not covered, out-of-scope by design**: 14 (FN-5*, FN-6*, FN-7*, GLOBAL-*)
- **Documented observations (bugs)**: 5
- **Total**: 59

The 33/40 coverage rate for in-scope verifiable properties is good. The 3 unverified functions (`alloc`, `dealloc`, `init`) are thin wrappers around verified core logic and use `unsafe` global state that Verus cannot model. This is a reasonable verification boundary.

---

## 3. Proof Completeness

- **admit() count: 0**
- **Locations**: None
- **Assessment**: Clean — no verification escapes via admit.

The proof file (`kheap.proof.rs`) contains 7 proof functions, all fully verified:

| Proof | Property Proved |
|---|---|
| `lemma_regions_ordered` | Transitive ordering across non-consecutive slabs |
| `lemma_kheap_inv_implies_cross_slab_disjointness` | MOD-1, MOD-2, MOD-3 |
| `lemma_slab_for_size_valid` | spec_slab_for_size maps to valid index |
| `lemma_alloc_dealloc_round_trip` | LIVE-5 |
| `lemma_allocate_conserves` | MOD-5 (allocation direction) |
| `lemma_deallocate_conserves` | MOD-5 (deallocation direction) |
| `lemma_slab_for_size_tightest_fit` | FN-1c strengthened |
| `lemma_block_sizes_strictly_increasing` | TYPE-3 strengthened |
| `lemma_slab_for_size_total` | Totality over supported range |

All proofs verify cleanly with no admits. This is excellent.

---

## 4. Trust Boundary Audit

### assume_specification count: 2

#### 1. `Layout::size` (kheap.spec.rs:83-84)

```rust
pub assume_specification[ Layout::size ](layout: &Layout) -> (result: usize)
    ensures result == spec_layout_size(*layout),
;
```

- **What it assumes**: `Layout::size()` returns a value consistent with the uninterpreted function `spec_layout_size`.
- **Human-approved**: ✅ Yes — marked `[x]` in property_analysis.md §9 Needed Assumptions.
- **Ensures minimal**: ✅ Minimal — introduces an uninterpreted function, does not over-constrain.
- **Correctness**: ✅ This is the standard pattern for opaque accessor specs.

#### 2. `Error::new` (kheap.spec.rs:88-91)

```rust
pub assume_specification[ Error::new ](code: ErrorCode, reason: &'static str) -> (result: Error)
    ensures result.code == code,
;
```

- **What it assumes**: `Error::new(code, reason)` produces an Error with matching code.
- **Human-approved**: ✅ Yes — marked `[x]` in property_analysis.md.
- **Ensures minimal**: ✅ Only constrains the `.code` field; does not constrain `reason` or other fields.
- **Correctness**: ✅ Matches Error constructor semantics.

### external_body count: 2

#### 1. `ExLayout` (kheap.spec.rs:59-61)

```rust
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLayout(Layout);
```

- **What it assumes**: `Layout` is an opaque type with no Verus-visible fields.
- **Standard pattern**: ✅ This is the prescribed approach for `external_type_specification` on opaque foreign types.
- **Risk**: Minimal — no logical properties are assumed about Layout's internals.

#### 2. `usize_to_mut_ptr` (kheap.spec.rs:95-100)

```rust
#[verifier::external_body]
fn usize_to_mut_ptr(addr: usize) -> (result: *mut u8)
    ensures result as usize == addr,
{ addr as *mut u8 }
```

- **What it assumes**: `addr as *mut u8` produces a pointer whose address equals `addr`.
- **Human-approved**: ⚠️ Not explicitly listed in Needed Assumptions, but is a cfg-gated Verus workaround for `addr as *mut u8` cast (which Verus cannot verify). The property_analysis Human note acknowledges the need for pointer cast support.
- **Ensures minimal**: ✅ Only constrains address equality.
- **Correctness**: ✅ `usize as *mut u8` preserves the integer value — this is guaranteed by Rust's semantics.
- **Risk**: Negligible — universally true property of Rust's pointer casts.

### axiom count: 0 (custom)

The `broadcast use vstd::std_specs::control_flow::group_control_flow_axioms` calls (3 locations in kheap.rs) reference vstd's built-in axiom group for `?` operator control flow, not custom axioms.

### Unapproved Items

| Item | Status | Blocker? |
|---|---|---|
| `usize_to_mut_ptr` external_body | Not explicitly in Needed Assumptions list | **No** — trivially correct Verus workaround; ensures are minimal and universally valid |

**No blockers found.** All trust assumptions are either human-approved or trivially sound workarounds for Verus limitations.

---

## 5. Exec Fidelity

### AST check result: PASS (with documented deviations)

**Tool output** (`ast_consistency.py` with `dev` branch as baseline):

```
Functions:
  ArenaAllocator::alloc       MATCH       UNVERIFIED
  ArenaAllocator::dealloc     MATCH       UNVERIFIED
  Kheap::allocate             MISMATCH    VERIFIED
  Kheap::deallocate           MISMATCH    VERIFIED
  Kheap::from_raw_parts       MISMATCH    VERIFIED
  Kheap::layout_to_allocator  MISMATCH    VERIFIED

Structs:
  ArenaAllocator   MATCH
  HeapStorage      MATCH
  Kheap            MATCH

Consistent: matched=3 mismatched=4 missing=0 extra=0
```

### Mismatch Analysis

All 4 mismatches are pre-approved or documented Verus deviations:

| Function | Deviation | Classification |
|---|---|---|
| `layout_to_allocator` | Named return `-> (result: ...)` | Pre-approved |
| `from_raw_parts` | Named return; `mem::PAGE_SIZE` cfg-gated; `addr as *mut u8` cfg-gated; `info!()` cfg-gated | Pre-approved + legitimate cfg-gating |
| `allocate` | Named return; `\|_\|` → `\|_e\|` in closures | Pre-approved + documented Verus limitation |
| `deallocate` | Named return; `\|_\|` → `\|_e\|` in closures | Pre-approved + documented Verus limitation |

**Key observations**:
- All original exec code is preserved under `#[cfg(not(verus_keep_ghost))]` where applicable.
- The `|_|` → `|_e|` rename is semantically identical (both discard the argument).
- No accidental exec code modifications detected.
- Zero missing or extra functions/structs.

**Verdict**: **PASS** — all mismatches are explained and justified.

---

## 6. Verification

### Result: PASS

```
verification results:: 19 verified, 0 errors (partial verification with --verify-*)

=== Results ===
  19 verified
  0 errors
  Exit code : 0

=== Cheating Pattern Check ===
  ⚠️  external_body: 2
  Affected functions:
    - usize_to_mut_ptr (line 96): external_body

=== Function Coverage ===
  4/7 exec functions have contracts.
  Unverified functions:
    - alloc
    - dealloc
    - init

=== Summary ===
  verification: 19 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0
  coverage: 4/7 exec functions have contracts
```

**19 verified, 0 errors.** Clean verification with no admits, no assumes (beyond the 2 justified `assume_specification`), and no trusted functions.

The 3 unverified functions (`alloc`, `dealloc`, `init`) are:
- `alloc`/`dealloc`: `GlobalAlloc` trait methods accessing `static mut HEAP` — Verus cannot model global mutable state or `unsafe` trait implementations.
- `init()`: One-time initialization accessing `static mut HEAP_STORAGE` and `HEAP` — same limitation.

These are thin delegation layers; the core logic they call (`Kheap::allocate`, `Kheap::deallocate`, `Kheap::from_raw_parts`) is fully verified.

---

## Overall Assessment

### Grade: A−

### Strengths

1. **Clean verification**: 19 verified, 0 errors, 0 admits — the gold standard.
2. **Strong spec quality**: All verified function contracts are bidirectional, with exact state transitions, frame conditions via `spec_allocate`/`spec_deallocate`, and proper error preservation.
3. **Good abstraction**: `KheapView` with `Seq<SlabView>` is clean, declarative, and enables quantified reasoning. The View pattern is applied correctly.
4. **Comprehensive proofs**: Cross-slab disjointness (MOD-1/2/3), allocation conservation (MOD-5), round-trip (LIVE-5), tightest-fit, monotonicity — all proven from first principles with no admits.
5. **Exec fidelity**: All deviations are documented, cfg-gated, and justified. Original exec code fully preserved.
6. **Minimal trust boundary**: Only 2 `assume_specification` (both human-approved, both minimal) and 1 trivial `external_body` helper.

### Weaknesses

1. **3 unverified exec functions**: `alloc`, `dealloc`, `init` lack any Verus contracts. While justified by Verus's `static mut` limitation, this leaves FN-5, FN-6, FN-7, LIVE-2, and the HEAP_STORAGE end-to-end chain (TYPE-4, MOD-7) unverified. These are the most likely sources of real bugs (e.g., BUG-1 double init).
2. **MOD-4 (no address zero) unproven**: This safety property (no null pointer returned) is not covered. It requires knowing `HEAP_STORAGE` has a non-zero address, which depends on linker placement — outside Verus scope but could be axiomatized.
3. **LIVE-1/LIVE-2 not formally proven**: The argument that slab construction always succeeds given known constants is convincing but informal. A Verus proof would strengthen confidence.
4. **TYPE-5, TYPE-6 not formalized**: Enum discriminant values and struct alignment are Rust compiler guarantees, not Verus-verified. Low risk but noted for completeness.

### Key Issues

| Priority | Issue | Impact |
|---|---|---|
| Medium | `init()`, `alloc`, `dealloc` unverified | Leaves GlobalAlloc wrapper + initialization unverified; core logic is verified |
| Low | MOD-4 (no null addr) unproven | Safety property about non-zero addresses not formalized |
| Low | LIVE-1/LIVE-2 informal | Slab construction feasibility argued but not machine-checked |
| Info | `usize_to_mut_ptr` not in Needed Assumptions list | Trivially correct; should be added for documentation completeness |

### Verdict

This is a high-quality verification effort. The core allocator logic (routing, allocation, deallocation, construction) has strong, bidirectional specs with clean proofs and zero admits. The trust boundary is minimal and well-justified. The main gap — unverified `GlobalAlloc` wrappers and `init()` — is a known Verus limitation for `static mut` global state, not a methodology failure. The verification provides meaningful assurance that the heap allocator correctly routes sizes to slabs, maintains cross-slab disjointness, preserves state on errors, and supports allocation/deallocation round-trips.
