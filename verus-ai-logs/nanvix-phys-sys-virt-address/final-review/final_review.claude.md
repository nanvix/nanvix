# Final Review — `sys-virt-address` (`VirtualAddress`)

Independent, skeptical, review-only assessment of the Verus verification effort
for `src/libs/sys/src/sys/mm/address/virt.rs`.

In-scope target functions (ONLY these): `VirtualAddress::new`,
`VirtualAddress::from_raw_value` (inherent), `VirtualAddress::into_raw_value`
(the `Address` trait method), and the type `VirtualAddress` (struct + `View`).

**Reviewer:** independent final-review pass. No source/spec/proof code was
modified. All findings below are backed by command output captured live.

---

## Checklist

### Caller Analysis
- [x] `caller_analysis.md` exists and scopes the four in-scope items.
- [x] Each in-scope function's caller expectations (success + failure) documented.
- [x] Round-trip / totality / constructor-equivalence invariants enumerated.

### View Design
- [x] `view_design.md` exists and is derived from caller analysis + body-removed API.
- [x] `View for VirtualAddress` defined: `type V = int`, `closed spec fn view(&self) -> int = self.0 as int`.
- [x] View is minimal (scalar `int`) and matches the abstract resource (a single address integer).
- [~] `inv()` — design specifies `inv() == true`; **no `inv()` is actually defined in source**. Acceptable (trivially true, no in-scope spec consumes it), but note the design doc and source diverge.

### Specification
- [x] `VirtualAddress::new` carries `#[verus_spec] ensures result@ == value as int` (correct).
- [x] inherent `VirtualAddress::from_raw_value` carries `ensures result@ == raw_addr as int` (correct).
- [ ] **`VirtualAddress::into_raw_value` has NO `#[verus_spec]` ensures** and its `impl Address for VirtualAddress` block has **NO `#[verus_verify]`** → unverified, no contract. **DESK-REJECT for an in-scope function.**
- [x] Specs are declarative, caller-oriented, and total (no error paths needed — all infallible).
- [ ] All in-scope exec functions annotated/verified — **FAIL** (`into_raw_value` unannotated & unverified).

### Proving
- [x] `make verify-sys` → exit 0, `2 verified, 0 errors`.
- [x] `./z build` → `[OK] Build complete.`
- [x] No `admit()` anywhere in virt.rs / virt.spec.rs / virt.proof.rs.
- [x] No open proof obligations on the two verified functions.

### Cheating Elimination
- [x] `admit` = 0.
- [x] `assume` = 0.
- [x] `external_body` = 0 (virt scope).
- [x] `assume_specification` = 0.
- [x] cfg-gated **exec** code in virt.rs = 0 (the two `#[cfg(verus_keep_ghost)] include!` lines are the standard spec/proof-include mechanism, excluded by the counter).
- [x] TCB compliance — no `external_body` in scope, nothing to justify.

### Source Integrity (AST)
- [ ] `ast_consistency.py … count` → `1 mismatched, 1 extra (17 functions match)` → **raw FAIL**.
- [x] In-scope target exec (`new`, inherent/trait `from_raw_value`, `into_raw_value`, struct) is **byte-identical** to baseline (per-function `diff --name` = MATCH; git diff confirms only annotations added).
- [~] `clone_address` is genuine **EXTRA** exec (added during verification by a sibling `kernel::all` commit `40a4c4b60`; it is a now-required `Address` trait method). Out of in-scope set.
- [x] Spec drift: the reported "ensures removed" is a **false positive** (working tree == HEAD; name-collision between inherent & trait `from_raw_value`).

### Bug Recording
- [ ] `bugs.md` **does not exist** — per the bug-reporting skill it should at minimum contain `None`. Process gap.
- [x] No true code bugs found in the in-scope functions during this review.

**Honest verdict: multiple unchecked items → FAIL.**

---

## Spec Quality

| Function | `#[verus_verify]` | `#[verus_spec]` ensures | Assessment |
|----------|:-----------------:|-------------------------|------------|
| `VirtualAddress::new` | yes (inherent impl) | `result@ == value as int` | ✅ Correct, minimal, caller-usable. Total `const` constructor; identity wrap. |
| inherent `from_raw_value` | yes (inherent impl) | `result@ == raw_addr as int` | ✅ Correct; mirrors `new`, gives constructor equivalence `new(x)@ == from_raw_value(x)@`. |
| `Address::into_raw_value` | **NO** | **NONE** | ❌ **Coverage gap.** Not verified, no contract. |
| trait `Address::from_raw_value` | **NO** | **NONE** | (Out of strict scope; also unverified — same un-annotated impl block.) |
| `View for VirtualAddress` | n/a (verus! block) | `view = self.0 as int`, `closed` | ✅ Correct abstraction (`int`); shared with sibling address types. |

**Key defect — `into_raw_value`.** Both `caller_analysis.md` (Key Invariant #1:
"`VirtualAddress::new(x).into_raw_value() == x`") and `view_design.md` §4
("`ensures result as int == self@`") require `into_raw_value` to be specified as
the inverse projection. The source provides neither a contract nor verification
(`impl Address for VirtualAddress` at line 167 lacks `#[verus_verify]`;
`into_raw_value` at line 253 lacks `#[verus_spec]`). Consequence: the
**round-trip identity** that the abstraction is built around is only half
established — `new(x)@ == x` and `from_raw_value(x)@ == x` are proven, but no fact
relates `into_raw_value`'s result back to `self@`, so a caller **cannot** derive
`new(x).into_raw_value() == x`. Per spec-design "Exec Binding Rules — Desk Reject
Criteria", an in-scope exec function with no contract is a desk reject.

The `impl Address` block lacking `#[verus_verify]` means that entire trait impl
(including `into_raw_value` and the trait `from_raw_value`) is invisible to
Verus — not merely unspecified but unchecked.

---

## Caller Coverage

**Covered: 2 / 3 in-scope functions (type View covered separately).**

| In-scope item | Call sites (per caller_analysis) | Expectation | Spec present? |
|---------------|----------------------------------|-------------|:-------------:|
| `new` | 8 (config.rs layout consts, `NULL_USER_FN`, internal) | total const wrap, `view==value` | ✅ |
| inherent `from_raw_value` | 5 (mmio.rs:126, sync.rs:30/58, thread_create_args:44/47) | total wrap, round-trip | ✅ |
| `Address::into_raw_value` | 3 (mmio.rs:67, sync.rs:37, sync.rs:65) | `result == self@` (inverse) | ❌ |
| type `VirtualAddress` | field/const type (32 refs) | `Copy/Eq/Ord`, View as `int` | ✅ (View defined) |

**Missing:**
- `Address::into_raw_value` — the 3 call sites that round-trip a `VirtualAddress`
  back to a `usize` (`u32::try_from(base.into_raw_value())` in `mmio.rs:67`;
  `From<MutexAddress>/From<ConditionAddress> for usize` in `sync.rs:37,65`) rely
  on `into_raw_value()` returning exactly `self@`. No ensures exists, so this
  caller expectation is unmet.

---

## Proof Completeness

- **`admit()` list:** none (0 occurrences across virt.rs / virt.spec.rs / virt.proof.rs).
- **`external_body` not in TCB list:** none (0 `external_body` in scope).
- Verus result: `2 verified, 0 errors` (the two annotated inherent functions).
- `virt.spec.rs` and `virt.proof.rs` are empty (`verus! { }`) — consistent with
  the trivial spec needs, but they hold no `into_raw_value` material either.

---

## TCB Compliance

**YES** (vacuously). There are zero `external_body` / `assume_specification` /
`axiom` constructs in the in-scope files, so no new trust boundary was
introduced and nothing needs to appear in `tcb-allowed.md`.

---

## Guardrails Compliance

Exact counts in `virt.rs` + `virt.spec.rs` + `virt.proof.rs`:

| Dimension | Count | Locations |
|-----------|:-----:|-----------|
| `admit` | 0 | — |
| `assume` | 0 | — |
| `external_body` | 0 | — |
| `assume_specification` | 0 | — |
| cfg-gated exec | 0 | only `virt.rs:9,11` = `#[cfg(verus_keep_ghost)] include!(...)` (standard spec/proof include — excluded by the guardrail counter, not exec) |

`make verify-sys` reported a **global** crate-level `cfg_gate=1`; traced to
`src/libs/sys/src/sys/mm/alignment.rs:151` (`#[cfg(verus_keep_ghost)]` guarding a
`verus! { }` block — a different, out-of-scope module). virt.rs contributes 0.

`admit > 0` or `assume > 0` → **NONE**. Guardrails: **PASS** for the in-scope files.

---

## AST Consistency

**Raw tool: FAIL** — `ast_consistency.py … count` → `⚠️ 1 mismatched, 1 extra (17 functions match)`.

Breakdown after investigation:
- **`VirtualAddress::from_raw_value` MISMATCH** — *false positive*. Two functions
  share this name (inherent `-> Self` and trait `-> Result<Self,Error>`); the
  summary's name-keyed dict collides. Direct `ast_consistency.py diff --name
  "VirtualAddress::from_raw_value"` → **MATCH**. `spec_drift.py` likewise reports
  a spurious "ensures removed" from the same collision (working tree == HEAD, so
  there is no actual change).
- **`clone_address` EXTRA_IN_VERUS** — *genuine extra exec fn*, but **out of
  scope** and **not introduced by this effort**: `git log -S` shows it was added
  in commit `40a4c4b60` (`[verus] … kernel::all`) together with the new required
  trait method `Address::clone_address`. `fn_coverage.py` flags it
  `EXTRA … Callers 0 … REMOVE`.

**In-scope exec fidelity: PASS.** `git diff <merge-base> -- virt.rs` shows the
only changes to the four in-scope items are added Verus annotations
(`#[verus_verify]`, `#[verus_spec]`), the import reshuffle, and the spec/proof
`include!`s; the in-scope exec bodies (`Self(value)`, `VirtualAddress::new(raw_addr)`,
`self.0`, `struct VirtualAddress(usize)`) are unchanged.

---

## Verification

- **Verus (`make verify-sys`): PASS** — exit 0; `2 verified, 0 errors`; cached
  (no recompilation). `status: CHEATING_DETECTED` is solely the out-of-scope
  `cfg_gate=1` in `alignment.rs`; `cheating-detail.txt` is empty;
  `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0`.
- **Build (`./z build`): PASS** — `[OK] Build complete.` (`make build` is a no-op;
  the repo builds via `./z build`). Kernel + all crates (incl. `sys`) compile,
  confirming Verus annotations erase cleanly under normal `cargo build`.

---

## Bug Summary

- **Total recorded bugs: 0** — `bugs.md` does not exist (a `None` entry should be
  written per the bug-reporting skill).
- **True bugs found this review: none.** The in-scope exec code is a correct,
  total `usize` newtype wrapper. No overflow/off-by-one/cast hazards.
- The `into_raw_value` gap is a **specification incompleteness**, not a code bug.

---

## Issues (highest priority first)

1. **[BLOCKER — Spec coverage] `into_raw_value` is unspecified and unverified.**
   In-scope `Address::into_raw_value` has no `#[verus_spec]`, and its
   `impl Address for VirtualAddress` block (virt.rs:167) has no `#[verus_verify]`,
   so Verus never checks it. The caller-relied round-trip inverse
   (`result as int == self@`, mandated by caller_analysis & view_design) is
   absent; 3 call sites (mmio.rs:67, sync.rs:37/65) cannot discharge their
   round-trip assumption. **Fix:** add `#[verus_verify]` to the `impl Address`
   block and `#[verus_spec(result => ensures result as int == self@)]` to
   `into_raw_value` (and `ensures match … Ok(r) => r@ == raw_addr as int` to the
   trait `from_raw_value`), then re-verify.

2. **[Medium — Source integrity] `clone_address` is extra exec vs baseline.**
   AST/`fn_coverage` flag it `EXTRA … REMOVE`. It is a required trait method
   added by a sibling kernel-verification commit, so it cannot simply be removed
   without the trait change; but it is an uncontracted, unverified exec function
   now resident in this file. Confirm it is intended and (eventually) specified.

3. **[Low — Process] `bugs.md` missing.** Create it with `None` (no bugs found)
   to satisfy the bug-recording step.

4. **[Info — Tooling] AST/spec-drift false positives** from the duplicate
   `from_raw_value` name. Not a real exec/spec change; documented above so future
   reviewers do not chase it.

5. **[Info] `inv()` not defined** though `view_design.md` specifies `inv()==true`.
   Harmless (trivially true, unused) but a doc/source divergence.

---

## Result: FAIL

The verification of `VirtualAddress::new` and the inherent
`VirtualAddress::from_raw_value` is correct, clean (no admit/assume/external_body),
and both Verus and the build pass. However the review **fails** because an
explicitly in-scope function — **`VirtualAddress::into_raw_value`** — carries no
contract and is not even verified (its `impl Address` block lacks
`#[verus_verify]`). This is a desk-reject-level coverage gap that leaves the
abstraction's central round-trip/inverse property (which 3 caller sites depend
on) unprovable. Secondary failures: an extra uncontracted exec fn
(`clone_address`) versus baseline and a missing `bugs.md`. Promote to PASS only
after `into_raw_value` (and ideally the trait `from_raw_value`) are annotated and
re-verified.
