## Turn 2: Verify turn-1 fix to `from_mmio_address` → RESOLVED

### Progress
- **Done (PASS or FIXED):** all 17 checklist items.
  - FIXED this turn: (4) no tautological ensures, (6) error paths meaningful, (2/9) caller coverage / spec usable in caller proofs — all via the `from_mmio_address` contract rewrite.
  - PASS from turn 1 (re-confirmed): fn-coverage, view-consistency, subsumed-ensures, assume_specification scoping, vstd-search, trait-obligations, spec-completeness, loop-invariants (N/A), cheating-counts, spec-drift, bug-awareness, cross-module regression, verification+build.
- **Current:** none — review complete.
- **Remaining:** none.

### Verification (independent — did not trust fixer claims)

**Source inspection** — `phys.rs:112–122`, `from_mmio_address` now reads:
```rust
#[verus_spec(result =>
    requires
        spec_frame_number(addr@) <= spec_max_frame_number(),
    ensures
        result is Ok,
        (result->Ok_0)@ == addr@,
        (result->Ok_0).inv(),
)]
pub unsafe fn from_mmio_address(addr: VirtualAddress) -> Result<Self, Error> {
    Ok(Self(addr))
}
```
Matches the turn-1 fix request exactly. The prohibited `Err(_) => true` arm is gone; `inv()` is now guaranteed on the `Ok` result.

**Tool output I ran myself:**
1. `grep -n "Err(_) => true" src/kernel/src/hal/mem/types/address/phys.rs` → **exit 1, no match** ✅ (tautology eliminated — item 4/6).
2. `spec_drift.py check hal-phys-address --logs-dir verus-ai-logs` → **0 contract drift, exit 0** ✅ (strengthening only; no original guarantee weakened — item 14).
3. `make verify-kernel` (cached) → **exit 0**.
4. **Forced non-cached re-verification** (`touch phys.rs && make verify-kernel`) → **`55 verified, 0 errors, exit 0`**. This proves the new `from_mmio_address` contract genuinely verifies from scratch — `result is Ok` and `(result->Ok_0)@ == addr@` discharge from the body `Ok(Self(addr))`, and `(result->Ok_0).inv()` reduces to the added `requires` (since `r@ == addr@`). Not a stale cache.

**Caller-usability (item 2/9) confirmed:** the result of `from_mmio_address` flows (via `book_mmio_regions`, `mm/phys/mod.rs:118` → `PageAligned::from_address` → `frame::is_covered`) into `is_covered`, which `requires phys_addr.inv()` (`frame.rs:526`). The contract now supplies `(result->Ok_0).inv()`, so the caller can discharge that precondition. The RAM-range validator remains intentionally bypassed (design intent preserved).

**Re-confirmed PASS items (unchanged since turn 1):**
- Cheating counts in-module: `assume=0 external_body=0 trusted=0`; only 2 `admit()` proof-lemma stubs (`phys.proof.rs:6,16`), commented "Proven in the proving phase" — phase-acceptable, explicitly tracked for the proving phase. Full-kernel `external_body=14/admit=26/cfg_gate=16` are all outside this module (frame/manager/upool/mod), out of scope.
- Cross-module regression: `kernel::all` exit 0.
- `fn_coverage.py`: 16/16 matched, 0 missing, 0 extra.
- View consistency: specs reference `self@`, `spec_frame_number`, `spec_from_number`, `inv()` per `view_design.md`.

### Result
All checklist items PASS or FIXED with tool-backed evidence. No outstanding FAIL. → **RESOLVED**.

### Fix Request
None — specification phase review complete.
