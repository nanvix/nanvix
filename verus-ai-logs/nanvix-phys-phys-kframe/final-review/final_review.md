# Final Comprehensive Review: phys-kframe

> Consolidated from two independent sub-agent reviews (model-diverse):
> `final_review.claude.md` (claude-opus-4.8) and `final_review.codex.md` (gpt-5.3-codex),
> plus reviewer-led re-derivation of all facts. Targets: `KernelFrame::new`,
> `KernelFrame::drop`, `KernelFrame::base`. Module verified via
> `make verify-kernel MODULE=mm::phys` (exit 0). The two sub-agents disagreed
> only on the `new` error-arm; adjudication is documented under **Issues**.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` (rust-analyzer LSP), see `find_callers_output.md`
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md` §Caller Expectations
- [x] Abstract resource identified — owned RAII handle; `@ == base@` (base physical address)
- [x] Pre-existing specs assessed — none upstream; `kframe.spec.rs`/`.proof.rs` were empty stubs; `View for KernelFrame` pre-existed and is caller-abstract

### View Design
- [x] Every field passes the substitution test — single abstract value `base@: int` survives any storage rewrite
- [x] All caller-observable state represented — address is the only caller-observable state
- [x] No implementation-specific fields — `View::V = int` (just the base address)
- [x] inv() encodes real constraints — handle invariants live in the do-not-modify `FrameAllocView`/`Inner::inv`; the `int` view is intentionally minimal and adequate
- [x] Mathematical types used — `int` for the view (address identity exception: `base()` returns `FrameAddress`, a usize-backed newtype)

### Specification
- [x] Every in-scope exec function has requires/ensures — `new`, `base`, `drop` all carry `#[verus_spec]`
- [x] Caller coverage: every caller expectation in `caller_analysis.md` has a corresponding ensures (see Caller Coverage below)
- [x] View consistency: specs reference `@`/`phys_view()` and maintain `inv()`
- [x] No tautological ensures — `new`'s `Err(_) => true` is a **justified** tautology (type-system no-consume + zero allocator-state mutation; strongest expressible fact). See Issues #1.
- [x] No subsumed ensures
- [x] Error paths have meaningful ensures (match style) — `new` uses `Ok => .. , Err => ..`; `drop` states `phys_view().inv()`
- [x] No assume_specification for workspace-internal code — the one `assume_specification` targets the external `sys::mm::Address` trait (TCB-allowed)
- [x] vstd searched before assume_specification — the boundary is the external `Address` trait, not a vstd type
- [x] Specs written for the caller — `result@ == base@`, `result@ == self@` are directly usable in `manager.rs` contiguity/membership proofs
- [x] Trait obligations satisfied — `View`, `Drop` (`no_unwind`/`opens_invariants none`) match the `frame::free` shim contract
- [x] Spec completeness (advisory) — `new`'s nondeterministic error is intentional and matches caller expectations
- [x] Loop invariants — no loops in any in-scope function
- [x] No cheating on module's own functions — admit:0 assume:0; `clear` external_body is TCB-allowed (out of scope)
- [x] No specs weakened — `spec_drift.py` (vs HEAD and vs base `verus-ai/phys-upool`): 0 contract drift, 0 ensures removed
- [x] Bug awareness — no in-scope code defect found; `bugs.md` correctly absent
- [x] Cross-module regression — `make verify-kernel MODULE=mm::phys` covers the whole `mm::phys` module, exit 0 (shared run; concurrent `make verify` deliberately avoided to prevent target-dir corruption)
- [x] Verification — `make verify-kernel` exit 0, 0 errors; `make build` exit 0

### Proving
- [x] No specs weakened — `spec_drift.py`: 0 contract drift
- [x] Zero remaining admit()
- [x] Zero external_body unless in `tcb-allowed.md` — only `clear` (listed)
- [x] Zero assume/assume_specification except external-bottom — 1 `assume_specification` on the external `sys::mm::Address` trait (TCB-allowed)
- [x] No cfg-gated exec code — `#[cfg(verus_keep_ghost)]` only gates imports/includes/ghost `View` block
- [x] Cheating audit — admit:0, external_body:1 (TCB), assume:0, cfg-gated exec:0
- [x] Claimed Verus limitation has isolated reproducer — N/A; no `// VERUS REWRITE` and no claimed limitation in the 3 in-scope functions
- [x] Exec rewrites minimal/equivalent — none exist (0 `// VERUS REWRITE`)
- [x] Cross-module regression — module-level verify exit 0
- [x] Verification — 0 errors, 0 warnings

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions (`trusted=0`)
- [x] Zero exec_allows_no_decreases_clause (`no_decreases=0`)
- [x] Zero cfg-gated exec code (only imports/includes/ghost block)
- [x] Zero external_body unless in `tcb-allowed.md` — only `clear` (listed)
- [x] AST consistency: zero mismatches — no `// VERUS REWRITE` markers in any kframe file
- [x] All exec rewrites have VERUS REWRITE comment + reproducer — N/A (no rewrites)
- [x] Each surviving external_body confirmed in `tcb-allowed.md` — `clear` confirmed
- [x] No specs weakened — `spec_drift.py`: clean
- [x] Cross-module regression — module verify exit 0
- [x] Verification — 0 errors, 0 warnings

### Bug Recording
- [x] bugs.md exists if bugs were found — no bugs found, so no file (correct)
- [x] Each bug is a real code defect — N/A (none)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A
- [x] No external_body used to mask a code defect — `clear` masks a raw-memory op Verus cannot model, not a defect
- [x] Bug entries include provenance — N/A

## Spec Quality
- **`base`** — textbook trivial-accessor contract: `ensures result@ == self@`. Correct, readable, caller-usable. ✔
- **`new`** — success path `Ok(frame) => frame@ == base@` is exactly what `manager.rs` needs to carry `allocated_frames`/contiguity facts onto the handle. The error arm `Err(_) => true` is a *justified* tautology (see Issues #1): `new` performs **no** frame-allocator mutation on any path, and the only caller-relevant error guarantee ("input `base` not consumed/freed") is a type-system guarantee (`FrameAddress: Copy`), which spec-design directs to skip. ✔
- **`drop`** — `ensures phys_view().inv()`, `no_unwind`, `opens_invariants none`, mirroring the `frame::free` shim verbatim. The exact refcount transition is not expressible because `phys_view()` is a zero-arg `uninterp spec fn` with no `old(phys_view())` form; the invariant-preservation fact is the strongest expressible and is precisely what callers (RAII bulk-free via `Vec::clear`) rely on. ✔

## Caller Coverage
- Covered: **8 / 8** caller expectations (all from `caller_analysis.md`)
  1. `new` Ok address identity `frame@ == base@` — ✔ ensures
  2. `new` Err no-consume of `base` — ✔ via `FrameAddress: Copy` (type-system guarantee; spec-design "skip"), and `new` constructs `Self` only on the Ok path
  3. `new` identity-mapped backing — ✔ effected by `identity_map_page`; caller_analysis §`new` explicitly states **no caller depends on it abstractly**, and the helper's contract is `ensures true`, so there is no abstract fact to surface
  4. `base` returns owned address `result@ == self@` — ✔ ensures
  5. `base` purity / no mutation — ✔ `&self` accessor semantics
  6. `drop` preserves subsystem invariant `phys_view().inv()` — ✔ ensures
  7. `drop` never unwinds — ✔ `no_unwind`
  8. `drop` opens no invariant — ✔ `opens_invariants none`
- Missing: **none**. (codex flagged #2/#3/#4 as "missing"; each is explicitly resolved by `caller_analysis.md` as a type-system guarantee, a not-caller-observable side effect, or a by-design inexpressible transition — not a coverage gap.)

## Proof Completeness
- Remaining admit(): **0** — none. (No BLOCKERS.)
- Remaining external_body NOT in `tcb-allowed.md`: **0** — none. (No BLOCKERS.)
  - Total external_body in the 3 kframe files: 1 → `kframe.rs:141 KernelFrame::clear` (listed in `tcb-allowed.md`).

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES**
  - `kframe.rs:141 clear` → listed (raw `*mut u8` materialization + identity-map `memset`, unmodelable by Verus).
  - `assume_specification <PageAligned<T> as Address>::from_raw_value` (`kframe.spec.rs:33`) → listed (external `sys::mm::Address` trait edge).
- No new trust boundary introduced; the pre-approved TCB was not expanded.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **1** (TCB-allowed `clear`), assume_specification: **1** (TCB-allowed external `Address::from_raw_value`), cfg-gated exec: **0**
  - (`#[cfg(verus_keep_ghost)]` occurrences gate only `use` imports, `include!`s, and the ghost `View` block — not exec code.)

## AST Consistency
- AST check: **PASS** — zero `// VERUS REWRITE` markers in `kframe.rs`/`kframe.spec.rs`/`kframe.proof.rs`; the three in-scope functions verify as written with no exec rewrites, so no semantic-equivalence obligations exist.

## Verification
- verus: **PASS** — `make verify-kernel MODULE=mm::phys` exit 0, 0 errors (log: `final-review/verify-kernel.log`). `make build` exit 0. `spec_drift.py` (vs HEAD and base): 0 contract drift.

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` correctly absent)
- True Bugs: **0** — no in-scope logic error, safety violation, or incorrect behavior found. Reviewer findings were spec-shape observations, not code defects.

## Issues (highest priority first)
1. **(Advisory — resolved, not a blocker) `KernelFrame::new` error arm is `Err(_) => true`.**
   Sub-agent gpt-5.3-codex flagged this as an acceptance blocker (one-sided/tautological
   error spec). Adjudication: it is a **justified** tautology. (a) The only caller-relevant
   error guarantee — input `base` is not consumed/freed on `Err` — is a *type-system*
   guarantee (`FrameAddress` is `Copy`; `new` builds `Self` only on the Ok path), which
   spec-design explicitly says to skip. (b) `new` performs **no** frame-allocator mutation
   on any path (`identity_map_page` has `ensures true`; no `alloc`/`free`), so there is no
   frame-condition/state-preservation fact to assert beyond `true`. (c) `caller_analysis.md`
   §`new` recommends exactly `result@ == base@` on Ok + no-consume on Err, which the
   implementation meets. A stronger `phys_view().inv()` arm would require an artificial
   `requires phys_view().inv()` precondition even though `new` never touches the allocator.
   Conclusion: the spec is correct and not weakenable into something more meaningful; the
   item passes. No code change required.
2. **(Informational) `drop` states only `phys_view().inv()`, not the exact frame release.**
   By-design inexpressible: `phys_view()` is a zero-arg `uninterp spec fn` with no
   `old(phys_view())`. Mirrors the trusted `frame::free` contract; `caller_analysis.md`
   confirms callers do not rely on the precise refcount transition. Accepted.
3. **(Informational) Concurrent `make verify` skipped.** To avoid cargo target-dir
   corruption from two parallel reviewers, the shared `make verify-kernel MODULE=mm::phys`
   (exit 0) and `make build` (exit 0) were run once by the orchestrator; both sub-agents
   consumed the captured logs. Module-level verification covers all `mm::phys` files.

## Result: PASS
