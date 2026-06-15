## Turn 1: Full Checklist Walkthrough — arch-frame-number final review

### Progress
- Done (PASS, evidence below): **ALL** checklist items across Caller Analysis,
  View Design, Specification, Proving, Cheating Elimination, and Bug Recording.
- Current: full sweep complete.
- Remaining: none.

### Module under review
- Source: `src/libs/arch/src/x86/mem/paging/frame/number.rs`
- Spec:   `src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`
- Proof:  `src/libs/arch/src/x86/mem/paging/frame/number.proof.rs`
- In-scope exec fns: `FrameNumber::into_raw_value`, `FrameNumber::from_raw_value`
  (+ the type `FrameNumber`). Two `#[test]` fns are de-facto callers, out of scope.

### Verification evidence (run this turn)
- `make verify-arch` → **Exit 0**, `status: CLEAN`.
  - Cheating summary: `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
  - Verus log: `verification results:: 6 verified, 0 errors` (no warnings emitted).
  - Crate compiled ("Finished dev profile") — build embedded in verify.sh succeeds.
- `fn_coverage.py number.rs number.rs` → Source exec fns 4, Matched 4, Missing 0,
  Extra 0. Both in-scope fns (`from_raw_value`, `into_raw_value`) carry contracts.
- Cheating grep over the 3 module files for
  `admit|assume|external_body|trusted|exec_allows_no_decreases`:
  only hit is the word "assumed" inside a comment in `number.spec.rs:9`. No real
  cheating constructs. No loops. No `cfg(` except the two ghost `include!`
  directives (`#[cfg(verus_keep_ghost)]` on spec/proof includes — allowed ghost,
  not exec). No `// VERUS REWRITE` (none needed; no exec rewrites).

---

### Caller Analysis — PASS
- **Callers searched (tool-verified):** `caller_analysis.md` records output of
  `find_callers_lsp.py` (rust-analyzer LSP): `from_raw_value` 8 external callers,
  `into_raw_value` 12 external callers, type `FrameNumber` 36 references. I
  independently re-grepped: real `FrameNumber` call sites confirmed in `pde.rs`,
  `pte.rs`, `page_table.rs`, `identity_map.rs`, `mm/phys/frame.rs`, `phys.rs`.
- **Caller expectations (success + failure) documented:** Yes — per-function
  success (index preservation, range bound, round-trip) and failure (`None` is
  the *only* failure signal; no panic) captured in `caller_analysis.md`.
- **Abstract resource identified:** "abstract identity of one physical page
  frame — bounded non-negative integer index `0 ..= MAX`."
- **Pre-existing specs assessed:** Yes — boundary contracts in `phys.spec.rs` /
  `tcb-allowed.md` for both fns assessed; in-module specs designed to coincide
  with `spec_max_frame_number()`.

### View Design — PASS
- **Substitution test:** documented in `view_design.md`; single field
  `self@ : int` survives a complete rewrite (raw usize / address/FRAME_SIZE /
  PTE-field unshift all project the same index). ✅
- **All caller-observable state represented:** the one observable quantity (frame
  index) is `self@`; range/round-trip/NULL/MAX are properties/derivations, not
  state. ✅
- **No implementation-specific fields:** `view()` is `closed`, hides the
  `usize`-newtype mapping. ✅
- **`inv()` encodes real constraints:** `0 <= self@ && self@ <= spec_max_frame_number()`
  — non-trivial, load-bearing for caller no-overflow proofs. ✅
- **Mathematical types:** `type V = int`; bound via `spec_max_frame_number() -> int`. ✅

### Specification — PASS
- **requires/ensures on every in-scope exec fn:** both fns have `ensures`
  (`fn_coverage` Matched 4/4; both targets specced). No `requires` needed
  (`from_raw_value` validates a dynamic runtime value → `None`, not a precond).
- **Caller coverage:** `from_raw_value` ensures
  `(Some) <==> value <= spec_max_frame_number()` + `Some(f) ==> f@==value && f.inv()`
  matches every caller expectation incl. `phys.rs:211` total `unwrap()`.
  `into_raw_value` ensures identity + range, matching `<<FRAME_SHIFT` / `*FRAME_SIZE`
  no-overflow callers.
- **View consistency:** specs reference `self@`, `f@`, `inv()`, and
  `spec_max_frame_number()` — exactly the View vocabulary; `from_raw_value`
  re-establishes `inv()`.
- **No tautological / no subsumed ensures:** `into_raw_value`'s range ensures is
  **NOT subsumed** — `into_raw_value` consumes `self` by value, so after the call
  the caller can no longer apply `use_type_invariant` to recover the bound on the
  returned `usize`; the explicit range ensures is the only way the bound reaches
  the caller. Genuinely needed.
- **Error paths meaningful:** `from_raw_value` failure is encoded as the
  bidirectional `<==>` (not `Err(_) => true`).
- **No assume_specification for workspace-internal code:** module files contain
  none. (The historical `phys.spec.rs` boundary `assume_specification`s are
  caller-side, out of this module's scope, and are now superseded by these real
  in-module specs — strictly stronger, so no caller regression.)
- **vstd / trait obligations:** only derived `Debug/Clone/Copy`; no trait
  semantic contract beyond duplication, satisfied by scalar View equality.
- **Specs written for the caller:** directly usable (identity + range + iff).
- **Loop invariants:** no loops in module. N/A → PASS.
- **No cheating on own functions:** grep counts 0 for admit/assume/external_body/trusted.
- **No specs weakened (spec drift):** new in-module contracts are equal-or-stronger
  than the upstream `tcb-allowed` boundary contracts (add exact identity
  `result as int == self@` / `f@ == value as int` on top of the range/iff facts).
- **Bug awareness:** no code defect found; no `bugs.md` required.
- **Cross-module regression:** `make verify-arch` verifies the whole arch crate →
  6 verified, 0 errors.
- **Verification + build:** Exit 0, 0 errors, 0 warnings.

### Proving — PASS
- **No specs weakened:** see spec-drift above.
- **Zero admit():** grep 0.
- **Zero external_body (unless TCB-listed):** grep 0; nothing to list.
- **Zero assume/assume_specification:** grep 0 (only the comment word "assumed").
- **No cfg-gated exec code:** only `#[cfg(verus_keep_ghost)]` on ghost `include!`
  of spec/proof — allowed.
- **Cheating audit counts:** admit 0, external_body 0, assume 0, cfg-gated exec 0.
- **Claimed Verus limitations:** none claimed; no reproducers needed.
- **Exec rewrites minimal:** none present; `into_raw_value` uses a legitimate
  `proof! { use_type_invariant(&self); }` (proof block, not a rewrite).
- **Cross-module regression / Verification + build:** as above, clean.

### Cheating Elimination — PASS
- admit 0, assume 0, trusted 0, exec_allows_no_decreases 0, cfg-gated exec 0,
  external_body 0 (all grep-verified across the 3 files).
- **AST consistency:** `fn_coverage` Missing 0 / Extra 0 — exec surface matches.
- **No external_body to mask defects:** none exist.
- **No specs weakened / cross-module / verification:** clean as above.

### Bug Recording — PASS
- No bugs found (no logic error, safety violation, or incorrect behavior). The
  code is a straightforward validated newtype with a correct `> MAX` reject path.
- Per checklist, **no `bugs.md` is required when no bugs exist.** ✅

### Verdict
All checklist items are PASS with concrete tool evidence. No fix requests.
Writing STOP = RESOLVED.
