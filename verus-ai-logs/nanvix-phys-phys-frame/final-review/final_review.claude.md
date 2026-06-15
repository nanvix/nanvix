# Final Verification Review — `mm::phys::frame` (independent, strict)

Reviewer: automated strict final review (from-scratch, no prior summary trusted).
Date: 2026-06-15. Project root: `/home/ruize/nanvix-phy`.
HEAD: `75ea3888a` (frame.rs last verified at `5fd42e370`).

Files reviewed:
- `src/kernel/src/mm/phys/frame.rs` (1074 lines)
- `src/kernel/src/mm/phys/frame.spec.rs` (11 lines)
- `src/kernel/src/mm/phys/frame.proof.rs` (150 lines)

Scope functions: `share`, `Inner::share`, `instance`, `refcount`, `Inner::refcount`,
`is_covered`, `Inner::is_covered`, `free_count`, `free`, `Inner::free`, `book`,
`Inner::book`, `alloc_range`, `Inner::alloc_range`, `alloc_contiguous`,
`Inner::alloc_contiguous`, `alloc`, `Inner::alloc`. `init` = SKIP/EXCLUDE.

---

## Checklist

**Caller Analysis**
- [x] Every `pub(super)` shim's caller success+failure expectation maps to a concrete
  requires/ensures. Verified 9/9 (`alloc`, `alloc_contiguous`, `free_count`, `free`,
  `is_covered`, `book`, `alloc_range`, `share`, `refcount`); see Caller Coverage.
- [x] `Drop`-path constraint on `free` (no precondition, `no_unwind`,
  `opens_invariants none`) honored — frame.rs:880–895.

**View Design**
- [x] View is caller-abstract: `FrameAllocView` = `Set<int>` allocated/free +
  `Map<int,int>` refcounts over physical addresses; bitmap/BSS array hidden
  (frame.spec.rs `View for Inner`, frame.proof.rs:22–51). Mathematical types only.
- [x] State-threading mechanism (`PhysAuth` tracked carrier) lets mutating shims name
  pre (`old(auth)@`) and post (`final(auth)@`) — resolves the constant-`phys_view()`
  limitation. frame.rs:732,761 etc.

**Specification**
- [x] All in-scope exec fns carry `#[verus_spec]` requires/ensures (8 `Inner::*` +
  10 shims; `init` excluded). fn_coverage: 11/11 names matched, 0 missing.
- [x] No tautological ensures among in-scope fns (only the excluded `init` has
  `Err(_) => true`, frame.rs:686). Err arms are all meaningful.
- [~] Minor: 5 shims restate facts derivable from their `spec_alloc_one/_set/_share`
  equality (caller-convenience redundancy, not a defect — spec-design principle 8).

**Proving**
- [x] `admit` = 0; `assume` = 0; `no_decreases` = 0 in frame files.
- [x] `make verify-kernel MODULE=mm::phys` → exit 0, **31 verified, 0 errors**.
- [x] `make verify` (cross-module) → exit 0, **0 errors**, no regressions.
- [x] Sole proof `lemma_free_count` (frame.proof.rs:92) is discharged without `admit`.

**Cheating Elimination**
- [x] frame module: admit=0, assume=0, assume_specification=0, cfg-gated exec=0.
- [x] `external_body` = 11 in frame.rs, **all** present in `tcb-allowed.md` (see TCB).
- [x] Spec drift vs HEAD: 0 (no weakening).

**Bug Recording**
- [x] `bugs.md` present, internally consistent, classifies findings correctly
  (0 code bugs; spec-architecture limitation resolved; `free` Drop exception governed).
- [x] No undiscovered code defect surfaced during this review.

---

## Spec Quality

External/top-level specs are correct, caller-oriented, and readable.

- **Mutating reservation shims** (`alloc`, `alloc_contiguous`, `book`, `alloc_range`,
  `share`) carry the **strong** post-state contract via the `tracked PhysAuth`
  carrier: each names both `old(auth)@` and `final(auth)@` and asserts the exact
  `FrameAllocView` transition (`spec_alloc_one`/`spec_alloc_set`/`spec_share`) with a
  meaningful `Err` arm `final(auth)@ == old(auth)@` (+ a negative witness, e.g.
  `!free_frames.contains` for `book`, frame.rs:951–954). No weak/pre-state-only specs.
- **Query shims** (`is_covered`, `refcount`, `free_count`) are pure and exact:
  `is_covered` is a biconditional over `covered()` (frame.rs:923); `refcount` returns
  `count == refcounts[frame@]` (1067); `free_count` returns
  `result == free_frames.len()` with `free_frames.finite()` (840–841).
- **`free`** deliberately carries only `ensures phys_view().inv()` (frame.rs:889)
  under `opens_invariants none`/`no_unwind`. This is the documented `Drop`-path
  exception (callers swallow errors, do not rely on the precise refcount delta — see
  caller_analysis.md:100–114). Justified, not a weakening of a reservation op.
- **Tautological ensures**: none in scope. Only excluded `init` has `Err(_) => true`.
- **Subsumed ensures**: the Ok arms of `alloc`/`book`/`share`/etc. list both the
  transition equality and its consequences (`allocated_frames.contains(...)`,
  `refcounts[...]==1`). These are derivable but written for direct caller use
  (spec-design principle 8). Acceptable; cosmetic at most.

Readability: thorough doc-comments and inline justification on every contract.

---

## Caller Coverage — Covered 9 / 9 (Missing: none)

| pub fn | Caller (Ok) expectation | Mapped ensures | Caller (Err) expectation | Mapped ensures | Status |
|---|---|---|---|---|---|
| `alloc` | aligned, in allocated, refcount 1 | `frame.inv()`, `spec_alloc_one`, `allocated.contains`, `refcounts==1` (757–753) | allocator unchanged | `final==old` (754) | Covered |
| `alloc_contiguous` | base+i·page reserved, single-ref, contiguous; `count>0` | `spec_alloc_set({base+i·page})`, `subset⊆allocated`, `requires count>0` (782–802) | unchanged | `final==old` (804) | Covered |
| `free_count` | #free frames, finite | `result==free_frames.len()`, `finite()` (840–841) | n/a (pure) | — | Covered |
| `free` | inv preserved, Drop-safe, no precond | `phys_view().inv()`, `opens_invariants none`, `no_unwind` (889–894) | errors returned not panicked | same | Covered |
| `is_covered` | true ⟺ tracked (allocated∪free) | `ret <==> covered().contains` (923) | — | — | Covered |
| `book` | frame reserved, refcount 1 | `spec_alloc_one`, `allocated.contains`, `refcounts==1` (947–949) | unchanged, not free | `final==old`, `!free.contains` (952–953) | Covered |
| `alloc_range` | every region frame reserved | `spec_alloc_set(region_frames)`, `subset⊆allocated` (986–989) | unchanged, region not fully free | `final==old`, `!subset⊆free` (992–994) | Covered |
| `share` | still allocated, +1 ref | `spec_share`, `allocated.contains`, `refcounts.contains_key` (1026–1028) | no ref, untouched; not allocated or ≥255 | `final==old` + disjunction (1031–1034) | Covered |
| `refcount` | allocated, `count==refcounts[frame]` | `allocated.contains`, `count==refcounts[frame@]` (1065–1067) | not allocated | `!allocated.contains` (1069) | Covered |

`init` excluded per scope. **Note (doc drift):** `caller_analysis.md:83–98,219` claims
the `alloc_contiguous` and `free_count` shims have *no* `#[verus_spec]`. The current
code specs **both** (frame.rs:774, 825) — the implementation is *stronger* than the
analysis records. Caller_analysis.md is stale here but the discrepancy favors coverage.

---

## Proof Completeness

- `admit()` count: **0** (grep over frame.rs/spec/proof = 0).
- `external_body`-not-in-TCB count: **0** — all 11 are TCB-approved (next section).
- Only proof obligation `lemma_free_count` (frame.proof.rs:92–148) is fully
  discharged (injective `i→i·page` image cardinality argument), no `admit`.

---

## TCB Compliance — YES

11 `external_body` in frame.rs, each mapped to a `tcb-allowed.md` entry:

| frame.rs line | function | tcb-allowed.md entry |
|---|---|---|
| 115 | `Inner::alloc` | §"The `Inner::*` methods" |
| 181 | `Inner::alloc_contiguous` | §"The `Inner::*` methods" |
| 254 | `Inner::free` | §"The `Inner::*` methods" |
| 339 | `Inner::share` | §"The `Inner::*` methods" |
| 411 | `Inner::refcount` | §"The `Inner::*` methods" |
| 459 | `Inner::book` | §"The `Inner::*` methods" |
| 505 | `Inner::is_covered` | §"The `Inner::*` methods" |
| 535 | `Inner::alloc_range` | §"The `Inner::*` methods" |
| 644 | `instance` | §"The singleton bridge: `instance()`" |
| 677 | `init` | §"Skip / exclude from current proof target" |
| 880 | `free` (shim) | §"`frame::free` (`Drop` path)" |

None outside the approved list. No invented justifications.

---

## Guardrails Compliance (frame module: frame.rs + frame.spec.rs + frame.proof.rs)

- admit: **0**
- assume: **0**
- external_body: **11** (all TCB-approved)
- assume_specification: **0**
- cfg-gated exec: **0** (only `#[cfg(verus_keep_ghost)] include!` at frame.rs:49,52 —
  the allowed include!/imports class; no cfg-gated branch/expr/match-arm exists)

Harness reports global `external_body=23`, `cfg_gate=9`, `CHEATING_DETECTED`. These are
**kernel-wide** counts. The 9 cfg-gates are all `#[cfg(verus_keep_ghost)] verus! {` /
spec-block or macro gating in `hal/mem`, `macros.rs`, `kframe.rs`, `manager.rs` — **none
in the frame target files** and none gate exec branches. The 23 external_body span the
whole subsystem; all 23 (incl. the 11 here) are TCB-governed. `CHEATING_DETECTED` is the
harness flagging the *presence* of governed `external_body`, not an ungoverned escape.

---

## AST Consistency — PASS (semantic equivalence; minor doc nit)

Baseline: last non-`[verus]` frame.rs at `a49054e16`. `ast_consistency.py count` →
6 MISMATCH / 13 MATCH. All 6 mismatches inspected via `diff`:

| fn | change | verdict |
|---|---|---|
| `alloc` | `instance().alloc()` → `let r=instance(); let res=r.alloc(); <proof!> res` | equivalent (pre-approved intermediate-binding; proof block stripped/ghost) |
| `alloc_contiguous` | same shape | equivalent |
| `alloc_range` | same shape | equivalent |
| `book` | same shape | equivalent |
| `share` | same shape | equivalent |
| `free_count` | `nbits - used` split into named `let nbits/used` | equivalent; **documented** with `// VERUS REWRITE` (frame.rs:845) |

All six are the pre-approved "`f(complex_expr)` → `let x = complex_expr; f(x)`
(intermediate value for assertions)" deviation: the `let r`/`let res` bindings are
required only so the ghost `proof! { auth.v.frames = (*r)@; }` block can name the
post-state and the result is returned afterward. Exec behavior is byte-for-byte
identical (`r` is the same `&'static mut Inner`, `res` the same `Result`). No semantic
mismatch → exec fidelity preserved.

**Minor nit (not a soundness blocker):** 5 of the 6 (`alloc`, `alloc_contiguous`,
`alloc_range`, `book`, `share`) lack the explanatory `// VERUS REWRITE` comment the
skill recommends for pre-approved deviations; only `free_count` has it. Recommend
adding a one-line comment to each. Does not affect verification integrity.

---

## Verification

- `make verify-kernel MODULE=mm::phys`: **PASS** — exit 0, 31 verified, **0 errors**
  (`verus_2026-06-15_10-19-39.log`).
- `make verify` (cross-module): **PASS** — exit 0, **0 errors**, no regressions
  (`verus_2026-06-15_10-22-02.log`).
- `spec_drift.py git-diff frame.rs --before HEAD`: exit 0, **0 contract drift**
  (no ensures removed, no requires added/strengthened). Note: working tree == HEAD,
  so this compares the committed verified state; no weakening present.
- `fn_coverage.py`: 11 source exec fns, 11 matched, 0 missing, 0 extra.

---

## Bug Summary — Total recorded: 0 code bugs; True bugs: 0

`bugs.md` states no code bugs (no overflow, off-by-one, missing bounds check, or
unchecked cast). Review confirms: `share` already uses `checked_add` (frame.rs:385);
`free`/`refcount`/`share` bounds-check `frame_number >= refcount.len()` before indexing;
`alloc_range` checks coverage before `set`; `init` guards `nframes > NFRAMES`. The two
documented items in `bugs.md` are a **spec-architecture limitation** (resolved via the
`PhysAuth` carrier) and the **`free` Drop-path `external_body` exception** — both are
verification/design matters correctly classified as *not* code defects, with full
provenance. Consistent with `bug-reporting` ("write None" when no bug). No
proving-time defect went unrecorded.

---

## Issues (highest priority first)

1. **[Minor / documentation] Undocumented pre-approved AST deviations.** 5 shims
   (`alloc`, `alloc_contiguous`, `alloc_range`, `book`, `share`) introduce
   intermediate `let r`/`let res` bindings (to thread the ghost `proof!` auth update)
   without a `// VERUS REWRITE` comment. Semantically equivalent and pre-approved, but
   the ast-consistency skill asks for a documenting comment (as `free_count` has).
   *Fix:* add a one-line comment to each. No soundness impact.
2. **[Minor / documentation] Stale `caller_analysis.md`.** It records the
   `alloc_contiguous` and `free_count` shims as unspecced; the code now specs both
   (stronger than documented). Update the analysis to match.
3. **[Informational] Harness `CHEATING_DETECTED` label.** Driven by kernel-wide
   `external_body=23` / `cfg_gate=9`, all governed by `tcb-allowed.md` and none in the
   frame target files as forbidden constructs. No action needed for this module.

None of the above are soundness/verification blockers.

---

## Result: PASS

Justification: every checklist item is satisfied. `admit=0`, `assume=0`,
`assume_specification=0`, cfg-gated-exec=0 in the frame module; all 11 `external_body`
are TCB-approved; both `make verify-kernel MODULE=mm::phys` and full `make verify` pass
with 0 errors; spec drift = 0; caller coverage 9/9; AST differences are all
semantically-equivalent pre-approved deviations; bugs.md is consistent. The only
findings are minor documentation nits (missing deviation comments on 5 shims; stale
caller_analysis.md), neither of which compromises verification integrity.

Key counts: verus errors=0 | admit=0 | assume=0 | external_body(frame)=11 (all TCB) |
cfg-gated-exec(frame)=0 | caller coverage=9/9 | spec drift=0 | AST=6 equivalent
rewrites | code bugs=0.
