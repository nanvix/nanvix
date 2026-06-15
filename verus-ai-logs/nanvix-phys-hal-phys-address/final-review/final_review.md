# Final Comprehensive Review: hal-phys-address

Consolidated from two independent sub-agent reviews:
- `final_review.claude.md` (claude-opus-4.8)
- `final_review.gpt-5.3-codex.md` (gpt-5.3-codex)

Both reviewers independently re-derived all claims (verification re-run, AST
equivalence, overflow/totality proofs) and **both returned PASS with zero
blockers**. In-scope targets: the type `PhysicalAddress` (`View` + `inv`),
`from_number`, `into_frame_number`, `from_mmio_address`.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `caller_analysis.md` derived from `find_callers_lsp.py` (rust-analyzer LSP); leaf module, no external dependents.
- [x] Caller expectations (success + failure) documented for each pub function — success/total + failure arms documented for all 3 in-scope fns.
- [x] Abstract resource identified — "validated integer handle to a guest-physical byte location"; frame = `addr / FRAME_SIZE`.
- [x] Pre-existing specs assessed (if any exist from upstream verification) — clean slate (`phys.spec.rs`/`phys.proof.rs` were empty `verus!{}`).

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite) — single scalar `view = self.0@ : int`; survives newtype removal.
- [x] All caller-observable state represented (no missing fields) — raw address is the only observable; frame index is derived (`spec_frame_number`).
- [x] No implementation-specific fields (only caller-observable state) — inner `VirtualAddress` hidden via `closed` view.
- [x] inv() encodes real constraints (not trivially true) — `spec_frame_number(self@) <= spec_max_frame_number()` (load-bearing: makes `into_frame_number` unwrap total).
- [x] Mathematical types used (int/Seq/Set/Map; exception: addresses keep usize) — `type V = int`; foreign `FrameNumber` projected by uninterp `spec_frame_raw_value`.

### Specification
- [x] Every in-scope exec function has requires/ensures (`fn_coverage`) — 3/3 in-scope carry `#[verus_spec]`; 13 untouched out-of-scope fns intentionally uncontracted.
- [x] Caller coverage: each caller expectation has corresponding requires/ensures/inv — 10/10 (claude) / 11/11 (codex), 0 gaps.
- [x] View consistency: specs reference View fields and maintain inv() — all contracts stated over `self@` / `spec_frame_number`; both constructors establish `inv()`.
- [x] No tautological ensures (e.g., `Err(_) => true`) — none; `from_mmio_address` uses `result is Ok` + identity.
- [x] No subsumed ensures (derivable from inv() + other ensures) — `from_number` alignment clause is derivable but retained as deliberate caller convenience (avoids nonlinear mod lemma at call site); not a violation.
- [x] Error paths have meaningful ensures (Ok => ..., Err => ...) — `from_mmio_address` always-Ok with identity; `into_frame_number` total via `inv()`; `from_number` infallible `-> Self`.
- [x] No assume_specification for workspace-internal code — all 6 are at the `arch`/`sys` library edge (not yet Verus-enabled).
- [x] vstd searched before any assume_specification — div/mod/pow2/shr lemmas drawn from `vstd`; assume_specifications only for non-vstd arch/sys items.
- [x] Specs written for the caller (usable directly in caller proofs) — `from_number`/`into_frame_number` round-trip + `from_mmio` identity directly usable by `FrameAddress`/allocator callers.
- [x] Trait obligations satisfied (specs match trait-level semantic contracts) — `Address::is_aligned` expectation met via `from_number` alignment ensures.
- [x] Spec completeness (advisory) — no unintended nondeterminism; from_mmio determinism matches caller expectation.
- [x] Loop invariants — no loops in any in-scope function (N/A).
- [x] No cheating on module's own functions — admit=0, assume=0, trusted=0; only `ExFrameNumber` external_body (allowlisted).
- [x] No specs weakened — `spec_drift.py git-diff --before HEAD`: "No contract drift detected" (0 ensures removed, 0 requires added).
- [x] Bug awareness — no fundamentally incorrect code; overflow/panic proven impossible.
- [x] Cross-module regression: `make verify` — exit 0, all verified modules pass.
- [x] Verification: `make verify-kernel` + `make build` — exit 0; module CLEAN.

### Proving
- [x] No specs weakened — spec_drift clean (see above).
- [x] Zero remaining admit() — 0.
- [x] Zero external_body unless listed in tcb-allowed.md — sole `ExFrameNumber` is listed.
- [x] Zero assume/assume_specification beyond external-bottom boundaries — assume=0; 6 assume_specification all arch/sys edge, all in tcb-allowed.md.
- [x] No cfg-gated exec code — 3 `cfg(verus_keep_ghost)` gates cover only `include!`/`use`; 0 exec branches/expressions/match arms gated.
- [x] Cheating audit (exact counts + locations) — see Guardrails section.
- [x] Any claimed Verus limitation has an isolated reproducer — `ExFrameNumber` opacity & arch-edge constants documented in tcb-allowed.md (orphan-rule / unsupported-constant).
- [x] Exec rewrites minimal and semantically equivalent (`// VERUS REWRITE`/`DEVIATION`) — 2 deviations, both independently re-derived equivalent.
- [x] Cross-module regression: `make verify` — pass.
- [x] Verification: `make verify-kernel` + `make build` — 0 errors.

### Cheating Elimination
- [x] Zero admit() remaining — 0.
- [x] Zero assume() remaining — 0.
- [x] Zero trusted functions — 0.
- [x] Zero exec_allows_no_decreases_clause — 0 (no_decreases=0).
- [x] Zero cfg-gated exec code — confirmed (only include!/use).
- [x] Zero external_body unless allowlisted — only `ExFrameNumber`, allowlisted.
- [x] AST consistency: zero mismatches — 2 flagged are pre-approved semantically-equivalent let-binding rewrites; no genuine mismatch.
- [x] All exec rewrites have VERUS REWRITE/DEVIATION comment and minimal reproducer — present at phys.rs:156, 199.
- [x] Each surviving external_body confirmed in tcb-allowed.md — `ExFrameNumber` ✓.
- [x] No specs weakened — spec_drift clean.
- [x] Cross-module regression: `make verify` — pass.
- [x] Verification: `make verify-kernel` + `make build` — 0 errors.

### Bug Recording
- [x] bugs.md exists if bugs were found — none found across all phases; no bugs.md correctly absent.
- [x] Each bug is a real code defect — N/A (no bugs).
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A.
- [x] No external_body used to mask a code defect — `ExFrameNumber` is foreign-type opacity, not a defect mask.
- [x] Bug entries include provenance — N/A.

## Spec Quality
Public-API specs are correct, complete, and readable. `inv()` is a single
load-bearing clause (representable frame number) — exactly the precondition that
makes `into_frame_number`'s internal `unwrap()` total; it correctly excludes
alignment (MMIO may be unaligned) and RAM-validity (`from_mmio_address` bypasses
it by design). `from_number` ensures the functional base-address relation
(`frame*FRAME_SIZE`), alignment, and `inv()`. `into_frame_number` is a total
projection (`result == self@ / FRAME_SIZE == self@ >> FRAME_SHIFT`).
`from_mmio_address` encodes the `unsafe` obligation as a `requires`
(frame-representability) and ensures identity wrap (`r@ == addr@`) + `inv()`. The
6 `assume_specification` library-edge boundaries are minimal and load-bearing
(frame-size positivity, `pow2(FRAME_SHIFT)==page_size`, newtype identity,
`FrameNumber` range/success). No tautological or genuinely subsumed ensures.

## Caller Coverage
- Covered: 11 / 11 (codex enumeration; claude 10/10 with the `>>FRAME_SHIFT`==`/FRAME_SIZE` expectations merged) — both agree **0 gaps**.
- Missing: none.

## Proof Completeness
- Remaining admit(): **0** — no blockers.
- Remaining external_body not in tcb-allowed.md: **0** — the module's only
  external_body (`ExFrameNumber`, `phys.spec.rs:38-40`,
  `external_type_specification`) is explicitly listed in `tcb-allowed.md`.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES**. In-scope module: only
  `ExFrameNumber` (allowlisted). Global (`make verify`): 25 external_body, every
  one cross-checked against `tcb-allowed.md` (frame.rs allocator core + Drop path,
  manager.rs facade, mod.rs LinkedList booking, kframe.rs, page.rs, frame.rs
  into_raw_value, identity_map.rs, bump_allocator alloc/alloc_as, ExLinkedList,
  ExFrameNumber). No new trust boundary introduced.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **1** (`ExFrameNumber`, allowlisted),
  assume_specification: **6** (arch/sys library edge, all allowlisted),
  cfg-gated exec: **0** (3 `cfg(verus_keep_ghost)` gates cover only `include!`/`use`).
- (Global cross-module counters: assume=0 external_body=25 admit=0 trusted=0
  no_decreases=0; all external_body allowlisted.)

## AST Consistency
- AST check: **PASS**. 14/16 functions MATCH; the 2 flagged (`from_number`,
  `into_frame_number`) are the pre-approved deviation
  `f(complex_expr) -> let x = complex_expr; f(x)` plus ghost `proof!` blocks.
  Both reviewers independently substituted the locals back and confirmed identical
  value, evaluation order, and side effects. `// VERUS DEVIATION` comments present
  at phys.rs:156 and :199. No genuine mismatch.

## Verification
- verus: **PASS**. `make verify-kernel MODULE=hal::mem::types::address::phys`
  exit 0, "No cheating detected in module"; `make verify` (cross-module) exit 0;
  `make build` exit 0 (kernel crate compiles within verify). spec_drift: no
  contract drift.

## Bug Summary
- Total bugs recorded: **0**.
- True Bugs: **0**. Both reviewers re-derived the two candidate defect sites:
  (1) `from_number` multiply `frame_raw * page_size` — overflow proven impossible
  (`fr <= usize::MAX/p - 1`, `p > 0` ⇒ `fr*p <= MAX - p <= usize::MAX`);
  (2) `into_frame_number` `.unwrap()` — proven total (`raw_addr >> shift ==
  self@ / page_size`, bounded by `inv()` ⇒ `from_raw_value` returns `Some`).
  Frame arithmetic, alignment, and MMIO identity all correct. Correctly no bugs.md.

## Issues (highest priority first)
1. (Minor / non-blocking) `from_number`'s `result@ % spec_page_size() == 0` is
   mathematically derivable from `result@ == frame*FRAME_SIZE`; retained
   deliberately as a caller-facing convenience (sole caller immediately calls
   `PageAligned::from_address`). Not a subsumption violation worth removing.
2. (Informational) The arch-edge `assume_specification`s encode
   `FrameNumber::MAX == usize::MAX/FRAME_SIZE - 1`. Governed, internally-consistent
   library-edge trust assumption (in `tcb-allowed.md`), used identically for
   `into`/`from_raw_value` so the round-trip stays sound. Discharged when `arch`
   is verified.

No blocking issues.

## Result: PASS
