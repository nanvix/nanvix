## Turn 1: Cheating Elimination — full checklist walk; item 1 (admit) FAIL

### Progress
- Done (PASS, evidence below):
  - [x] Zero `assume()`
  - [x] Zero trusted functions
  - [x] Zero `exec_allows_no_decreases_clause`
  - [x] Zero cfg-gated exec beyond allowed (logging only)
  - [x] Zero unlisted `external_body` (both listed in `tcb-allowed.md`)
  - [x] AST consistency (matched=8, mismatched=0)
  - [x] Exec rewrites have comment + reproducer (vacuous: zero exec rewrites)
  - [x] Each surviving `external_body` confirmed listed
  - [x] No specs weakened (spec-drift = 0 on all 3 files)
  - [x] Cross-module regression (all kernel modules verify, exit 0)
  - [x] Verification exit 0 (kernel compiles clean under Verus build)
- Current: **[ ] Zero `admit()` — FAIL (4 remaining)**
- Remaining: none uninspected; the only open item is the 4 `admit()`s.

### Verification

Scope = manager editable files only: `manager.rs`, `manager.spec.rs`, `manager.proof.rs`.

**`make verify-kernel`** → Exit code 0; cheating gate:
`assume=0 external_body=18 admit=24 trusted=0 no_decreases=0 cfg_gate=15` → `CHEATING_DETECTED`.
Per-manager breakdown from `verus-logs/cheating-detail.txt`:
```
mm/phys/manager.proof.rs:12  lemma_manager_attached:        admit
mm/phys/manager.proof.rs:27  lemma_kernel_alloc_one:        admit
mm/phys/manager.proof.rs:40  lemma_kernel_alloc_contiguous: admit
mm/phys/manager.proof.rs:210 lemma_user_bulk_err_restored:  admit
mm/phys/manager.rs:104       init:             external_body
mm/phys/manager.rs:529       kernel_watermark: external_body
```

- **assume / trusted / no_decreases**: grep of all three manager files = 0. PASS.
- **cfg-gated exec** (9 sites: lines 207,213,339,345,382,385,452,458,500): every one is
  `#[cfg(not(verus_keep_ghost))]` guarding an `error!`/`warn!` logging macro. Logging is on the
  allowed list. PASS. (Lines 8,10 are `#[cfg(verus_keep_ghost)] include!` of the spec/proof —
  imports, allowed.)
- **external_body** (2 sites): `init` (manager.rs:104) and `kernel_watermark` (manager.rs:529).
  Both explicitly listed in `verus-ai-logs/tcb-allowed.md` ("External-bottom: build-time constant
  accessor" for `kernel_watermark`; cross-module/singleton-write rationale for `init`). PASS.
- **AST consistency**: `ast_consistency.py --base-ref 3a0b7204c... manager.rs summary` →
  `Consistent: YES (matched=8 mismatched=0 missing=0 extra=0)`. PASS.
- **Spec drift**: `spec_drift.py git-diff` on all three files vs phase-start SHA
  `3a0b7204c...` → `No contract drift detected` (0 ensures removed, 0 requires added) for each.
  `git diff --stat <phase-start> -- manager.{rs,spec.rs,proof.rs}` is **empty** — files are
  byte-identical to phase start, i.e. the prover changed nothing. PASS for drift, but it confirms
  the admits were never addressed.
- **Cross-module**: `verify-kernel` verifies `mm::phys`, `mm::phys::frame`, `mm::phys::kframe`,
  `mm::phys::manager`, `mm::phys::upool` → exit 0, 0 verification errors. PASS.
- **Build**: Verus cargo build of the kernel crate finished (exit 0, no warnings emitted). PASS.

**Conclusion:** 11/12 PASS. The lone FAIL is **Zero `admit()`** — 4 `admit()`s survive in
`manager.proof.rs`. This is the cheating the phase exists to remove; it is a blocker.

### Fix Request

The phase produced **zero source changes** (empty diff) and a `fix_report.md` that *argues* the 4
admits are unremovable. **Justification is not a fix.** Remove the `admit()`s, or produce tool
output proving each is a verified Verus limitation. Address each lemma individually:

1. **`lemma_manager_attached` (manager.proof.rs:16)** — `ensures m@ == phys_view().frames` with an
   `admit()` body. This axiomatizes that the manager's view *is* the global partition. Do not
   assume it. Either:
   (a) thread the §8 ghost token in as a `Tracked<...>` parameter and derive the equality from the
   token's invariant, or
   (b) if `phys_view()` cannot yet be linked in-scope, run the escalation ladder (search vstd →
   isolated reproducer) and attach the minimized reproducer showing Verus cannot express the link.
   A prose claim alone will be rejected.

2. **`lemma_kernel_alloc_one` (manager.proof.rs:35)** — `ensures post == pre.alloc_one(addr)` via
   `admit()`. `frame::alloc`'s tcb-allowed contract only guarantees `Ok(frame) => frame.inv()`; it
   does **not** export the free→reserved partition transition, so this lemma is currently a raw
   axiom. Eliminate the admit by **deriving** the transition from an actual exported contract. If
   the needed transition is genuinely absent from every in-scope contract, that is a missing
   contract — say precisely which contract on which callee must supply it, and prove the gap with a
   minimal reproducer (`make verify-kernel MODULE=mm::phys` output with the admit removed).

3. **`lemma_kernel_alloc_contiguous` (manager.proof.rs:55)** — same defect as #2 for the contiguous
   path (`post == pre.book_all(kernel_addr_set(frames))`). Same requirement: derive from a real
   contract or produce the reproducer. Note `lemma_contig_no_overflow` (same file) was proven
   without admit via `vstd::arithmetic::mul` — demonstrate the same diligence here.

4. **`lemma_user_bulk_err_restored` (manager.proof.rs:216)** — `ensures m@ == pre` via `admit()`,
   justified by "Drop side effects are not modeled." Removing the admit must not be replaced by an
   assume. Show the actual restoration: model the `clear()`/`UserFrame::drop → frame::free` path
   through an explicit proof using the per-handle free contract, or restructure the exec so the
   restoration is witnessed by a value the verifier sees. If Drop semantics are provably
   unrepresentable in-scope, attach the isolated reproducer.

**Required verification after the fix:** run `make verify-kernel` and paste the cheating line. The
acceptance bar is `admit=0` for the three manager files (i.e. lines 16/35/55/216 gone) with
`Exit code 0` and zero new `assume`/`external_body`. If any admit truly cannot be discharged
in-scope, it must be converted into a documented, `tcb-allowed.md`-listed boundary **with a
minimized reproducer committed** — not left as a bare `admit()` and not defended with prose.

Do not weaken any spec to make the admit removable (`spec_drift.py` must stay at 0).
