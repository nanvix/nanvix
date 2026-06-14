# Final Comprehensive Review — `arch::x86::mem::paging::pte`

- Reviewer: independent strict final review (Claude)
- Date: 2026-06-15
- Repo: `/home/ruize/nanvix-phy-specs-bottom-up`
- Branch: `verus-ai-prove-bottom-up`
- Baseline (pre-work) commit: `61b47c783` (`[arch] E: Add copy-on-write flag to PTEs`),
  i.e. the parent of `e86feeddb` (`caller-analysis START`).
- In-scope functions (ONLY): `PageTableEntryFlags::new`, `PageTableEntry::new`,
  `PageTableEntry::is_present`, `PageTableEntryFlags::is_present`.

---

## Checklist

### Caller Analysis
- [x] `caller_analysis.md` present, identifies all real call sites in `kernel`.
- [x] All 4 in-scope functions have documented caller expectations.

### View Design
- [x] `view_design.md` present; `PteFlagsView` (8 bools) and `PteView` (`flags`, `frame:int`) defined.
- [x] View is `closed` (encoding hidden); fields are caller-visible only.
- [x] `inv()` chosen and justified (frame bound; vacuous flags inv).

### Specification
- [x] All 4 in-scope exec functions carry `#[verus_spec]` contracts bound to exec code.
- [x] Ensures reference View fields (`result@`, `self@.present`, `self@.flags.present`).
- [x] No tautologies, no subsumed clauses, no one-sided/operational specs.
- [x] No spec drift vs baseline (drift checker exit 0).

### Proving
- [x] `make verify-arch` exits 0; pte module verifies (47 verified, 0 errors).
- [x] Zero `admit()` in pte.rs/spec/proof.

### Cheating Elimination
- [x] pte.rs/spec/proof: 0 admit, 0 assume, 0 external_body, 0 assume_specification, 0 cfg-gated exec.
- [x] Crate-wide external_body (3) all in `tcb-allowed.md`.
- [x] AST consistency PASS (23 fns, 2 structs MATCH).

### Bug Recording
- [x] No `bugs.md` exists; no bugs found during review — correctly nothing to record.

---

## Spec Quality

The 4 external-top API contracts are correct, complete, and understandable.

- `PageTableEntryFlags::new` — `ensures result@ == spec_pte_flags_new(present, …, dirty)`.
  `spec_pte_flags_new` pins all eight View bits: the seven argument bits via the
  two-valued projection helpers (`spec_present_set`, `spec_rw_set`, …) and the
  OS-defined `cow` bit to `false` (`NotCopyOnWrite`). This is load-bearing
  (rejects a `new` that leaks a stale cow bit) and is **not** one-sided — every
  argument is recorded, so a buggy `new` dropping any argument fails the spec.
- `PageTableEntry::new` — `ensures result@ == spec_pte_new(flags@, frame@)` and
  `result.inv()`. `spec_pte_new` pairs the exact `flags@` with the exact `frame@`
  (`PteView { flags, frame }`). `inv()` is meaningful: `0 <= self@.frame <=
  FrameNumber::spec_max()`, inherited from the `FrameNumber` type invariant
  (discharged in the proof via `use_type_invariant(frame)`). Not a tautology, not
  subsumed.
- `PageTableEntry::is_present` — `ensures result == self@.flags.present` (presence
  delegation, a one-line View identity).
- `PageTableEntryFlags::is_present` — `ensures result == self@.present` (pure
  projection).

`inv()` quality: `PageTableEntry::inv` is non-vacuous and well-motivated (keeps the
out-of-scope `frame_address` total/overflow-free). `PageTableEntryFlags::inv` is
vacuously `true`, correctly justified — neither hardware nor OS imposes any
cross-flag coupling, and `unmap`/`set_*` callers exercise every bit combination.

`view()` is `closed` (hides bit-packing) per spec-design Part 3; reusable helpers
(`spec_*_set`, `spec_pte_*`) live as free spec fns / on the View, not as extra pub
spec fns on the exec impl. All ensures pass the caller-substitution test: each is
directly usable in a caller proof without transformation.

No anti-patterns found: no exec mutation, no verification escape, no operational
spec, no tautological/subsumed clauses, no missing frame condition (constructors
are immutable value-builders, so struct-update frame conditions are N/A).

---

## Caller Coverage

**Covered: 4 / 4 in-scope functions.**

| Caller expectation | Captured by |
|---|---|
| Flags::new reflects all 7 args; `is_present == (present==Present)`; `cow` defaults `NotCopyOnWrite` | `result@ == spec_pte_flags_new(...)` (pins all 8 bits incl `cow:false`) |
| Flags::new total/infallible | no `requires`, `-> Self` (no error path) |
| Entry::new stores flags+frame faithfully; `is_present==flags.is_present`, `frame_number==frame`, `flags()` equivalent | `result@ == spec_pte_new(flags@, frame@)` (derives all three) |
| Entry::new infallible, immediately serializable | no `requires`, `-> Self`; `result.inv()` keeps frame well-formed |
| Entry::is_present == `self.flags().is_present()`, pure | `result == self@.flags.present` |
| Flags::is_present == constructed-with-Present, pure | `result == self@.present` |

**Missing important properties: none** (for the 4 in-scope functions).

Note (not a gap): the `TableEntry` round-trip obligation (`from_raw`/`raw` inverse)
is explicitly **out of scope** — `from_raw_value`/`into_raw_value` are unspecified
by design (confirmed in caller_analysis.md and view_design.md). It is a boundary
obligation on out-of-scope functions, not a missing contract on the 4 in-scope fns.

---

## Proof Completeness

- `admit()` count in pte.rs/spec/proof: **0** (locations: none).
- `external_body` on pte's own functions: **0**.
- `external_body` not in tcb-allowed.md: **0** (locations: none).
- pte.proof.rs is an empty `verus! { }` — all proofs discharge automatically; the
  only proof obligation (`new`'s `inv()`) is met by an in-line `proof! {
  use_type_invariant(frame); }` in the exec body (pte.rs:318), which is a proof
  macro, not exec logic.

---

## TCB Compliance

**Compliant: YES.**

Crate-wide `external_body` in `arch` (from `cheating-detail.txt`): exactly 3, all
listed in `tcb-allowed.md`:

- `x86/mem/paging/mod.rs:80` `invlpg` — listed (inline-asm TLB flush, empty contract). ✓
- `x86/mem/paging/table.rs:209` `Table::read` — listed (usize→ptr volatile read). ✓
- `x86/mem/paging/table.rs:246` `Table::write` — listed (usize→ptr volatile write). ✓

`pte` itself contributes **ZERO** `external_body` — as required.

---

## Guardrails Compliance

Exact counts.

**pte.rs / pte.spec.rs / pte.proof.rs (in-scope module):**
| Dimension | Count | Locations |
|---|---|---|
| `admit()` | 0 | — |
| `assume(...)` | 0 | — |
| `external_body` | 0 | — |
| `assume_specification` | 0 | — |
| cfg-gated EXEC code (`cfg(not(verus_keep_ghost))` on exec) | 0 | — |
| `#[cfg(verus_keep_ghost)] include!(...)` (allowed spec/proof includes) | 2 | pte.rs:9, pte.rs:11 (NOT exec gating — allowed) |

**Crate-wide (`arch`), from verifier `cheating-detail.txt`:**
| Dimension | Count | Locations |
|---|---|---|
| `assume` | 0 | — |
| `admit` | 0 | — |
| `external_body` | 3 | mod.rs:80 invlpg, table.rs:209 read, table.rs:246 write (all TCB-allowed) |
| `trusted` | 0 | — |
| `no_decreases` | 0 | — |
| `cfg_gate` (counted) | 2 | pde.rs:83, pde.rs:307 — `#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]` (benign lint `allow`, NOT exec gating; in sibling out-of-scope module) |
| `assume_specification` | 0 | — |

There is **no** `cfg(not(verus_keep_ghost))` exec-gating anywhere in the `arch`
crate (verified by grep). The 8 `#[cfg(verus_keep_ghost)] include!` lines across
mod/pde/pte/table are spec/proof includes, excluded by the checker, and explicitly
allowed.

Verdict: admit=0, assume=0 → no BLOCKER. All `external_body` in TCB → no BLOCKER.

---

## AST Consistency

**PASS.** `ast_consistency.py --base-ref 61b47c783 pte.rs count`: ✅ Consistent —
23 functions, 2 structs all MATCH (matched=23, mismatched=0, missing=0, extra=0).
No `// VERUS REWRITE`, `// VERUS DEVIATION`, or `// VERUS BUG FIX` comments exist in
pte.rs (grep: none), so there are no rewrite-equivalence claims to audit. Exec code
is byte-for-byte semantically unchanged from baseline.

Spec drift (`spec_drift.py git-diff … --before 61b47c783`): exit 0 — 0 contract
drift (0 ensures removed, 0 requires added). 15 functions added (the new specs),
which is strengthening, not drift.

---

## Verification

**PASS.** `make verify-arch` (forced fresh, cache busted):

```
verification results:: 47 verified, 0 errors
Exit code : 0
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```

The module `x86::mem::paging::pte` is verified (`note: verifying module
x86::mem::paging::pte`) with 0 errors, 0 warnings. The verifier's overall
`status: CHEATING_DETECTED` label is solely due to the 3 TCB-allowed `external_body`
and the 2 benign pde `allow` cfg_attrs — none in pte, none disallowed.

---

## Bug Summary

- Total recorded: **0** (no `bugs.md` exists; per bug-reporting skill, absence of
  bugs needs no file unless one was created — none was).
- True Bugs: **0**.
- Context-Dependent: **0**.
- False Positives: **0**.

No verification failures arose during this review (verification passes cleanly), so
there is nothing to classify. No latent bug was observed in the 4 in-scope
functions: each constructor faithfully records its inputs and each query is a pure
projection, consistent with both the source and the specs.

---

## Issues (highest priority first)

None. No blockers, no correctness issues, no spec-quality issues, no guardrail
violations within scope.

(Informational, non-blocking, out-of-scope: the sibling `pde.rs` carries 2
`cfg_attr(verus_keep_ghost, allow(...))` lint attributes counted as `cfg_gate=2`
crate-wide. These are lint allowances, not exec gating, and lie outside this
review's scope.)

---

## Result: PASS

Every checklist item passes:
- Spec quality: PASS (4/4 contracts correct, complete, View-bound, no anti-patterns).
- Caller coverage: PASS (4/4 covered, 0 missing).
- Proof completeness: PASS (admit=0, external_body-in-pte=0, external_body-not-in-TCB=0).
- TCB compliance: PASS (3 external_body, all listed; pte has 0).
- Guardrails: PASS (admit=0, assume=0, assume_specification=0, no exec cfg-gating).
- AST consistency: PASS. Spec drift: PASS.
- Verification: PASS (47 verified, 0 errors, exit 0).
- Bugs: PASS (none found, none expected).

**No BLOCKERS found.**
