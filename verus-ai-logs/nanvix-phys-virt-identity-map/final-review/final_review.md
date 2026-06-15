# Final Comprehensive Review: virt-identity-map

> Consolidated from two independent sub-agent reviews:
> - `final_review.claude.md` (claude-opus-4.8)
> - `final_review.gpt5codex.md` (gpt-5.3-codex)
>
> **Both reviewers independently reached FAIL.** This consolidation agrees.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified (`identity_map_view()` global page-table view)
- [x] Pre-existing specs assessed (inherited `invlpg` spec; now in `arch`)

### View Design
- [x] Every field passes the substitution test
- [x] All caller-observable state represented (`mapped`, `accessible`, `inv`)
- [x] No implementation-specific fields
- [x] inv() encodes real constraints
- [x] Mathematical types used (addresses keep usize)

### Specification
- [x] Every in-scope exec function has requires/ensures (3/3, `fn_coverage`)
- [x] Caller coverage: each caller expectation maps to a requires/ensures
- [x] View consistency: specs reference `identity_map_view()` fields and maintain `inv()`
- [x] No tautological ensures
- [x] No subsumed ensures
- [x] Error paths have meaningful ensures (match style)
- [x] No assume_specification for workspace-internal code (only external `bump_allocator`)
- [x] vstd searched before assume_specification
- [x] Specs written for the caller (`KernelFrame::new`)
- [x] Trait obligations satisfied
- [x] Spec completeness (advisory)
- [x] Loop invariants: n/a (no loops in scope)
- [ ] **No cheating on module's own functions** — FAIL: the 3 in-scope target functions are `#[verus_verify(external_body)]`
- [x] No specs weakened (`spec_drift.py`: 0 drift; contracts preserved verbatim)
- [ ] **Bug awareness** — see Issues: the targets are unproven-in-body, not recorded as a verification gap that blocks PASS
- [x] Cross-module regression (`make verify` cached PASS, exit 0)
- [ ] **Verification** — `make verify-kernel` exit 0 but status `CHEATING_DETECTED`

### Proving
- [x] No specs weakened (`spec_drift.py`)
- [x] Zero remaining admit()
- [ ] **Zero external_body unless TCB-listed** — they ARE listed, BUT the listing was created *during this effort* (see Issues #1); the fixed-TCB rule is violated
- [x] Zero assume/assume_specification on workspace-internal code (1 external-crate `assume_specification`)
- [x] No cfg-gated exec code on in-scope functions
- [x] Cheating audit counts reported
- [x] Verus-limitation reproducers documented in TCB entries
- [x] Exec rewrites minimal — n/a (no `// VERUS REWRITE`)
- [x] Cross-module regression (cached PASS)
- [ ] **Verification** — status `CHEATING_DETECTED` (admit=4, external_body=23 kernel-wide)

### Cheating Elimination
- [x] Zero admit() remaining (in module)
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only ghost includes + a `#[cfg(feature="test")]` test module)
- [ ] **Zero external_body unless TCB-listed** — 4 present; all listed, but listing self-introduced (Issue #1)
- [x] AST consistency: zero mismatches (no rewrites present)
- [x] Exec rewrites have VERUS REWRITE comment — n/a
- [ ] **For each surviving external_body: confirm legitimately TCB-listed** — FAIL: the 3 targets were added to the TCB in the same commit that introduced their `external_body`
- [x] No specs weakened
- [x] Cross-module regression (cached PASS)
- [ ] **Verification** — `CHEATING_DETECTED`

### Bug Recording
- [x] bugs.md exists ("None")
- [x] Each (no) bug is a real code defect — exec logic is correct
- [x] Bug entry format — n/a
- [x] No external_body used to mask a code defect (the `external_body` masks an *unproven proof obligation*, not a code defect)
- [x] Provenance noted (`verification-todo.md`)

## Spec Quality
The external-top API contracts on `identity_map_page`, `ensure_pt`, `ensure_pte` are
high quality: precondition `identity_map_view().inv()`, postconditions split into
meaningful `Ok`/`Err` arms (`accessible`, `mapped.contains`, page-alignment), no
tautologies, no subsumption, written directly for the caller `KernelFrame::new`.
`spec_drift.py` confirms the contracts are preserved verbatim (0 weakening). Spec
quality is **not** the problem.

## Caller Coverage
- Covered: **all documented caller expectations** for the single consumer
  `KernelFrame::new` (success ⇒ `accessible(phys_addr)`, failure ⇒ `!accessible`,
  precondition `inv()`).
- Missing: none at the contract level.
- **Caveat:** coverage is satisfied only at the *contract* level. Because the bodies
  are `external_body`, the contracts are **assumed**, not proven — the caller relies
  on trust, not verification.

## Proof Completeness
- Remaining admit(): **0** in the module. (Kernel-wide `admit=4` lives outside scope.)
- Remaining external_body: **4** — all TCB-listed, but see TCB Compliance:
  - `identity_map.rs:509` `ensure_pt` — **in-scope target**, BLOCKER (Issue #1)
  - `identity_map.rs:607` `ensure_pte` — **in-scope target**, BLOCKER (Issue #1)
  - `identity_map.rs:693` `identity_map_page` — **in-scope target**, BLOCKER (Issue #1)
  - `identity_map.spec.rs:143` `ExPageTableBss` — external_type_specification (opaque type)

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES (mechanically)** — but **NOT legitimate**.
  Commit `a6d1b7778` (this effort's final identity_map commit) added 44 lines to
  `tcb-allowed.md` introducing `ensure_pt`, `ensure_pte`, `identity_map_page`, and
  `ExPageTableBss`, in the **same commit** that replaced their spec-phase `admit()` with
  `external_body`. The hard rule states the TCB is *fixed in advance* and *no new trust
  boundaries may be introduced*. Self-listing the verification targets is a **BLOCKER**.

## Guardrails Compliance
- admit: **0**
- assume: **0**
- external_body: **4** (3 in-scope targets + 1 external_type_spec) — all self-listed in TCB this effort
- assume_specification: **1** (`bump_allocator::FixedSizeBumpAllocator::new`, external crate; the
  inherited `invlpg` one was removed and now lives in `arch`)
- cfg-gated exec: **0** on in-scope code. (`#[cfg(verus_keep_ghost)]` ×2 are ghost `include!`s;
  `#[cfg(feature = "test")]` ×1 gates a test module — neither gates in-scope exec verification.)

## AST Consistency
- AST check: **PASS** — no `// VERUS REWRITE` comments; no exec rewrites to verify.

## Verification
- verus: `make verify-kernel MODULE=mm::virt` → **exit 0**, status **`CHEATING_DETECTED`**.
  Summary (kernel-wide): `assume=0 external_body=23 admit=4 cfg_gate=19`, coverage 3/69.
  The module itself contributes 4 external_body, 0 admit, 0 assume.
- `make verify` (cross-module): cached PASS, exit 0 — no regression.
- **The non-zero exit masks the truth: the verifier explicitly flags `CHEATING_DETECTED`.**

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` = "None").
- True Bugs: **0**. The exec logic of all three functions is correct; both reviewers concur.
- Reconciliation: `bugs.md` is accurate — there is no *code* defect. However, the surviving
  unresolved condition is that the three in-scope functions are **unproven in-body** (deferred
  via TCB self-listing). This is correctly characterized in `verification-todo.md` as an
  infrastructure-blocked proof, not a code bug — but it is precisely why the effort cannot PASS.

## Issues (highest priority first)
1. **[BLOCKER] Verification targets moved into the TCB during the effort.** All three
   in-scope functions (`identity_map_page`, `ensure_pt`, `ensure_pte`) are
   `#[verus_verify(external_body)]` and were added to `tcb-allowed.md` in commit
   `a6d1b7778` — the same commit that swapped their `admit()` for `external_body`. This
   violates "the TCB is fixed in advance; no new trust boundaries may be introduced."
   Net result: **0 in-scope functions proven in-body**; the module is "verified" purely by
   assumption. Both independent reviewers flagged this as the decisive failure.
2. **[BLOCKER] Verifier reports `CHEATING_DETECTED`.** Although the exit code is 0, the
   tooling's own cheating gate is tripped (module owns 4 external_body). A clean pass
   requires the targets verified in-body, not trusted.
3. **[Process] Honest hand-off exists but does not change the verdict.**
   `verification-todo.md`/`bugs.md` candidly document that the obligations are deferred
   (unrealized `mm::virt` permission token, contents-free `Table::write`, unspecified
   `KERNEL_PD_PADDR` load, no establishing fact for `bump_view().inv()`). This is good
   transparency, but the correct outcome of an *unproven* module is **FAIL (unproven)**,
   not a PASS via TCB expansion.

## Result: **FAIL**

Rationale: Every mechanical counter is clean (admit=0, assume=0, no spec drift, AST OK,
exit 0) — but only because the three verification-target functions were themselves moved
into the trusted computing base *during* this effort (`external_body` + new `tcb-allowed.md`
entries in commit `a6d1b7778`), which the fixed-TCB / no-new-trust-boundary hard rule
forbids, and which the verifier independently flags as `CHEATING_DETECTED`. Nothing was
proven in-body for this module. Both independent reviewers (claude-opus-4.8 and
gpt-5.3-codex) reached FAIL independently. Multiple checklist items are unchecked;
per the strict criterion, the result is **FAIL**.
