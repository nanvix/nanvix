## Turn 1: Full Proving Checklist Walkthrough

### Progress
- Done (all PASS, evidence below):
  1. No specs weakened (spec drift) — PASS
  2. Zero remaining `admit()` — PASS
  3. Zero `external_body` outside `tcb-allowed.md` (HARD RULE) — PASS
  4. Zero `assume`/`assume_specification` except std/external trust boundaries — PASS
  5. No cfg-gated exec code — PASS
  6. Cheating audit (counts + locations) — PASS
  7. Claimed Verus limitations have isolated reproducers — PASS
  8. Exec rewrites minimal / semantically equivalent (`// VERUS REWRITE`) — PASS
  9. Cross-module regression (`make verify`) — PASS
  10. Verification + build, 0 errors / 0 warnings — PASS
- Current: none remaining
- Remaining: none

### Verification

Source files read in full: `lib.rs`, `lib.spec.rs`, `lib.proof.rs`.
Baselines: specification END SHA `2315c1794e2ee472b7b0434c6ecc0cc127de22da`;
base branch `exp`.

**Item 1 — Spec drift.** Ran the spec-drift tool against specification END for all
three files:
```
python3 scripts/spec_drift.py git-diff src/libs/bump_allocator/src/lib.rs       --before 2315c179
python3 scripts/spec_drift.py git-diff src/libs/bump_allocator/src/lib.spec.rs  --before 2315c179
python3 scripts/spec_drift.py git-diff src/libs/bump_allocator/src/lib.proof.rs --before 2315c179
```
All three: "✅ No contract drift detected", exit 0. Ensures removed 0, requires added 0,
functions removed 0. PASS.

**Item 2 — admit().** `grep -nE "admit\("` over the three files → 0 call sites (only a
descriptive comment at `lib.proof.rs:6`). The three lemmas (`lemma_geometry`,
`lemma_exhausted_boundary`, `lemma_alloc_transition`) carry fully discharged bodies.
`make verify-bump-allocator` cheating line: `admit=0`. PASS.
(Minor non-blocking note: the comment at `lib.proof.rs:6` still says bodies are
`admit()` placeholders — now stale since bodies are discharged. Documentation-only;
not a checklist failure.)

**Item 3 — external_body (HARD RULE).** Cheating detail:
```
lib.rs:286 alloc: external_body
lib.rs:373 alloc_as: external_body
```
Exactly 2, both explicitly listed in `verus-ai-logs/tcb-allowed.md`
(`FixedSizeBumpAllocator::alloc`, `FixedSizeBumpAllocator::alloc_as`). Legitimacy
confirmed via `verus-unsupported.md`: the success path does `usize as *mut [u8; N]`
and dereferences it, which Verus rejects (exact errors quoted: "Verus does not
support this cast: `usize` to `*mut [u8; N]`" and "dereferencing a raw pointer …
only supports raw pointers through the permissioned raw_ptr interface"). No
`PointsTo` source exists for the externally-owned `BssStorage` region. Mirrors the
approved `src/libs/raw-array` trust boundary. PASS.

**Item 4 — assume / assume_specification.** `assume(...)`: 0. One
`assume_specification [<usize>::div_ceil]` at `lib.spec.rs:28`. Verified that vstd
ships no div_ceil spec (`grep -rn div_ceil` over the vstd tree → 0 spec/ensures
hits). `usize::div_ceil` is a std-library function ⇒ legitimate external-bottom std
trust boundary, which the checklist explicitly allows. Spec is faithful
(`requires y != 0`; `ensures result == (x + y - 1) / y`). PASS.

**Item 5 — cfg-gated exec.** Cheating line: `cfg_gate=0`. Only cfg directives are
`#[cfg(verus_keep_ghost)]` on the spec/proof `include!`s (ghost-only) and the
crate-level `#![cfg_attr(not(any(test, feature = "std")), no_std)]`. No exec
branch/expression/match-arm gating. PASS.

**Item 6 — Cheating audit (exact counts + locations).**
- `admit`: 0
- `external_body`: 2 — `lib.rs:286` (`alloc`), `lib.rs:373` (`alloc_as`); both TCB-listed
- `assume`: 0; `assume_specification`: 1 — `lib.spec.rs:28` (`usize::div_ceil`, std boundary)
- cfg-gated exec: 0
- `trusted`: 0; `no_decreases`: 0
PASS — every pattern accounted for and justified.

**Item 7 — Isolated reproducers for claimed limitations.** `verus-unsupported.md`
records three limitations, each tied to the specific construct with the exact Verus
error message: (1) `usize`→`*mut` cast + raw-pointer deref; (2) `AtomicUsize` value
not spec-readable (motivating `uninterp bump_view`); (3) `usize::div_ceil` absent
from vstd. Each cites the failing construct and compiler error, not merely "the
function fails". PASS.

**Item 8 — Exec rewrites minimal/equivalent.** `git diff exp -- lib.rs`: zero exec
line deletions; all `+` lines are added `#[verus_spec]`/`#[verus_verify]` attributes
and ensures/requires. Exec logic, signatures, and comments unchanged ⇒ no rewrites
needed, hence no `// VERUS REWRITE` markers, and semantic equivalence is trivial.
PASS.

**Item 9 — Cross-module regression.** `make verify` (bitmap, sys, nanvix-slab,
bump-allocator, kernel): every crate Exit code 0, 0 verification errors. The
`CHEATING_DETECTED` flags in bitmap/nanvix-slab/kernel are pre-existing
(`cfg_gate`, kernel `external_body`), unchanged, and out of scope for this review.
PASS.

**Item 10 — Verification + build.** `make verify-bump-allocator`: exit 0, 6 verified,
0 errors; verus log contains 0 warnings. `cargo build` and `cargo clippy` on the
crate: 0 errors, 0 warnings. `cargo test`: 3 unit tests + 1 doc-test pass. PASS.

### Fix Request
None. All 10 checklist items PASS with tool-backed evidence. No fix required.

Non-blocking observation (no action gating RESOLVED): update the stale comment at
`lib.proof.rs:6` to reflect that the lemma bodies are now discharged (no longer
`admit()` placeholders).

### Verdict
RESOLVED — all checklist items PASS.
