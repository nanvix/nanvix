## Turn 1: Full Checklist Walkthrough — bump-allocator Cheating Elimination

### Progress
- Done (PASS with evidence): ALL 12 checklist items
- Current: full checklist verified on first pass
- Remaining: none

### Verification

Commands run (from repo root `/home/ruize/nanvix-phy-specs`):

1. `make verify-bump-allocator`
   - Exit code: 0 (verification cached, no recompilation).
   - Cheating Pattern Check: `assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
   - `cheating-detail.txt`:
     - `lib.rs:286 alloc: external_body`
     - `lib.rs:367 alloc_as: external_body`

2. `python3 .../ast_consistency.py src/libs/bump_allocator/src/lib.rs summary`
   - Result: `Consistent: ✅ YES (matched=12 mismatched=0 missing=0 extra=0)`, exit 0.
   - Every exec fn/struct = MATCH (alloc, alloc_as, align_up, new, default, fmt, backends, tests, all structs).

3. `python3 .../spec_drift.py check bump-allocator`
   - Baseline `a9faa9c8a61d` (cheating-elimination phase start) → HEAD.
   - Result: `✅ No contract drift detected`, exit 0. Contract drift: 0; ensures removed: 0; requires added: 0.

4. `make verify` (cross-module regression, all crates + kernel)
   - All modules verified; every crate reported `Exit code : 0`. No regressions introduced by bump-allocator. (Cheating counts reported for `arch`/`kernel` belong to other, not-yet-completed modules and are out of scope for this module's review.)

5. `cargo build -p bump-allocator` (kernel target, build-std)
   - `Finished dev profile ... in 13.15s`, exit 0, **0 warnings, 0 errors**.

Per-item findings:

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | Zero `admit()` | **PASS** | cheating check `admit=0`; proof bodies (`lemma_geometry`, `lemma_exhausted_boundary`, `lemma_alloc_transition`) contain no `admit()`. Only a stale *comment* at `lib.proof.rs:6` mentions the word — see advisory below. |
| 2 | Zero `assume()` | **PASS** | cheating check `assume=0`. The lone `assume_specification [<usize>::div_ceil]` (lib.spec.rs:28) is the accepted std-lib spec construct (not `assume()`), correctly pinned to Rust's documented `div_ceil` semantics with `y != 0`. |
| 3 | Zero trusted functions | **PASS** | cheating check `trusted=0`. |
| 4 | Zero `exec_allows_no_decreases_clause` | **PASS** | cheating check `no_decreases=0`. |
| 5 | Zero cfg-gated exec code | **PASS** | cheating check `cfg_gate=0`. Only cfg gates present are `#[cfg(verus_keep_ghost)]` on the spec/proof `include!`s (ghost-only) and `#[cfg(test)]` on the unit-test module — both allowed. |
| 6 | Zero `external_body` unless in tcb-allowed | **PASS** | Exactly 2: `FixedSizeBumpAllocator::alloc` and `::alloc_as`. Both explicitly listed in `verus-ai-logs/tcb-allowed.md` with `usize as *mut` / re-materialization rationale, mirroring `raw-array`/`frame::instance`. Each carries a contract (`requires bump_view(self).inv()`, alignment + in-bounds `ensures` over `bump_view`) — not contract-free. |
| 7 | AST consistency | **PASS** | ast_consistency summary: 0 mismatch / 0 missing / 0 extra; exit 0. |
| 8 | All exec rewrites have VERUS REWRITE comment + reproducer | **PASS (N/A)** | No exec rewrites exist (all AST nodes MATCH; `grep "VERUS REWRITE"` → none needed). |
| 9 | Each surviving `external_body` listed in tcb-allowed | **PASS** | Both `alloc` and `alloc_as` individually confirmed present in `tcb-allowed.md`. |
| 10 | No specs weakened | **PASS** | spec_drift `check` exit 0, 0 contract drift vs phase-start baseline. |
| 11 | Cross-module regression (`make verify`) | **PASS** | All verified modules exit 0. |
| 12 | Verification + build, 0 errors / 0 warnings | **PASS** | `make verify-bump-allocator` exit 0; `cargo build -p bump-allocator` exit 0, 0 warnings. |

Per-function external_body audit (challenged individually):
- `lib.rs:286 FixedSizeBumpAllocator::alloc` — LISTED in tcb-allowed.md (`usize as *mut [u8; N]` from backend address; no `PointsTo` for externally-owned `BssStorage`). Has full `#[verus_spec]` contract. **Accepted.**
- `lib.rs:367 FixedSizeBumpAllocator::alloc_as` — LISTED in tcb-allowed.md (delegates to `alloc`, re-materializes as `&'static mut MaybeUninit<T>`; adds size/align guard arms). Has full `#[verus_spec]` contract. **Accepted.**

### Fix Request
None — all checklist items PASS with concrete tool evidence.

### Advisory (non-blocking, not a checklist failure)
`src/libs/bump_allocator/src/lib.proof.rs:6` still reads *"Bodies are `admit()` placeholders during the specification phase"*, which is now stale: the lemma bodies are fully discharged (`admit=0`). This is a documentation-only inaccuracy and does not violate any checklist item, so it does not gate resolution. Recommend updating the header comment in a future cleanup.
