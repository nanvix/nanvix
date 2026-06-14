## Turn 1: Full checklist walk-through — hal-frame-address cheating-elimination

### Progress
- Done (PASS): all checklist items below verified by tool output and code reading.
- Current: full sweep complete on turn 1.
- Remaining: none.

### Scope
Files:
- `src/kernel/src/hal/mem/types/address/frame.rs`
- `src/kernel/src/hal/mem/types/address/frame.spec.rs`
- `src/kernel/src/hal/mem/types/address/frame.proof.rs`

Baselines from `pipeline_state.json`:
- cheating-elimination phase start: `e384d053f498e4693b65afb6b3898987152ba223`
- proving phase start: `ed017dfeade9ef069894713c362e87dc51fcdf78`
- original merge: `281417515`

---

### Item-by-item verification

**1. Zero admit() — PASS**
`grep -rnE 'admit\('` over all three frame files → NONE FOUND. Module-scoped
verify cheating check: no `admit` attributed to the frame module
(`cheating-detail.txt` has no `hal/mem/types/address/frame` lines). The proof
bodies that previously held `admit()` (`lemma_frame_base_aligned`,
`lemma_aligned_div_mul`) now carry real proofs (`lemma_mod_multiples_basic`,
`lemma_fundamental_div_mod`) — confirmed by reading `frame.proof.rs`.

**2. Zero assume() — PASS**
`grep -rnE '\bassume\('` → NONE FOUND. Cheating gate: `assume=0`.
(`assume_specification` is a distinct construct — addressed in item 6/9.)

**3. Zero trusted functions — PASS**
`grep '#[trusted]'` → NONE. Cheating gate: `trusted=0`.

**4. Zero exec_allows_no_decreases_clause — PASS**
`grep exec_allows_no_decreases` → NONE. Cheating gate: `no_decreases=0`.

**5. Zero cfg-gated exec code — PASS**
Only two `#[cfg(verus_keep_ghost)]` occurrences (frame.rs:9, frame.rs:11), both
guarding `include!("frame.spec.rs")` / `include!("frame.proof.rs")` — i.e.
imports/includes, which are allowed. The previously-flagged cfg gate over the
`verus! { … }` spec block was removed this phase. Module-scoped verify reports
`✅ No cheating detected in module hal::mem::types::address::frame`.

**6. Zero external_body unless TCB-listed — PASS**
`grep external_body` over the three files → NONE FOUND. The frame module has
zero `external_body`. (The `tcb-allowed.md` entries for
`FrameAddress::from_raw_value`/`into_raw_value` describe their status *as
consumed by other modules*; in this module both are fully specced real-bodied
functions with `#[verus_spec]`.) Global `external_body=11` belongs to
out-of-scope, not-yet-verified modules and pre-dates this phase
(see commit `30e99602f`).

**7. AST consistency: zero mismatches — PASS**
`ast_consistency.py summary` run against three baselines:
- vs cheating-elimination start `e384d05`: matched=9 mismatched=0 missing=0 extra=0 ✅
- vs proving start `ed017df`: Consistent ✅
- vs original merge `281417515`: matched=9 mismatched=0 missing=0 extra=0 ✅
All 9 exec functions + `FrameAddress` struct MATCH. No exec drift cumulatively.

**8. All exec rewrites have VERUS REWRITE comment + reproducer — PASS (vacuous)**
AST consistency shows zero exec rewrites (all MATCH), so there are no rewrites
requiring justification comments.

**9. Surviving external_body listed in TCB — PASS (N/A) + assume_specification TCB-checked**
Zero `external_body` in the module. One `assume_specification` survives
(`frame.spec.rs:20`, `<PhysicalAddress as ::sys::mm::Address>::from_raw_value`).
It is explicitly listed in `verus-ai-logs/tcb-allowed.md` (lines 154-168) as the
intra-crate bottom-up placeholder for the not-yet-verified `phys` sibling
(`phys.rs:185` carries no `#[verus_spec]`). Cheating gate does not count it
(`assume=0`). Acceptable.

**10. No specs weakened (spec-drift) — PASS**
`spec_drift.py git-diff frame.rs --before e384d05` (phase baseline):
`Contract drift: 0`, `Ensures removed: 0`, `Requires added: 0` → exit 0,
"✅ No contract drift detected." (vs proving start the only flagged items are
proof-body fill-ins of the two lemmas — admit() → real proof — with
ensures-removed=0/requires-added=0, i.e. not spec weakening.)

**11. Cross-module regression — PASS**
`make verify-kernel` (full) → exit 0, "verification: ... (exit 0)". All
registered/verified modules pass. Global cheating counts
(`external_body=11 admit=29 cfg_gate=14`) are unchanged pre-existing residue in
out-of-scope modules (identical to baseline commit `30e99602f`), not a
regression.

**12. Verification + build: 0 errors, 0 warnings — PASS**
- `make verify-kernel MODULE=hal::mem::types::address::frame` → Exit 0, status
  CLEAN, `6 verified, 0 errors`.
- `./z build -- all-kernel` with forced recompile (`touch frame.rs`) →
  `Compiling kernel v0.16.17` … `Finished` with no warnings, `[OK] Build
  complete.` 0 errors, 0 warnings.

---

### Verdict
All 12 checklist items PASS with concrete tool evidence. No fix requests.
Cheating elimination for hal-frame-address is RESOLVED.
