# Final Independent Review — `hal::mem::types::address::frame` (`FrameAddress`)

Reviewer: Claude (independent, strict). Date: 2026-06-15.

Scope (ONLY): `FrameAddress` (type), `from_raw_value`, `into_raw_value`,
`from_frame_number`, `into_frame_number`. All other functions out of scope and
were not modified/judged.

Method: read all three module files, the dependency contracts they rely on
(`Address` trait spec in `sys`, `PageAligned`/`phys` specs), the four analysis
docs, and the five skills. Independently re-ran `ast_consistency.py`,
`spec_drift.py`, and all guardrail greps. Did not re-run `make verify`/
`./z build` (relied on central results to avoid cargo-lock contention).

---

## Checklist

| Dimension | Verdict | Justification |
|---|---|---|
| Caller Analysis | PASS | `caller_analysis.md` enumerates every in-scope fn, call-site counts, per-caller assume/break/don't-care; round-trip identities identified. Matches source on re-read. |
| View Design | PASS | `view_design.md` justifies `V = int` (physical address) + `inv()` = page-aligned via substitution test against all callers; single-field minimal View; rejected alternatives documented. Consistent with code. |
| Specification | PASS | All 4 in-scope fns carry `#[verus_spec]`; contracts are caller-abstract, correct, and complete for the documented caller expectations (see Spec Quality). |
| Proving | PASS | 0 `admit()`, 0 `assume()`; two elementary divisibility lemmas in `frame.proof.rs` are sound and used; central `make verify-kernel` exit 0, "No cheating detected". |
| Cheating Elimination | PASS | admit=0, assume=0, external_body=0, no `cfg(not(verus_keep_ghost))` on exec, no `// VERUS REWRITE`, no `uninterp`. Only `include!` lines are cfg-gated spec/proof includes (allowed). |
| Bug Recording | PASS | BUG-001 (duplicate `use ::vstd::prelude::*;`) recorded and confirmed fixed in source (single import at line 8). No new bugs found. |

All checklist items PASS.

---

## Spec Quality (assessment)

- `into_raw_value` — `ensures result as int == self@`. Exact, minimal, matches the
  `Address` trait contract it delegates to. The 19 callers need precisely the
  physical address for pointer math / MMU programming. ✅
- `from_raw_value` — `ensures Ok(fa) => fa.inv() && fa@ == raw_addr as int;
  Err(_) => true`. The `Ok` arm is exactly what `boot_init.rs` / `mm/phys` rely
  on. The `Err(_) => true` arm is tautological but **acceptable**: all 3 callers
  propagate with `?`, the function has no side effects, and a failed constructor
  produces nothing — per spec-design, an `Err`-tautology on a side-effect-free
  fallible constructor is the correct (non-over-specified) choice. ✅
- `from_frame_number` — `ensures result is Ok; (Ok).inv(); (Ok)@ ==
  spec_from_number(spec_frame_raw_value(frame_number))`. Stronger than the
  `Err => true` style: it proves the call **never fails**, matching reality (the
  body is total) and the 9 callers' expectation. `(Ok)@ == frame@ * page_size`
  is exactly the caller-required base-address identity. ✅
- `into_frame_number` — `requires self.inv(), spec_frame_number(self@) <=
  spec_max_frame_number(); ensures spec_frame_raw_value(result) ==
  spec_frame_number(self@), spec_from_number(spec_frame_raw_value(result)) ==
  self@`. The second `ensures` is the inverse identity `result@ * page_size ==
  self@` the callers depend on. The added precondition is genuinely necessary
  (inherited from `PhysicalAddress::into_frame_number`; not derivable from
  page-alignment alone) and is the minimal correct guard — correctly surfaced as
  `requires` rather than folded into `inv()` (which would strengthen the
  invariant relied on by already-verified `mm/phys` callers). ✅

No subsumed or redundant ensures. Round-trip identities
(`from_raw_value↔into_raw_value`, `from_frame_number↔into_frame_number`) are
derivable from the per-function view-level contracts and need not be stated
separately.

---

## Caller Coverage

**Covered: 5 / 5 in-scope entities (all enumerated caller properties covered).**

| In-scope entity | Caller expectation | Covered by |
|---|---|---|
| `from_frame_number` | `Ok`, `inv()`, `fa@ == n·PAGE_SIZE`, round-trips to `into_frame_number` | `result is Ok` + `(Ok).inv()` + `(Ok)@ == spec_from_number(spec_frame_raw_value(n))`; round-trip derivable ✅ |
| `into_frame_number` | `result·PAGE_SIZE == self@`, inverse of `from_frame_number` | `spec_frame_raw_value(result) == spec_frame_number(self@)` + `spec_from_number(...)==self@` ✅ |
| `from_raw_value` | `fa@ == raw_addr`, `inv()`; `Err` propagated, no frame | `Ok(fa) => fa.inv() && fa@ == raw_addr`; `Err(_)=>true` ✅ |
| `into_raw_value` | `result as int == self@`; inverse of `from_raw_value` | `result as int == self@`; round-trip derivable ✅ |
| `FrameAddress` (type) | always page-aligned, `Copy`, comparable, lossless conversions | `inv()` defined; `Copy` derived; conversions specified ✅ |

**Missing properties: none.** The two round-trip identities listed in
`caller_analysis.md` "Key Invariants" are not separate `ensures` but follow
transitively from the stated view-level contracts — acceptable.

---

## Proof Completeness

- `admit()`: **0** (none in `frame.rs`/`frame.spec.rs`/`frame.proof.rs`).
- `external_body`: **0** in-scope.
- `external_body` not in tcb-allowed: **0** (list empty).
- Proof obligations discharged by two reusable vstd-backed lemmas
  (`lemma_frame_base_aligned` via `lemma_mod_multiples_basic`;
  `lemma_aligned_div_mul` via `lemma_fundamental_div_mod`) — both sound,
  both used.

No BLOCKERS.

---

## TCB Compliance

**YES — compliant.** No `external_body` anywhere in the module. The single
`assume_specification` (`<PhysicalAddress as ::sys::mm::Address>::from_raw_value`,
`frame.spec.rs:20`) is recorded in `tcb-allowed.md` lines 154–168 with a
verified rationale: the sibling `phys` module's `impl Address for PhysicalAddress`
(`phys.rs:179–196`) is **not** annotated `#[verus_verify]`, so the concrete
`from_raw_value` carries no callable contract for Verus; the placeholder is
genuinely required for bottom-up proving and is scheduled for removal once `phys`
is verified.

Tracked note (not a violation): this is a *workspace-internal* placeholder, which
the generic verus-constraints guidance discourages, but it is explicitly
permitted under the bottom-up methodology because it is pre-recorded in the TCB
allowed list and will be superseded. It is sound — its `Ok` arm matches the
`Address` trait contract exactly and its `Err(_)=>true` arm is *weaker* than the
trait (assuming less is conservative). No new trust boundary was introduced by
this review phase.

---

## Guardrails Compliance

Independently re-derived via grep over the three frame files:

- admit: **0**
- assume: **0**
- external_body: **0**
- assume_specification: **1** (recorded in tcb-allowed.md — acceptable bottom-up placeholder)
- cfg-gated exec: **0** (the only `#[cfg(verus_keep_ghost)]` lines guard the
  `include!("frame.spec.rs")` / `include!("frame.proof.rs")` spec/proof includes —
  allowed; `external_derive` on the struct is a derive attribute, not `external_body`)

No guardrail BLOCKERS (admit=0, assume=0, all external_body/assume_specification accounted for).

---

## AST Consistency

**PASS.** `ast_consistency.py --base-ref 38885545d~1 frame.rs count` →
"✅ Consistent: 9 functions, 1 structs match." 0 mismatches. Independent grep
confirms **no** `// VERUS REWRITE` comments in any of the three files, so there
are no rewrites requiring semantic-equivalence justification. Exec signatures and
bodies are unchanged except the BUG-001 import removal (a pre-approved bug-fix
deviation).

Spec drift (`spec_drift.py git-diff`, 2 contract-drift items) — both reviewed and
**not** weakenings:
1. `from_raw_value`: old `Ok(fa)=>fa.inv()` → new `Ok(fa)=>fa.inv() && fa@ ==
   raw_addr`. The "ensures removed" flag is an artifact of the whole match-arm
   text changing; the new ensures **strengthens** (adds a conjunct that implies
   the old). ✅
2. `into_frame_number`: `requires` + `ensures` added to a function that was
   **previously unspecified** (no prior contract). Adding a necessary precondition
   to a contractless function establishes (does not weaken) a guarantee; the
   precondition is the minimal one inherited from
   `PhysicalAddress::into_frame_number`. ✅

---

## Verification

**PASS.** Central results (trusted, not re-run to avoid cargo-lock contention):
`make verify-kernel MODULE=hal::mem::types::address::frame` exit 0 with
"No cheating detected"; cross-module `make verify` exit 0 (all crates);
`./z build -- all-kernel` exit 0 (clean normal build). Module-local cheating
counts assume=0/external_body=0/admit=0/trusted=0/cfg_gate=0. Coverage 4/9 exec
functions have contracts — the 4 are exactly the in-scope functions.

---

## Bug Summary

- **Total recorded: 1** (BUG-001).
- **True bugs: 1** — BUG-001: duplicate `use ::vstd::prelude::*;` broke the
  normal `-D warnings` build (unused-import). **Severity: correctness (build
  breakage), low risk.** Auto-fixed by removing the redundant import; verified
  fixed (single `use vstd::prelude::*;` at `frame.rs:8`, matching siblings).
- **New bugs discovered this review: 0.** No surviving verification failures to
  classify.

---

## Issues (highest priority first)

1. *(Tracked, non-blocking)* The single workspace-internal `assume_specification`
   for `<PhysicalAddress as Address>::from_raw_value` must be removed once the
   `hal::mem::types::address::phys` `Address` impl gains its own `#[verus_spec]`.
   Already recorded in `tcb-allowed.md`; sound and conservative in the meantime.
2. *(Informational)* The `into_frame_number` precondition
   (`spec_frame_number(self@) <= spec_max_frame_number()`) imposes a new proof
   obligation on its 7 (currently unverified) callers. Expected and correct under
   bottom-up ordering; those modules must discharge it when verified.

No correctness, soundness, TCB, or guardrail blockers.

---

## Result: **PASS**

Every checklist dimension passes. Specs are correct, complete against documented
caller expectations, and caller-abstract; proofs are complete (0 admit / 0
assume / 0 in-scope external_body); the lone `assume_specification` is a sound,
pre-recorded bottom-up placeholder; AST is consistent; the two spec-drift items
are strengthening / newly-established (not weakenings); verification and build
are clean. No BLOCKERS.
