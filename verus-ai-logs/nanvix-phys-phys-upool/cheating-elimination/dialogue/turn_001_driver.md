## Turn 1: Full checklist walk-through — phys-upool cheating-elimination

### Progress
- Done (PASS, with evidence below):
  1. Zero admit()
  2. Zero assume()
  3. Zero trusted functions
  4. Zero exec_allows_no_decreases_clause
  5. Zero cfg-gated exec code (only imports/derives/debug_assert/logging)
  6. Zero external_body unless listed in tcb-allowed.md
  7. AST consistency: zero *real* mismatches (only allowed logging cfg-gate)
  8. All exec rewrites documented appropriately
  9. Every surviving external_body listed in tcb-allowed.md
  10. No specs weakened (spec-drift)
  11. Cross-module regression (verify all modules)
  12. Verification + build (0 errors, 0 warnings)
- Current: none
- Remaining: none

### Verification

Files in scope:
- `src/kernel/src/mm/phys/upool.rs` (283 lines)
- `src/kernel/src/mm/phys/upool.spec.rs` (17 lines — `UserFrame::inv`)
- `src/kernel/src/mm/phys/upool.proof.rs` (1 line — empty `verus!{}`)

**`make verify-kernel`** — exit code 0. All modules verified:
`mm::phys`, `mm::phys::frame`, `mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool`.
Cheating-pattern check (crate-wide): `assume=0 trusted=0 no_decreases=0`.
`cheating-detail.txt` attributes to upool ONLY:
```
mm/phys/upool.rs:221 Upool (struct): external_body
mm/phys/upool.rs:246 new:            external_body
mm/phys/upool.rs:279 alloc:          external_body
```
No `admit` and no `assume` entries reference upool.

**Item 1–4 (admit/assume/trusted/no_decreases):** PASS.
`grep` over `upool.rs`, `upool.spec.rs`, `upool.proof.rs` for `admit`, `assume`,
`trusted`, `exec_allows_no_decreases_clause`/`no_decreases` returns zero exec hits
(spec.rs/proof.rs are clean; the only `assume` text is in a doc/comment? — none found).
Confirmed by crate-wide counters (`admit` detail list contains no `upool` line).

**Item 5 (cfg-gated exec):** PASS. The four `cfg` occurrences in upool.rs are all allowed:
- L9, L11 `#[cfg(verus_keep_ghost)] include!("upool.spec.rs"/"upool.proof.rs")` — spec/proof imports.
- L37 `#[cfg(verus_keep_ghost)] verus! { ... View impls ... }` — ghost/spec code (not exec).
- L203 `#[cfg(not(verus_keep_ghost))] error!(...)` inside `Drop::drop` — **logging**, explicitly allowed.

**Item 6 & 9 (external_body listed):** PASS. Exactly 3 `#[verus_verify(external_body)]`:
- `Upool` struct (L220) — listed in `tcb-allowed.md` ("Upool (struct) ... no specs yet; opaque type/callee needed so verified init can construct the user page pool").
- `Upool::new` (L241) — listed in `tcb-allowed.md` (same entry).
- `Upool::alloc` (L262) — listed in `tcb-allowed.md` ("pool allocation primitive ... ensures describes the free→allocated transition (alloc_one) and the empty-pool Err arm").
No unlisted external_body. (`UserFrame`'s `external_derive` on L31 is a `#[derive(Debug)]` helper, not an external_body / trusted function.)

**Item 7 (AST consistency):** PASS. `ast_consistency.py … count` reports
`1 mismatched (7 functions match)`. The sole mismatch is `UserFrame::drop`, diff:
```
     fn drop(&mut self) {
         if let Err(e) = frame::free(self.addr) {
+            <#[cfg(not(verus_keep_ghost))] stripped>
             error!("failed to free user frame: {:?}", e);
         }
     }
```
This is purely the allowed logging cfg-gate (item 5). It is the project-accepted
convention: the already-verified sibling modules report the identical tool
"mismatch" from the same pattern and passed their own cheating-elimination —
`kframe.rs` 1 mismatch (`KernelFrame::drop`, identical diff), `frame.rs` 7,
`manager.rs` 4 — all logging gates. The shipped (non-`verus_keep_ghost`) binary is
byte-identical to the original; the gate only removes the call during verification.
This is a "semantically-equivalent rewrite for a verified Verus limitation" (logging
macros do not translate), so it satisfies the AST-consistency exception.

**Item 8 (exec rewrites documented):** PASS. The only exec deviation is the
cfg-gated logging call, a categorically-allowed logging gate that needs no
VERUS-REWRITE reproducer (and siblings carry none). No compound-assignment /
mutable-index / closure rewrites exist in scope.

**Item 10 (spec drift):** PASS. `spec_drift.py check phys-upool`
(baseline `510085f` = cheating-elimination start → HEAD): "Functions with changes: 0;
Contract drift: 0; Ensures removed: 0; Requires added: 0. ✅ No contract drift detected."

**Item 11 (cross-module regression):** PASS. `make verify-kernel` verifies all 5
phys modules with exit 0 (above).

**Item 12 (verification + build):** PASS.
- Verify: `make verify-kernel` exit 0.
- Build: `./z build -- all-kernel` → "Finished dev profile … Build complete." — 0 errors, 0 warnings.

### Fix Request
None. Every checklist item is PASS with concrete tool evidence. No code changes required.
