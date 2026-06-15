## Turn 1: Full Proving-Phase Checklist Review — phys-frame

### Progress
- Done (all PASS): spec-drift, admit, external_body, assume/assume_specification,
  cfg-gated exec code, cheating audit, Verus-limitation reproducers, exec rewrites,
  cross-module regression, verification + build.
- Current: final sign-off.
- Remaining: none.

### Verification

Target files:
- `src/kernel/src/mm/phys/frame.rs`
- `src/kernel/src/mm/phys/frame.spec.rs`
- `src/kernel/src/mm/phys/frame.proof.rs`

Commands run by reviewer (not trusting any verbal claim):
- `make verify-kernel` (fresh, cache invalidated by `touch frame.proof.rs`)
- `python3 spec_drift.py git-diff src/kernel/src/mm/phys/frame.rs --before 2a529ed` (specification END)
- `git diff` of exec/spec/proof files (specification END → HEAD)
- `./z build`
- direct `grep`/AST scans of frame files

---

#### 1. No specs weakened (spec-drift) — **PASS**
`spec_drift.py git-diff ... --before 2a529edcd87067b1d16e2f622c63e0a26f1242ed`
(specification END boundary) → exit **0**, "✅ No contract drift detected"
(0 ensures removed, 0 requires added, 0 functions removed).
`git diff 2a529ed..HEAD` over `frame.rs`, `frame.spec.rs`, `frame.proof.rs` is
empty — no spec text changed since specification END.

#### 2. Zero remaining admit() — **PASS**
Global cheating scan: `admit=0`. `grep 'admit('` across the three frame files: none.

#### 3. Zero external_body unless TCB-allowed (HARD RULE) — **PASS**
`frame.rs` contains exactly **11** `external_body` functions. Each verified
individually against `verus-ai-logs/tcb-allowed.md`:

| Line | Function | TCB-allowed entry |
|------|----------|-------------------|
| 137 | `Inner::alloc` | "Inner::* methods" |
| 210 | `Inner::alloc_contiguous` | "Inner::* methods" |
| 290 | `Inner::free` | "Inner::* methods" |
| 368 | `Inner::share` | "Inner::* methods" |
| 428 | `Inner::refcount` | "Inner::* methods" |
| 481 | `Inner::book` | "Inner::* methods" |
| 517 | `Inner::is_covered` | "Inner::* methods" |
| 565 | `Inner::alloc_range` | "Inner::* methods" |
| 652 | `instance()` | "singleton bridge: instance()" |
| 689 | `init()` | "Skip / exclude from current proof target" |
| 888 | `frame::free` (Drop path) | "frame::free (Drop path)" |

The `frame::*` free-function shims (`alloc`, `alloc_contiguous`, `book`,
`alloc_range`, `share`, `is_covered`, `refcount`, `free_count`) are **not**
`external_body` — confirmed absent from the cheating-detail list, i.e. they are
body-verified as the TCB doc states. All 11 boundaries are governed; none is a
new/unlisted boundary.

#### 4. Zero assume / assume_specification — **PASS**
Global `assume=0`. `grep 'assume('` / `'assume_specification'` across the three
frame files: none. (The one `assume_specification` in the codebase lives in
`kframe.spec.rs`, outside this proof target and separately TCB-approved.)

#### 5. No cfg-gated exec code — **PASS**
Only two `#[cfg(verus_keep_ghost)]` in `frame.rs` (lines 49, 52), both guarding
`include!("frame.spec.rs")` / `include!("frame.proof.rs")` — ghost includes,
explicitly excluded by `count_cfg_gates` in `scripts/verify.sh`. No cfg-gated
branch, expression, match arm, closure, or body-duplication. Reviewer re-ran the
exact counter logic against `frame.rs` → both lines classified `SKIP`. frame.rs
contributes **0** to the global `cfg_gate=9` (those 9 are other modules' ghost
includes/derives, outside this target).

#### 6. Cheating audit (exact counts + locations) — **PASS**
For the proof target `frame.rs`:
- `admit`: 0
- `external_body`: 11 (all TCB-listed; locations in table above)
- `assume` / `assume_specification`: 0
- cfg-gated exec code: 0
- `trusted`: 0, `no_decreases`: 0
- `// VERUS REWRITE`: 0

Whole-kernel scan (context): `assume=0 external_body=23 admit=0 trusted=0
no_decreases=0 cfg_gate=9` — `CHEATING_DETECTED` status is driven solely by the
pre-approved TCB `external_body` set and ghost-include cfg gates, none new in
this phase.

#### 7. Claimed Verus limitations have isolated reproducers — **PASS**
Proving introduced **no new** Verus-limitation claims: every `external_body` is a
pre-approved TCB boundary (fixed in advance), and there is no `// VERUS REWRITE`
and no `verus-unsupported.md` entry for phys-frame. Proof-difficulty reproducers
are present and isolated under `reproducers/` (`01_shim_fails.rs`,
`02_goal_is_false.rs`, `03_strengthening_derives_false.rs`). No exec construct was
declared unsupported in this module.

#### 8. Exec rewrites minimal & semantically equivalent — **PASS (N/A)**
No `// VERUS REWRITE` comments in `frame.rs`; `git diff 2a529ed..HEAD` shows no
exec-body changes. Nothing to challenge.

#### 9. Cross-module regression — **PASS**
`make verify-kernel` (all modules): `mm::phys`, `mm::phys::frame`,
`mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool` all verified.
**32 verified, 0 errors, exit 0.**

#### 10. Verification + build, 0 errors / 0 warnings — **PASS**
- Verus (fresh, non-cached): **32 verified, 0 errors, exit 0**; `grep -c warning`
  on the run log = **0**.
- `./z build`: `[OK] Build complete`, exit 0, no compiler errors/warnings. (The
  lone "Sysroot directory ... not found; skipping symlink update" is a benign
  build-script notice, not a code warning.)

### Fix Request
None — every checklist item is PASS with reproduced tool evidence. No code change
requested. Proving phase for `phys-frame` is clean.
