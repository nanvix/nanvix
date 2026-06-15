# Final Independent Review — `hal-memory-region`

## Spec Quality
- In-scope getters have explicit external-top contracts:
  - `MemoryRegion::start`: `ensures result@ == self@.start`
  - `MemoryRegion::size`: `ensures result as int == self@.size`
  - `TruncatedMemoryRegion::start`: `ensures result@ == self@.start`
  - `TruncatedMemoryRegion::size`: `ensures result as int == self@.size`
- Contracts are understandable, non-tautological, and appropriate for trivial getters (spec-design: “basic ensures only”).
- Mathematical modeling is appropriate (`start`, `size` as `int` in `MemoryRegionView`).
- View consistency: both region types share `MemoryRegionView`; truncated view delegates to inner (`self.0@`) consistently.
- `inv()` is non-trivial:
  - `MemoryRegion::inv()`: `size > 0`
  - `TruncatedMemoryRegion::inv()`: `size > 0` + page alignment (`start % page_size == 0`, `size % page_size == 0`)
- Quality gap: no spec-level in-range invariant (`start + size - 1 <= max_addr`) for generic regions.

## Caller Coverage (Covered 9/11, Missing 2)
Covered:
1. `MemoryRegion::start` returns stored start exactly (`ensures`).
2. `MemoryRegion::size` returns stored size exactly (`ensures`).
3. `MemoryRegion::size` non-zero (`MemoryRegion::inv().wf()`).
4. `TruncatedMemoryRegion::start` returns stored start exactly (`ensures`).
5. `TruncatedMemoryRegion::start` page-alignment (`TruncatedMemoryRegion::inv().is_page_aligned()`).
6. `TruncatedMemoryRegion::size` returns stored size exactly (`ensures`).
7. `TruncatedMemoryRegion::size` non-zero (`inv().wf()`).
8. `TruncatedMemoryRegion::size` page-multiple (needed by `frame.rs` division) (`inv().is_page_aligned()`).
9. Ordering-key role of `start` is preserved (`Ord::cmp` uses `start`; getter ensures project `self@.start`).

Missing/uncovered:
1. `MemoryRegion::size` caller expectation: in-range geometry (`start_raw + size - 1 <= T::max_addr()`) is not encoded in getter requires/ensures or `inv()`.
2. Derived truncated-size in-range expectation (`start + size` within address-space bound) likewise not encoded in getter contracts/`inv()`.

## Proof Completeness (counts + locations)
- `admit()`: **0** in `region.rs`, `region.spec.rs`, `region.proof.rs`.
- `external_body`: **0** in `region.rs`, `region.spec.rs`, `region.proof.rs`.
- `region.proof.rs` currently contains only `verus! { }` (no remaining proof placeholders).

## TCB Compliance
- `external_body` occurrences in region files: **0**.
- Therefore, TCB allowed-list compliance is **PASS (vacuous)**.

## Guardrails Compliance (exact counts, region files only)
- `admit`: **0**
- `assume(`: **0**
- `external_body`: **0**
- `assume_specification`: **0**
- cfg-gated exec code: **0**
  - Note: the two `#[cfg(verus_keep_ghost)] include!(...)` lines in `region.rs` are spec/proof includes, not cfg-gated exec logic.

## AST Consistency
- `ast_consistency.py ... summary`: **MISMATCH = 1** (`MemoryRegion::start` only), all others MATCH.
- Diff: `self.start.clone()` → `self.start.clone_address()`.
- Verdict on mismatch: **ACCEPTABLE-JUSTIFIED (not a blocker)**.
  - Required `VERUS REWRITE` comment and minimal reproducer are present in `region.rs`.
  - `Address::clone_address` contract exists with `ensures result@ == self@` in `src/libs/sys/src/sys/mm/address/mod.rs`.
  - Implementations are genuine view-preserving clones:
    - `phys.rs`: `PhysicalAddress(self.0)`
    - `aligned/page.rs`: `PageAligned(self.0.clone_address())`
    - `aligned/pgtab.rs`: `PageTableAligned(self.0.clone_address())`

## Verification
- Command: `make verify-kernel MODULE=hal::mem::types::region`
- Result: **PASS** (exit code 0)
- Verus errors: **0**

## Bug Summary
- `bugs.md` at `verus-ai-logs/nanvix-phys-hal-memory-region/bugs.md`: **does not exist**.
- No recorded bugs for this module.
- Independent defect assessment for in-scope exec code: no runtime logic defect found in the 4 getters (all are direct field projections/delegations).
- However, there is a **spec coverage defect** (missing in-range contract coverage as noted above).

## Issues (priority order)
1. **P1 (BLOCKER): Missing caller-expected in-range property coverage** for `MemoryRegion::size` (and consequently truncated geometry bound) in getter contracts/`inv()`.

## Result: **FAIL**
Reason: strict caller-coverage requirement not fully met (missing/uncovered expectations).
