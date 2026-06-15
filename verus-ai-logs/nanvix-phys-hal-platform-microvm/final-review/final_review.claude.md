# Independent Final Review — `hal::platform::microvm::gva_to_gpa`

**Reviewer:** Independent final review (skeptical, tool-verified)
**Date:** 2026-06-15
**Scope:** ONLY `gva_to_gpa` in
`src/kernel/src/hal/platform/microvm/mod.rs` (~L415–432). All other module
functions are out of scope.
**Module files:** `mod.rs`, `mod.spec.rs`, `mod.proof.rs`

---

## Function & spec under review

```rust
// mod.rs:425-432
#[verus_spec(result =>
    ensures
        result as int == spec_gva_to_gpa(gva as int),
)]
#[inline(always)]
pub fn gva_to_gpa(gva: usize) -> usize {
    gva
}
```

```rust
// mod.spec.rs
pub open spec fn spec_gva_to_gpa(gva: int) -> int {
    gva
}
```

```
// mod.proof.rs
verus! { } // verus!   (empty — no lemmas needed for identity)
```

The ensures reduces to `result == gva` (the MicroVM identity map).

---

## Task 1 — Spec quality — **PASS**

Applying `spec-design` criteria:

- **Bound to exec code (#1):** Yes. The `#[verus_spec]` ensures is attached
  directly to the real exec function (confirmed by diff in Task 9 — only the
  attribute + includes were added; the body `gva` is byte-identical). Not a
  copied shadow function. `spec_gva_to_gpa` is referenced by the exec ensures,
  so it is **not** a floating/orphan spec.
- **Sufficient to reject bugs (#3 / anti-pattern #8 tautological):** The ensures
  pins `result == gva`. A buggy impl (`gva + 1`, `0`, `gva & MASK`, a non-injective
  remap) would violate it. **Non-tautological.** Verified: replacing the body
  would fail verification.
- **Non-subsumed (anti-pattern #9):** Single clause; nothing to subsume. The
  design correctly folds determinism / injectivity / frame-stepping into
  corollaries of `result == gva` rather than emitting redundant clauses (matches
  view_design.md "Rejected Alternatives" #6).
- **Declarative, not operational (#4):** The indirection through
  `spec_gva_to_gpa` names the platform translation map (WHAT) rather than
  restating "return the argument" (HOW). A future non-identity platform redefines
  one named hook.
- **`open` correctness:** **Correct.** The sole caller must derive **frame
  correspondence** (the booked GPA backs the MMIO GVA), which on MicroVM *is*
  `result == gva`. `closed` would hide the identity and defeat the only reason
  the function is specced. The identity is a platform-level VMM contract, not a
  leaked implementation detail — exposing it is the contract.
- **Error path (anti-pattern #5):** N/A — total/infallible `usize -> usize`, no
  `Result`/`Option`. Correctly modeled with **no `requires`** (totality).

### Minor (non-blocking) observation: `int` vs `usize`

`spec-design` principle #7 explicitly says *"For pointer addresses, prefer
`usize` over `int` — addresses are inherently non-negative and bounded by the
address space."* The spec uses `int` (with `gva as int` / `result as int`
casts). For the **identity** map this is harmless: there is no arithmetic, hence
no overflow or negativity concern, and `result as int == gva as int` is
equivalent to `result == gva` over `usize`. view_design.md justifies `int` as
"abstract, overflow-free." This is a **stylistic deviation from the recommended
default**, not a correctness defect. **Not a blocker.** (If a future platform
implements a real offset/walk with arithmetic, `usize` would be the safer
choice.)

**Verdict: PASS** (one minor, non-blocking style note on `int`).

---

## Task 2 — Caller coverage — **PASS**

Sole caller confirmed: `book_mmio_regions`, `src/kernel/src/mm/phys/mod.rs:114`:

```rust
let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);
let phys_addr = PageAligned::from_address(unsafe {
    PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(mmio_addr))?
})?;
if frame::is_covered(phys_addr) { frame::book(phys_addr)?; }
start += mem::FRAME_SIZE;
```

Mapping caller expectations (from `caller_analysis.md`) to spec clauses:

| Caller expectation | Captured by | Status |
|---|---|---|
| Totality / infallibility (called in a loop, no error handling on the call) | No `requires`; direct `usize` return; no panic/trap path | ✅ captured |
| Determinism / purity (same input → same output) | `spec_gva_to_gpa` is a math function of `gva` alone | ✅ derivable |
| Frame correspondence (per-`FRAME_SIZE` stepping lands on matching frame) | `result == gva` ⇒ `gva+FRAME_SIZE` ⇒ `gpa+FRAME_SIZE` | ✅ derivable |
| Identity on MicroVM (`gpa == gva`) | `result == gva` directly | ✅ captured |
| Post-failure behavior | N/A — no failure path (the `?` belongs to `from_mmio_address`) | ✅ correctly absent |

Every success expectation has a spec clause or is an immediate corollary of
`result == gva`. There is no failure expectation to cover.

Note (informational, out of scope): `book_mmio_regions` is itself `external_body`
(LinkedList limitation, per `tcb-allowed.md`), so it does not *currently* consume
this ensures in a proof. That is an out-of-scope `mm/phys` concern; the
`gva_to_gpa` contract is correct and ready for when that caller is verified.

**Verdict: PASS.**

---

## Task 3 — Proof completeness (microvm files) — **PASS**

Scoped grep over `mod.rs`, `mod.spec.rs`, `mod.proof.rs`:

```
$ grep -nE "assume\s*\(|assume!|admit\s*\(|admit!" mod.rs mod.spec.rs mod.proof.rs
(no output, exit 1 = 0 matches)
```

- `admit()` in microvm files: **0** → no BLOCKER.
- `external_body` in microvm files: **0** → no BLOCKER.

Cross-check against the kernel-wide verifier report (Task 6): the cheating-detail
file lists **zero** microvm entries:

```
$ grep -i microvm verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt
NONE (no microvm entries in cheating detail)
```

All 12 kernel-wide admits live in `mm/phys/manager.proof.rs`,
`mm/virt/identity_map.proof.rs`, `mm/virt/identity_map.rs` (out of scope).

**Verdict: PASS — no admit(), no external_body in scope.**

---

## Task 4 — TCB compliance — **PASS (vacuous)**

`external_body` count in the microvm module files = **0**. There is nothing to
reconcile against `tcb-allowed.md`. No microvm function appears in
`tcb-allowed.md` (correct — none is needed). No un-listed `external_body` exists.

**Verdict: PASS.**

---

## Task 5 — AST consistency — **PASS**

The single-arg auto-detect picked a base commit (`3b7e25cc2 "add order plan"`)
that predates the file's existence, so it errored. Re-run with the correct
pre-verus base (`b52e0c915`, the last commit touching `mod.rs` before the verus
work):

```
$ python3 .../ast_consistency.py --base-ref b52e0c915 \
    src/kernel/src/hal/platform/microvm/mod.rs count
✅ Consistent: 28 functions, 3 structs match.
EXIT=0
```

- **0 MISMATCH.**
- **`// VERUS REWRITE` comments in the module: 0** (`grep -rc` → 0). Nothing to
  audit for semantic equivalence — no exec rewrites were performed.
- Diff confirms `gva_to_gpa`'s body (`gva`) is byte-identical pre/post verus.

**Verdict: PASS.**

---

## Task 6 — Verification (`make verify-kernel`) — **PASS**

```
$ cd /home/ruize/nanvix-phy-specs && make verify-kernel
...
=== Results ===
  cached (no recompilation)
  Exit code : 0
```

The module-targeted commit in history confirms the in-scope unit:

```
194b299b8 [verus] verify PASS: kernel::hal::platform::microvm (1 verified, 0 errors)
```

Kernel-wide verification exits **0 with 0 errors**. (The build reports
`status: CHEATING_DETECTED` purely from out-of-scope modules — see Task 7.)

**Verdict: PASS — exit 0, 0 errors.**

---

## Task 7 — Guardrails (microvm module files ONLY) — **PASS**

Exact counts (precise verification-escape patterns):

| Pattern | mod.rs | mod.spec.rs | mod.proof.rs | Blocker? |
|---|---|---|---|---|
| `admit(` / `admit!` | 0 | 0 | 0 | — |
| `assume(` / `assume!` | 0 | 0 | 0 | — |
| `external_body` | 0 | 0 | 0 | — |
| `assume_specification` | 0 | 0 | 0 | — |
| cfg-gated | 19 | 0 | 0 | see note |

**False-positive clarification on `assume`:** a loose `\bassume` prefix scan
returns 6 hits in `mod.rs`, but all 6 are the English word **"assumes"** in doc
comments (lines 256, 257, 276, 277, 303, 304) — NOT `assume()` verification
escapes:

```
$ grep -noE "\bassume[a-z_]*" mod.rs
256:assumes  257:assumes  276:assumes  277:assumes  303:assumes  304:assumes
$ grep -nE "assume\s*\(|assume!" mod.rs   →  0 matches (exit 1)
```

So **assume = 0**, **admit = 0** in scope → **no BLOCKER**.

**cfg-gate note:** the 19 `cfg(...)` in `mod.rs` are:
- 2 × `#[cfg(verus_keep_ghost)]` — the standard pattern that *includes*
  `mod.spec.rs`/`mod.proof.rs` (does not hide exec from Verus).
- 17 × ordinary Rust **feature gates** (`whp`, `pit`, `smp`, `stdio`,
  `exception-stack-guard`) — pre-existing build configuration, none of which
  excludes exec code from verification. None gates `gva_to_gpa` (L430 has no cfg).

These are benign and standard; they are not "cfg-gated-out-of-verus exec." Note
the kernel-wide `cfg_gate=19` reported by the build is an unrelated whole-kernel
tally.

**Verdict: PASS — admit=0, assume=0, external_body=0, assume_specification=0 in scope.**

---

## Task 8 — Bug reconciliation — **PASS**

`bugs.md` does **not** exist for this module:

```
$ cat .../nanvix-phys-hal-platform-microvm/bugs.md  →  NO bugs.md
```

This is **correct**. `gva_to_gpa` is the identity function (`gva`) on an
identity-mapped guest; there is no logic, arithmetic, branching, or state to
harbor a defect. The spec (`result == gva`) matches the code, and the code
matches the documented platform contract (VMM identity mapping). No real code
defect exists, and the review discovered none. The absence of `bugs.md` is the
expected, accurate state.

**Verdict: PASS — no defects, correctly no bugs.md.**

---

## Task 9 — Spec drift — **PASS**

```
$ python3 .../spec_drift.py git-diff \
    src/kernel/src/hal/platform/microvm/mod.rs --before HEAD
# Spec Drift Report
- Functions with changes: 0
- Contract drift (⚠ review required): 0
  - Ensures removed: 0
  - Requires added: 0
**✅ No contract drift detected.**
```

Manual diff against the pre-verus baseline (`b52e0c915`) confirms the only
changes are additive ghost annotations (the `#[verus_spec]` ensures, the
`use vstd::prelude::*;`, and the two `#[cfg(verus_keep_ghost)] include!`s). The
exec body is unchanged. **No ensures weakened, no requires strengthened, no
contract removed.**

**Verdict: PASS — no weakening.**

---

## Cheating counts (exact)

**In-scope (microvm module files):**

| Metric | Count |
|---|---|
| admit | **0** |
| assume | **0** (6 prose "assumes", not escapes) |
| external_body | **0** |
| assume_specification | **0** |
| no_decreases | 0 |
| cfg-gated exec hiding from verus | **0** (2 verus-include cfgs + 17 feature gates, all benign) |

**Out of scope (kernel-wide, other modules — informational only):**
`assume=0 external_body=19 admit=12 trusted=0 no_decreases=0 cfg_gate=19` — all
located in `mm/phys` and `mm/virt`; zero in microvm.

---

## Per-task scorecard

| # | Task | Verdict |
|---|---|---|
| 1 | Spec quality | ✅ PASS (minor `int`-vs-`usize` note) |
| 2 | Caller coverage | ✅ PASS |
| 3 | Proof completeness (admit/external_body in scope) | ✅ PASS |
| 4 | TCB compliance | ✅ PASS (vacuous) |
| 5 | AST consistency | ✅ PASS |
| 6 | Verification (`make verify-kernel`) | ✅ PASS (exit 0) |
| 7 | Guardrails (scoped) | ✅ PASS |
| 8 | Bug reconciliation | ✅ PASS |
| 9 | Spec drift | ✅ PASS |

---

## Overall verdict: **PASS**

The `gva_to_gpa` verification is complete and sound within scope:

- Correct, minimal, declarative external-top spec (`result == gva` via a named
  `open` platform-map spec function) that is non-tautological, non-subsumed,
  caller-usable, and bound to the real exec function.
- Zero verification escapes in scope: **admit=0, assume=0, external_body=0,
  assume_specification=0**.
- Exec code byte-identical to pre-verus (AST consistent, 0 rewrites, no drift).
- `make verify-kernel` exits 0 with 0 errors; module verified `1 verified,
  0 errors`.
- No code defects; absence of `bugs.md` is correct.

### Prioritized issues list

**Blockers:** none.

**Non-blocking / optional:**
1. (Minor, style) `spec_gva_to_gpa` uses `int` for an address where
   `spec-design` principle #7 recommends `usize`. Harmless for the identity map
   (no arithmetic ⇒ no overflow/negativity). Consider `usize` if a future
   MicroVM translation introduces arithmetic. No action required now.
