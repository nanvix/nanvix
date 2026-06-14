# Final Comprehensive Review: phys-mod

> Consolidation of two independent sub-agent reviews:
> - `final_review.claude.md` (model: **claude-opus-4.8**) — verdict **PASS**
> - `final_review.gpt-5.3-codex.md` (model: **gpt-5.3-codex**) — verdict **FAIL**
>
> The reviewers agree on every *hard guardrail* (cheating counts, TCB, AST,
> verification, spec-drift) and split only on **spec completeness of the failure
> paths**. This consolidation adjudicates each disputed point against the
> spec-design / verus-constraints / bug-reporting skills and the strict
> "all-boxes-checked" gate.
>
> Scope (ONLY): `init`, `book_mmio_regions`, `book_physical_memory_regions` in
> `src/kernel/src/mm/phys/mod.rs` (+ `mod.spec.rs`, `mod.proof.rs`). Sibling
> modules (`frame`, `kframe`, `manager`, `upool`) are separate targets; their
> admits/external_body are kernel-wide context only.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py`; sole caller `kernel_vas::init` (kernel_vas.rs:120)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified — global physical-memory subsystem (frame partition + manager/upool liveness)
- [x] Pre-existing specs assessed — `FrameAllocView`/`wf`/`View for Inner` reused, not re-derived

### View Design
- [x] Every field passes the substitution test (view_design §5: `initialized`, `frames`, `manager_ready` all survive a buddy/free-list rewrite)
- [x] All caller-observable state represented (no missing fields)
- [x] No implementation-specific fields (no bitmap/AtomicBool/refcount-slice)
- [x] inv() encodes real constraints (`initialized ==> frames.wf()`, `manager_ready ==> initialized`)
- [x] Mathematical types used (`Set<int>`, `Map<int,int>`; frame addresses as `int`)

### Specification
- [x] Every in-scope exec function has requires/ensures (`init` + both helpers; `fn_coverage` shows 4/4 in-scope annotated)
- [ ] **Caller coverage** — the caller_analysis **Key Invariant "Fail-fast: any booking conflict surfaces as `Err`"** has NO corresponding spec (no error-cause predicate, no liveness). 10/12 expectations covered (see Caller Coverage). **UNCHECKED — see Issue B1.**
- [x] View consistency — specs reference `phys_view()`/`FrameAllocView` fields and maintain `inv()`
- [ ] **No tautological ensures (e.g., `Err(_) => true`)** — the literal pattern `Err(_) => true` appears in all three functions (mod.rs:70, 100, 164). Mitigated by an *unconditional* `inv()` ensures, but the match arm itself is tautological and the stronger arms designed in view_design §4 were dropped. **UNCHECKED — see Issue B2.**
- [x] No subsumed ensures — `live()` is not implied by `inv()`; `all_reserved`/MMIO clause add genuine content
- [ ] **Error paths have meaningful ensures (match style)** — error paths carry only unconditional `inv()` (+`initialized` for helpers ⟹ `wf`); the failure-causality/liveness half is absent. **UNCHECKED — see Issue B1/B2.**
- [x] No assume_specification for workspace-internal code (0 in phys-mod)
- [x] vstd searched before any assume_specification (none used)
- [x] Specs written for the caller (the `Ok`-path facts are directly drop-in for the caller's proof)
- [x] Trait obligations satisfied (no traits implemented)
- [ ] **Spec completeness (advisory)** — dropped vs view_design: static one-shot `requires !initialized`, the composed `seed(..).book_all(P).book_covered(M)` transition, and the `!all_free(R)` conflict predicate. Sound to drop, but incomplete. **UNCHECKED (advisory) — see Issue B3.**
- [x] Loop invariants — the only loops live inside the two `external_body` helpers (bodies not verified → no invariant obligation); `init` has no loop
- [x] No cheating on module's own functions — phys-mod `admit=0 assume=0 external_body=2 (both TCB-listed) trusted=0`
- [x] No specs weakened — `spec_drift.py git-diff … --before HEAD`: **"No contract drift detected" (0 functions changed)**
- [x] Bug awareness — `bugs.md` present and reconciled (no code defect; one labeled verifier limitation)
- [x] Cross-module regression — `make verify` exit **0** (all verified modules pass)
- [x] Verification — `make verify-kernel MODULE=mm::phys` exit **0**, 0 errors (`make build` is a no-op: a `build/` dir shadows the phony target; the Verus compile+verify gate is `verify-kernel`)

### Proving
- [x] No specs weakened (`spec_drift.py`) — no drift
- [x] Zero remaining `admit()` (phys-mod: 0)
- [x] Zero `external_body` unless listed in `tcb-allowed.md` — the 2 phys-mod `external_body` (`book_physical_memory_regions` mod.rs:73, `book_mmio_regions` mod.rs:103) are pre-listed (tcb-allowed.md L74/L79)
- [x] Zero assume/assume_specification (phys-mod: 0)
- [x] No cfg-gated exec code (only `#[cfg(feature="test")]` test module + `#[cfg(verus_keep_ghost)]` spec/proof includes)
- [x] Cheating audit — counts + locations reported (Guardrails section)
- [x] Any claimed Verus limitation has an isolated reproducer — LinkedList/orphan-rule (E0117) limitation documented in bugs.md
- [x] Exec rewrites minimal and semantically equivalent — **0 `// VERUS REWRITE`**; AST 4/4 MATCH
- [x] Cross-module regression — `make verify` exit 0
- [x] Verification — `make verify-kernel` exit 0

### Cheating Elimination
- [x] Zero `admit()` remaining (phys-mod)
- [x] Zero `assume()` remaining (phys-mod)
- [x] Zero trusted functions
- [x] Zero `exec_allows_no_decreases_clause`
- [x] Zero cfg-gated exec code
- [x] Zero `external_body` unless listed in `tcb-allowed.md` (2, both listed)
- [x] AST consistency: zero mismatches (4 functions MATCH, 0 structs)
- [x] All exec rewrites have VERUS REWRITE comment + minimal reproducer (N/A — 0 rewrites)
- [x] Each surviving `external_body` confirmed in `tcb-allowed.md`
- [x] No specs weakened (`spec_drift.py`) — no drift
- [x] Cross-module regression — `make verify` exit 0
- [x] Verification — `make verify-kernel` exit 0

### Bug Recording
- [x] `bugs.md` exists
- [x] Each bug is a real code defect — the LinkedList entry is explicitly labeled "Verifier limitation (**not** a code bug)"; "Code bugs: None found"
- [x] Each bug entry has What / Why / How Verus Helped / Severity / Suggested Fix (limitation entry has What/Why/Consequence/Resolution/How-Verus-helped)
- [x] No `external_body` used to mask a code defect (helper bodies are simple correct iteration with meaningful contracts)
- [x] Bug entries include provenance (specification phase)

## Spec Quality
The public API contract (`init`, the only externally-observed function) is **sound,
declarative, written-for-the-caller, and substitution-test-clean** on the success
path: `Ok ⟹ live() ∧ all_reserved(phys_regions_frame_set) ∧ (covered MMIO ⟹ reserved)`.
These are exactly the caller-relied-upon facts, phrased over abstract `Set<int>`
frame sets (no bitmap/atomic/refcount-slice leak). The two private helpers carry
appropriately-scoped contracts.

**Adjudicated weakness (the reviewers' split): the failure paths.** All three
functions use `Err(_) => true` paired with an unconditional `inv()` ensures
(helpers also `initialized`). This *does* preserve `wf` on error (so it is **not**
the pure "error arm == nothing" anti-pattern), but it drops the failure-causality
and liveness that `view_design.md` §4 explicitly designed (`Err ⟹ !all_free(R)`,
explicit `wf`) and that `caller_analysis.md` lists as the **"Fail-fast" Key
Invariant**. Dropping an *assumed* error-cause contract on an `external_body`
helper is defensible (cf. the `table::write` soundness caution in tcb-allowed.md),
but the result is an **incomplete failure spec** against the documented caller
expectation — the basis of the strict FAIL below.

**`uninterp spec fn` (4) — RESOLVED, not a verification escape.** Both reviewers
agree on `phys_view()` (singleton-global ghost accessor, pinned by the TCB
`frame::instance`) and `phys_regions_frame_set`/`mmio_regions_frame_set` (direct
mechanical consequence of the foreign, un-modelable `LinkedList` external type).
On `byte_at_address`: codex flagged it; **adjudication — it is in the explicit
do-not-modify list (a pre-existing, protected external-bottom raw-memory
accessor), out of this effort's modification scope, so it cannot be a blocker for
phys-mod.** Verdict: all four acceptable.

## Caller Coverage
- Covered: **10 / 12** caller expectations (≈ 5/5 of the named Key Invariants are
  *partially* met: one-shot is enforced dynamically not statically; fail-fast is
  not encoded).
- Missing:
  - **Fail-fast causality** — no `Err`-side conflict predicate (`!all_free(R)`) and
    no liveness (all-free ⟹ `Ok`); the caller cannot prove "a booking conflict is
    *the* reason for `Err`" nor "a fully-free input *succeeds*". (caller_analysis
    "Key Invariants → Fail-fast")
  - `book_physical_memory_regions` `Err` failure-condition (dropped to `true`)
  - `book_mmio_regions` `Err` failure-condition (dropped to `true`)
- Sound-but-incomplete (not "missing guarantee", advisory): static one-shot
  `requires !initialized`; exact `seed(..).book_all(P).book_covered(M)` post-state;
  MMIO "uncovered-frame-unchanged" half. None weakens a *consumed* guarantee.

## Proof Completeness
- Remaining `admit()` (phys-mod): **0** — no blocker.
- Remaining `external_body` not in `tcb-allowed.md`: **0** — the 2 phys-mod
  `external_body` (`book_physical_memory_regions`, `book_mmio_regions`) are both
  pre-listed (tcb-allowed.md L74/L79). `init` is **body-verified**.
- Kernel-wide context (OUT OF SCOPE — sibling modules still in progress):
  `admit=27 external_body=18 cfg_gate=15`.

## TCB Compliance
- All phys-mod `external_body` listed in `tcb-allowed.md`: **YES** (2/2). No new
  trust boundary introduced.
- `ExLinkedList` `external_type_specification` (mod.spec.rs:69) — the
  skill-prescribed mechanism for a foreign std type; counted *separately* from
  `external_body` (not in the 18). Advisory: it lacks its own dedicated
  allowed-list line (justified only in the prose of the two `book_*` TCB entries).
  Not a violation.

## Guardrails Compliance
phys-mod (the three in-scope files) — exact counts:
- **admit: 0, assume: 0, external_body: 2** (both TCB-listed), **assume_specification: 0, cfg-gated exec: 0**
- (also: `external_type_specification: 1` ExLinkedList; `uninterp: 4` all justified; `// VERUS REWRITE: 0`; trusted: 0)

Kernel-wide totals (context only): `assume=0 external_body=18 admit=27 trusted=0 no_decreases=0 cfg_gate=15`.

## AST Consistency
- AST check: **PASS** — `✅ Consistent: 4 functions, 0 structs match`; summary: `init`,
  `book_mmio_regions`, `book_physical_memory_regions`, `test` all MATCH. No
  `// VERUS REWRITE` comments. `spec_drift`: no contract drift.

## Verification
- verus: **PASS** — `make verify-kernel MODULE=mm::phys` exit **0** (0 errors);
  `make verify` (cross-module) exit **0**. (`status: CHEATING_DETECTED` reflects
  the kernel-wide sibling-module admit/external_body tally, not a phys-mod
  verification error; phys-mod contributes admit=0.)

## Bug Summary
- Total bugs recorded: **1 entry**, correctly classified as a **verifier
  limitation, NOT a code bug** (LinkedList has no vstd model; orphan rule E0117
  blocks providing one — justifies the 2 `external_body` + `ExLinkedList`).
- True Bugs: **0**. No code defect (no overflow/off-by-one/impossible path) in the
  three target functions; no bug masked by `external_body`; no unrecorded
  verification failure.

## Issues (highest priority first)
- **B1 (BLOCKER, spec completeness) — Fail-fast caller invariant not encoded.**
  `caller_analysis.md` lists "Fail-fast: any booking conflict surfaces as `Err`"
  as a Key Invariant, but no spec captures the failure cause or liveness. Add (at
  minimum) the `view_design` §4.2 abstract conflict predicate to
  `book_physical_memory_regions`' `Err` arm and a corresponding liveness/`Err`
  characterization to `init`/`book_mmio_regions`, OR record an explicit,
  justified design decision (e.g. "no assumed error-causality on `external_body`")
  in `bugs.md`/`view_design.md` so the dropped Key Invariant is accounted for.
- **B2 (BLOCKER, checklist literal) — `Err(_) => true` in all three functions
  (mod.rs:70, 100, 164).** The checklist's exact anti-pattern example. Even though
  an unconditional `inv()` preserves `wf`, the match `Err` arm is tautological and
  the designed stronger arms were dropped. Strengthen the `Err` arms (e.g. helpers:
  state `frames.wf()` / the conflict predicate explicitly inside the match) so the
  failure semantics are visible at the match site rather than relying solely on a
  separate unconditional clause.
- **B3 (advisory) — init spec richness reduced vs view_design.** Static one-shot
  `requires !initialized`, the composed `seed(..).book_all(P).book_covered(M)`
  post-state, and the MMIO "uncovered-unchanged" half were dropped. Sound and
  caller-safe, but an over-reserving / re-entrant `init` would still pass.
- **B4 (advisory) — `ExLinkedList` lacks a dedicated `tcb-allowed.md` line.**
  Documentation tidiness; the mechanism is skill-sanctioned.
- **Note — codex's "`byte_at_address` blocker" is rejected:** it is a pre-existing
  do-not-modify external-bottom accessor, out of modification scope; not a
  phys-mod defect.

### Reviewer disposition
- All **hard guardrails** (admit=0, assume=0, external_body⊆TCB, AST 0-mismatch,
  no spec-drift, `make verify`/`verify-kernel` exit 0) — **both reviewers agree:
  PASS.** The module is sound and faithfully verified.
- The split is purely on **failure-path spec completeness** (B1/B2). Under the
  stated strict gate ("PASS only if ALL checklist items are checked"), the
  literal `Err(_) => true` anti-pattern and the uncaptured "Fail-fast" caller Key
  Invariant leave Specification-section boxes unchecked.

## Result: FAIL

**Rationale (strict gate):** every hard soundness/guardrail item passes cleanly —
no `admit`, no `assume`, both `external_body` pre-approved in the TCB, AST
4/4 consistent, zero spec-drift, and `make verify` / `make verify-kernel` at exit
0. The FAIL is driven **solely** by spec-completeness of the failure paths: the
literal `Err(_) => true` anti-pattern in all three functions (checklist item
"No tautological ensures") and the uncaptured caller-perspective **"Fail-fast"
Key Invariant** (checklist item "Caller coverage" / "Error paths have meaningful
ensures"). These are addressable by strengthening the `Err` arms (Issues B1/B2)
or by recording an explicit justification for the deliberate reduction; once
either is done, the remaining items are all green and the module is PASS-ready.
