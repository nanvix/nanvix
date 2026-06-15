# Final Comprehensive Review (claude-opus-4.8): hal-frame-address

Independent strict verification of the in-scope conversion API of
`FrameAddress` in `src/kernel/src/hal/mem/types/address/frame.rs`. Every claim
below was re-checked against the actual source/spec/proof/log artifacts; the
expensive shared builds (`make verify`, `make verify-kernel`, `./z build`) were
reused as ground truth per instructions, but all counts were re-confirmed with
`grep`/file reads. The four verus-skill docs named in the brief
(spec-design, verus-constraints, ast-consistency, bug-reporting) do not exist
under `.github/skills/` in this checkout; I applied the standard Verus
verification methodology those skills encode.

In-scope functions: `into_raw_value`, `into_frame_number`, `from_raw_value`,
`FrameAddress` (type/View/inv), `from_frame_number`. Out-of-scope and
intentionally unspecified: `new`, `into_physical_address`, `into_page_address`,
`fmt`, `eq`.

## Checklist

### Caller Analysis
- [x] All callers enumerated (intra-crate via rust-analyzer LSP; no cross-crate dependents) — `caller_analysis.md` lists per-fn counts for all 9 exec fns + type.
- [x] Per in-scope function caller expectations documented (`from_frame_number` 9, `into_frame_number` 7, `from_raw_value` 3, `into_raw_value` 19, type 93 refs).
- [x] Round-trip / inverse expectations captured (from_raw↔into_raw, from_frame↔into_frame, into_frame·PAGE_SIZE==self@) — "Key Invariants" §.
- [x] Abstract resource + caller invariants identified (page-aligned physical frame; `inv()` = `self@ % PAGE_SIZE == 0`).

### View Design
- [x] View type chosen and substitution-tested: `type V = int` (physical address); evaluated against every caller, all reduce to the address.
- [x] `inv()` defined and caller-relevant (page-alignment); `pub open` so callers can establish/consume it.
- [x] Minimal/non-redundant view — two-field and frame-index views explicitly rejected (frame number is derived `self@/PAGE_SIZE`).
- [x] Rejected alternatives documented (wrapper struct, two-field, index-primary, `usize` view, `internal_inv`, spec transitions).
- [x] No extra `pub spec fn` on `impl FrameAddress` beyond `view`/`inv` — confirmed in source (only `inv`, plus module-level `spec_page_size`).

### Specification
- [x] All 4 in-scope specced functions carry contracts (`from_raw_value` via `external_body` `#[verus_spec]`; the other three via plain `#[verus_spec]`).
- [x] Contracts are external-top / API-level, stated on `view`/`inv` only — no representation (`PageAligned`, `Error` payload) leaks.
- [x] Contracts faithful to verified dependency contracts (`PhysicalAddress::from_number`/`into_frame_number`, `PageAligned::from_address`, `PageAligned`/`PhysicalAddress` `View = int`).
- [x] Complete vs caller expectations (see Caller Coverage below).
- [x] Non-tautological; no harmful subsumption (the one mild redundancy in `into_frame_number` is a deliberate caller convenience — see Spec Quality).
- [x] Error paths adjudicated (`from_raw_value` `Err(_) => true` acceptable — see Spec Quality).
- [x] Out-of-scope functions intentionally unspecified (`fn_coverage.py`: 5 unspecified are all out-of-scope; not evaluated here).

### Proving
- [x] Module verifies: `make verify-kernel MODULE=hal::mem::types::address::frame` exit 0 (PASS, cached ground truth).
- [x] Proof obligations are elementary and sound (`lemma_frame_base_aligned` via `lemma_mod_multiples_basic`; `lemma_aligned_div_mul` via `lemma_fundamental_div_mod`) over the transparent constant `PAGE_SIZE == 4096`.
- [x] No `admit()` in the module (frame.rs / frame.spec.rs / frame.proof.rs) — grep confirms only comment occurrences, zero calls.
- [x] No `no_decreases` / decreases issues (global `no_decreases=0`; no recursive spec fns in module).

### Cheating Elimination
- [x] `external_body` count in module = 1 (`from_raw_value`, frame.rs:94); the only other text matches are comments.
- [x] That one `external_body` IS listed in `tcb-allowed.md` (line 137, `FrameAddress::from_raw_value`).
- [x] No `assume(...)`, no `admit(...)`, no `assume_specification` in the frame module (the two grep hits are descriptive comments of former state, line 40 / spec.rs:11).
- [x] cfg-gated count is a spec-include/spec-block false positive — `#[cfg(verus_keep_ghost)]` at lines 9/11 gate `include!` of spec/proof, line 36 gates the ghost `verus!` block (`spec_page_size`, `View`, `inv`). No exec fn/branch/match-arm is gated. Identical sanctioned pattern to siblings `phys.rs` and `aligned/page.rs`.
- [x] No `// VERUS REWRITE` markers in `frame*.rs` → no exec rewrites → AST consistency PASS trivially.

### Bug Recording
- [x] BUG-001 (duplicate `use ::vstd::prelude::*;`) recorded in `bugs.md` and confirmed fixed — current source has exactly one `use vstd::prelude::*;` (line 8).
- [x] No additional bugs found during this review (source builds clean per ground truth; contracts sound).

## Spec Quality

Strong and faithful. The four in-scope contracts are API-level (over `view`/`inv`
only) and each is grounded in an already-verified dependency contract:

- `into_raw_value` — `ensures result as int == self@`. Exactly the raw-address
  identity 19 call sites rely on. Chains through `PageAligned::into_raw_value`
  and `View(PageAligned) = self.0@ = View(FrameAddress)`. Meaningful, minimal.
  Note: it is *listed* in `tcb-allowed.md` (line 139) but in current source is a
  plain in-body-verified `#[verus_spec]` (no `external_body`). Listed-but-unused
  is acceptable and is strictly stronger than the documented intent (verified
  rather than trusted). Not a blocker.
- `from_raw_value` — `ensures Ok(fa) => fa.inv() && fa@ == raw_addr as int;
  Err(_) => true`. The `Ok` branch carries the full strengthened contract
  (`fa@ == raw_addr`) the boot-mapping callers need, beyond the bare
  `Ok => inv()` noted in `caller_analysis`. `Err(_) => true` is **acceptable**:
  all three callers propagate `Err` with `?` and rely only on the `Ok` path /
  no side effects; no caller inspects the error payload. The function is the
  one TCB `external_body` (PhysicalAddress `Address::from_raw_value` is
  intra-crate and not yet verified), correctly listed.
- `from_frame_number` — `ensures result is Ok; (Ok).inv();
  (Ok)@ == spec_from_number(spec_frame_raw_value(frame_number))`
  (= `frame@ * PAGE_SIZE`). Total constructor, page-aligned by construction,
  base address exact. Matches `caller_analysis` expectation `fa@ == n*PAGE_SIZE`
  and `Ok => inv()`. Proof discharges `PageAligned::from_address` alignment via
  `lemma_frame_base_aligned`. Complete and non-tautological.
- `into_frame_number` — `requires self.inv(),
  spec_frame_number(self@) <= spec_max_frame_number(); ensures
  spec_frame_raw_value(result) == spec_frame_number(self@),
  spec_from_number(spec_frame_raw_value(result)) == self@`. The
  representability precondition is a faithful propagation of
  `PhysicalAddress::into_frame_number`'s `inv()` (not derivable from
  page-alignment), correctly surfaced rather than hidden. The second ensures is
  mildly redundant with the first under `inv()` (since
  `(self@/PAGE_SIZE)*PAGE_SIZE == self@` when aligned), but it is a deliberate
  convenience: it hands callers the `from_frame_number`-inverse directly without
  forcing a div/mod lemma at every site. Justified, not harmful subsumption.

No contract is code-as-spec, no contract is biased to a single caller, and the
View is minimal (single `int`, no redundant frame-index field).

## Caller Coverage  (Covered: 5/5 in-scope caller expectations; Missing: none)

| Caller expectation (caller_analysis / view_design) | Backing contract | Status |
|---|---|---|
| `into_raw_value` yields physical address (`result as int == self@`) | `into_raw_value` ensures | Covered |
| `from_raw_value` Ok ⇒ `fa.inv()` and `fa@ == raw_addr` | `from_raw_value` ensures (both clauses) | Covered |
| `from_frame_number` Ok ⇒ `fa.inv()` and `fa@ == n*PAGE_SIZE`, always Ok | `from_frame_number` ensures (3 clauses) | Covered |
| `into_frame_number` = frame index, `result*PAGE_SIZE == self@` | `into_frame_number` ensures (both clauses) | Covered |
| Round-trip `from_raw_value(x).into_raw_value()==x` | derivable: `fa@==x` ∧ `result==self@` | Covered (composable) |
| Round-trip `from_frame_number(n).into_frame_number()==n` | derivable: `fa@==n*PS` ∧ `result==self@/PS` | Covered (composable) |
| `FrameAddress` always page-aligned (`inv()`) | `inv()` + Ok-ensures on both constructors | Covered |

Equality ⇔ same frame (CoW logic) relies on `PartialEq::eq`, which is
**out of scope** (intentionally unspecified) — correctly excluded.

## Proof Completeness  (Remaining admit(): 0; Remaining external_body not in tcb-allowed.md: 0)

Module-level: `admit()` = 0, `assume()` = 0, `assume_specification` = 0. The one
`external_body` (`from_raw_value`) is in `tcb-allowed.md`. Global ground-truth
counts (`make verify`: `external_body=20`, `admit=12`) are **pre-existing in
out-of-scope modules** and outside this review's target; the frame module itself
is admit-free and assume-free.

## TCB Compliance  (All external_body listed in tcb-allowed.md: YES)

Exactly one `external_body` in scope (`from_raw_value`, frame.rs:94) →
`tcb-allowed.md` line 137. No unlisted `external_body`. `into_raw_value` is also
listed (line 139) but is not currently `external_body` in source — a benign
listed-but-unused entry, strictly safe.

## Guardrails Compliance

- admit: 0
- assume: 0
- external_body: 1 (listed in tcb-allowed.md)
- assume_specification: 0
- cfg-gated exec: 0 real. The script's cfg count for the module is a
  **spec-include / ghost-`verus!`-block false positive** (lines 9, 11, 36 gate
  only `include!` of spec/proof and the ghost spec block). This is the sanctioned
  repo-wide pattern, matching `phys.rs` and `aligned/page.rs`.

## AST Consistency  (PASS)

No `// VERUS REWRITE` annotations exist in `frame*.rs`; there are no exec-body
rewrites to reconcile against an original. The verified bodies are the natural
delegations (`self.0.into_raw_value()`, `PageAligned::from_address(...)`, etc.).
Trivially PASS.

## Verification  (PASS — reused ground truth)

- `make verify-kernel MODULE=hal::mem::types::address::frame`: exit 0 (PASS).
- `make verify` (all crates + kernel): exit 0 (PASS).
- `./z build -- all-kernel`: clean, 0 errors/warnings.
- `spec_drift.py` (--before HEAD): 0 contract drift.
- `fn_coverage.py`: 9 source exec = 9 verus exec, 0 missing; 4/9 contracted
  (the in-scope conversions), 5 unspecified all out-of-scope.

## Bug Summary  (Total recorded: 1; True Bugs with severity)

- BUG-001 — duplicate `use ::vstd::prelude::*;` breaking the `-D warnings`
  normal build. Severity: Low (build hygiene, pre-existing at 38885545d).
  Status: **fixed and verified** — source now has a single `vstd` glob import
  (line 8); `./z build -- all-kernel` clean. No unrecorded bugs surfaced.

## Issues (highest priority first)

1. (Informational, non-blocking) `into_raw_value` is listed in `tcb-allowed.md`
   as `external_body` but is in-body-verified in current source. Listed-but-unused
   is safe; optionally prune the stale TCB entry for accuracy. Not a blocker.
2. (Cosmetic, non-blocking) `into_frame_number`'s second ensures is derivable
   from the first under `inv()`; retained intentionally as a caller convenience.
   No action required.

## Result: PASS

All checklist items satisfied. Module guardrails: admit=0, assume=0,
assume_specification=0, external_body=1 (TCB-listed), no real cfg-gated exec, no
AST rewrites, all in-scope caller expectations covered, verification green,
BUG-001 fixed. No blockers.
