## Turn 2: Verify fixer t1 + walk remaining checklist

### Progress
- FIXED: **Zero trusted functions** (the t1 FAIL) — verified below.
- PASS (re-confirmed): admit, assume, exec_allows_no_decreases, external_body-in-kframe,
  cfg-gated-exec, external_body-listing.
- PASS (newly checked this turn): AST consistency, exec-rewrite comment+reproducer,
  spec-drift, cross-module regression, build.
- Remaining: none. → STOP = RESOLVED.

### Verification

**1. Trusted-function FAIL (t1) — FIXED.**
- `verus-ai-logs/tcb-allowed.md`: stale `KernelFrame::new` `external_body` entry **removed**;
  a new entry for `KernelFrame::map_frame` **added** (lines 92-103) correctly documenting it as
  an `assume_specification` with an **empty** contract. `grep 'KernelFrame::new' tcb-allowed.md`
  → only a descriptive mention inside the map_frame entry; no ledger entry. Confirmed.
- Fixer's load-bearing claim independently verified: `mm/virt/mod.rs:17-21` re-exports only
  `identity_map_page, memcpy, sync_kernel_pdes` — `identity_map_view` is **not** re-exported, so
  the `identity_map_view().inv()` precondition cannot be named from `mm::phys`. Option A (re-inline)
  is genuinely blocked; option B (ledger) is the smaller trust boundary. The `assume_specification`
  trusts strictly less than the prior `external_body` on `new` (whose `kf@==base@`/`kf.inv()` posts
  are now machine-verified).
- `make verify-kernel MODULE=mm::phys::kframe` → Exit 0, `✅ No cheating detected`, `status: CLEAN`,
  `assume=0 external_body=15 admit=7 trusted=0 cfg_gate=12` (all non-zero counts are in OTHER
  bottom-up modules; `grep -i kframe cheating-detail.txt` → empty).

**2. AST consistency** — `ast_consistency.py summary`: matched=4, mismatched=2 (`new`, `drop`),
extra=1 (`map_frame`). Every mismatch is of the explicitly-permitted kind:
  - `new` (MISMATCH) + `map_frame` (EXTRA_IN_VERUS): the `external_body`-elimination rewrite.
    `ast diff` shows the original inlined `PageAligned::from_raw_value`/`identity_map_page` block was
    extracted **byte-for-byte** into `map_frame`; `new`'s body is now `Self::map_frame(base)?;
    Ok(Self{base})`. Semantically identical (same statements, same order, same `?` propagation) —
    a verified-Verus-limitation rewrite, the carve-out the checklist allows. Has a
    `// VERUS REWRITE:` comment (kframe.rs:92-102).
  - `drop` (MISMATCH): a blank-line artifact from `#[cfg(not(verus_keep_ghost))]` on the `error!`
    log line; the cfg-gate is **pre-existing** (present byte-identical at phase-start SHA 3763437,
    not introduced by this phase) and is the pre-approved "cfg-gating logging" deviation. Logging
    is explicitly allowed exec.

**3. Exec rewrite comment + reproducer** — `// VERUS REWRITE:` present at the extraction site
(kframe.rs:92-102) and points at the spec contract + ledger. The three blockers are evidenced:
(a) `error!` → "Unsupported constant type", (b) `PageAligned::from_raw_value` external,
(c) `identity_map_view` private/non-re-exported (I reproduced (c) by grep). Adequate.

**4. Spec-drift** — `spec_drift.py check phys-kframe` (baseline 3763437 → HEAD): "Functions with
changes: 0", "Contract drift: 0", "Ensures removed: 0". `new`'s contract is byte-identical and now
*verified* rather than assumed. **No spec weakened.** Exit 0.

**5. Cross-module regression** — full `make verify-kernel` (no MODULE) → Exit 0. All currently
verified modules pass (matches prior `kernel::all` PASS). The global `status: CHEATING_DETECTED` is
the aggregate of *other* in-progress bottom-up modules' ledgered trust, not a kframe regression.

**6. Build (`make check-kernel`)** — parsed cargo JSON: **0 warnings, 1 error**:
`unused variable: i @ src/kernel/src/mm/phys/manager.rs:245`. This is **not attributable to
kframe**: `git diff 3763437..HEAD -- 'src/**/*.rs'` shows only `kframe.rs`/`kframe.spec.rs` changed;
`manager.rs` is byte-identical to base branch `verus-ai-prove-bottom-up`. kframe itself emits **zero**
diagnostics (`grep kframe` over build output → 0). The error is a pre-existing lint in an
out-of-scope, still-mid-verification sibling module (`manager` carries 4 admits) where `i` is used
only in ghost code. Fixing it would require editing unrelated code, which the engineering rules
forbid. Build is clean for the phys-kframe scope.

### Fix Request
None. All checklist items PASS or FIXED with evidence. Creating STOP = RESOLVED.

Note (informational, not a blocker for this module): the pre-existing
`manager.rs:245 unused variable: i` `-D warnings` error should be addressed in the `manager`
module's own pipeline, not here.
