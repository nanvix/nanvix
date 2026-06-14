# Final Comprehensive Review — `hal::mem::types::address::phys` (`PhysicalAddress`)

Reviewer: Claude (independent, adversarial). Date: 2026-06-15.
Scope (verification-order targets ONLY): the type `PhysicalAddress` (View + `inv()`),
`PhysicalAddress::from_number`, `PhysicalAddress::into_frame_number`,
`PhysicalAddress::from_mmio_address`. Hard rule checked: unlisted functions not modified by this effort.

Method: static analysis only. I did NOT run `make verify-kernel`/`make build` (authoritatively run,
exit 0). I independently grepped the three phys files, ran `spec_drift.py` and `ast_consistency.py`,
and inspected the arch/sys dependency contracts the proofs rely on.

---

## 1. Per-Check Findings

### Check 1 — Spec Quality (external-top contracts) — PASS

**View** (`phys.rs:303-311`):
```rust
impl View for PhysicalAddress { type V = int; closed spec fn view(&self) -> int { self.0@ } }
```
`int`, `closed`. Consistent with sibling address types (`VirtualAddress`, `PageAligned`,
`FrameAddress` all `type V = int`). Caller-abstract (hides the inner `VirtualAddress`). PASS.

**`inv()`** (`phys.spec.rs:43-45`):
```rust
pub open spec fn inv(&self) -> bool { spec_frame_number(self@) <= spec_max_frame_number() }
```
i.e. `self@ / page_size <= FrameNumber::spec_max()`. `open` so callers can use it. This is exactly the
well-formedness needed for `into_frame_number` totality. Minimal (no alignment/RAM-range invariant —
correctly rejected because `from_mmio_address` produces unaligned, out-of-RAM addresses). PASS.

**`from_mmio_address`** (`phys.rs:112-122`):
- `requires spec_frame_number(addr@) <= spec_max_frame_number()` — load-bearing: it is what lets the
  body prove `result.inv()`. Justified per spec-design "Static vs Dynamic Enforcement": the function
  performs NO runtime range check (it deliberately bypasses `is_valid_physical_address` for MMIO), and
  validity is the caller's responsibility (the fn is `unsafe`). Formalizing the unsafe precondition as
  `requires` is correct, not masking — a caller that cannot prove it cannot call the fn (obligation
  pushed to the call site, sound). Real MMIO (LAPIC `0xFEE0_0000` ⇒ frame `0xFEE00` ≪ `~2^52`) satisfies
  it, so the precondition is satisfiable, not vacuous. PASS.
- `ensures result is Ok` — HONEST: body is unconditionally `Ok(Self(addr))`. This is the *strongest*
  correct statement and is strictly better than a one-sided `Err(_) => true` (it proves `Err`
  unreachable). No tautology. PASS.
- `(result->Ok_0)@ == addr@` — identity wrapping; matches caller expectation #12. PASS.
- `(result->Ok_0).inv()` — result is well-formed; downstream `into_frame_number` needs it. PASS.
- No `Err(_) => true` tautology anywhere. The success/never-fails arms are complete. PASS.

**`from_number`** (`phys.rs:138-141`):
- `ensures result@ == spec_from_number(spec_frame_raw_value(frame))` = `result@ == frame@ * page_size`
  (base address of the frame). Total (no `Result`). Matches caller expectation #5.
- Alignment (`result@ % page_size == 0`), `result.inv()`, and round-trip are all DERIVABLE from this
  single exact-arithmetic ensures (`frame@ <= spec_max` from `FrameNumber`'s contract ⇒ frame index of
  result `= frame@ ≤ max`). Correctly omitted per the Subsumed-Properties anti-pattern. PASS.

**`into_frame_number`** (`phys.rs:159-164`):
- `requires self.inv()` — underwrites the internal `FrameNumber::from_raw_value(..).unwrap()`. Only this
  method requires `inv()`, so it is not a "scattered invariant"; it is the projection's well-formedness
  precondition. PASS.
- `ensures spec_frame_raw_value(result) == spec_frame_number(self@)` = `result@ == self@ / page_size`.
  Exact integer division ⇒ same-frame/distinct-frame injectivity (caller expectation #10) and inverse of
  `from_number` follow as consequences. Matches caller expectations #8/#9. PASS.

**Bug-rejection test**: a buggy `into_frame_number` returning `self@ / (2*page_size)`, a buggy
`from_number` returning `frame@ * (page_size+1)`, or a `from_mmio_address` returning a different address
all violate their ensures. The specs are sufficient to reject these. PASS.

**Dependency facts verified independently:**
- `spec_page_size()` is a *concrete* `open spec` = `arch::mem::PAGE_SIZE as int` (`frame.rs:42-44`), NOT
  `uninterp`. PASS.
- `FrameNumber::into_raw_value` ensures `result as int == self@` **and** `0 <= self@ <= spec_max()`
  (`number.rs:79-83`) — this is the bound the `from_number` `let raw_value = …` binding brings into
  scope to discharge `lemma_from_number_no_overflow`. The VERUS DEVIATION rationale is therefore
  factually correct.
- `FrameNumber::from_raw_value` returns `Some` with `@==value` for `value ≤ spec_max` (`number.rs:56-68`),
  so the `unwrap` in `into_frame_number` is justified by `lemma_frame_index`'s
  `frame_number ≤ spec_max_frame_number`. PASS.

### Check 2 — Caller Coverage — PASS (see §3)

### Check 3 — Proof Completeness — PASS
Grep over the three phys files (`phys.rs`, `phys.spec.rs`, `phys.proof.rs`):
- `admit` occurrences: **0** (BLOCKER threshold is >0). PASS.
- `external_body` occurrences: **0 real** (one match is inside a prose comment at `phys.spec.rs:69`). PASS.
No `admit()` anywhere ⇒ no blocker.

### Check 4 — TCB Compliance — PASS
Exactly **1** `assume_specification` (`phys.spec.rs:74`):
`<::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value` ⇒ `ensures result as int == addr@`.
It is listed in `tcb-allowed.md` (lines 170-207, "retained due to a genuine Verus limitation"). Its body
(`self.0`) trivially satisfies the contract (`VirtualAddress@ == self.0 as int`), so the axiom is sound.
No new trust boundary. PASS.

### Check 5 — AST Consistency — PASS (with one out-of-scope observation)
`ast_consistency.py` vs the true pre-verus original `88ad0ffcc:phys.rs`:
- MISMATCH `from_number` — the documented `// VERUS DEVIATION` (intermediate-value `let raw_value =
  frame.into_raw_value();`). This is the pre-approved deviation `f(complex_expr)` → `let x =
  complex_expr; f(x)`. `mem::FRAME_SIZE` is a const; `frame.into_raw_value()` is evaluated exactly once
  in both versions, then multiplied. **Evaluation order and effects are identical. Semantically
  equivalent.** PASS.
- MISMATCH `into_frame_number` — `let shift = mem::FRAME_SHIFT; raw_addr >> shift` vs
  `raw_addr >> mem::FRAME_SHIFT`. `FRAME_SHIFT` is a const; binding then shifting is identical. Same
  pre-approved intermediate-value category; has an explanatory comment at `phys.rs:167`. **Semantically
  equivalent.** PASS. (Note: against the nearer base `40a4c4b60` this already MATCHED — the binding
  predates this effort.)
- EXTRA_IN_VERUS `clone_address` (`phys.rs:278-280`) — see Issue #1 below. Verified via
  `git log -S clone_address`: introduced by the *separate* prior commit `40a4c4b60` (`kernel::all`
  verification, adding a kernel-wide `Address` trait method), **not** by the phys verification under
  review. Trivial delegation `PhysicalAddress(self.0)`. Out of this effort's scope but not attributable
  to it. Informational, not a blocker.

All other 14 functions MATCH. No in-scope or out-of-scope function *body* was altered by the phys effort;
the only changes are the two pre-approved intermediate-value bindings and added annotations.

`spec_drift.py git-diff --before HEAD`: "✅ No contract drift detected" (0 ensures removed, 0 requires
added). No contract weakening. PASS.

### Check 6 — Verification — PASS
Relying on the stated authoritative `make verify-kernel` exit 0 (module verified, 6 verified / 0 errors).

### Check 7 — Guardrails — PASS (exact counts in §4)

### Check 8 — Bug Reconciliation — PASS (see §5)

---

## 2. Spec Quality Assessment

The contracts are caller-driven, declarative, and minimal:
- The View (`int`) is the right abstraction and matches the rest of the address tower.
- `inv()` captures the single universal property (frame-representability) and nothing more — alignment
  and RAM-range invariants were correctly rejected because `from_mmio_address` violates them.
- Each in-scope function has a non-trivial, bug-rejecting ensures stated over abstract `int` arithmetic,
  independent of the shift-vs-divide implementation detail.
- No tautological ensures; no one-sided error spec (`from_mmio_address` proves `result is Ok`, making the
  `Err` arm provably unreachable rather than hand-waved with `Err(_) => true`).
- Derivable properties (alignment, round-trip, injectivity) are correctly omitted as subsumed.
- The one trust assumption (`VirtualAddress::into_raw_value`) is the smallest honest boundary; the
  alternative (verifying it) would *expand* the TCB by forcing `external_body` onto two unsupported
  int-to-ptr casts.

This meets the spec-design skill's quality bar. The specs could have been written from the signatures +
module purpose alone (the independence test), confirming they abstract over rather than mirror the code.

---

## 3. Caller Coverage — Covered 15/15

| # | Caller expectation (caller_analysis.md) | Spec clause | Status |
|---|------------------------------------------|-------------|--------|
| 1 | Type = opaque Copy, totally-ordered, single `int` view | `View::view = self.0@ : int` + `#[derive(Clone,Copy,Ord,Eq,…)]` | Covered |
| 2 | Wrappable in `PageAligned`/`TruncatedMemoryRegion` via `Address` | View `int` supports it (trait methods out of scope) | Covered |
| 3 | Valid value always has well-defined frame number | `inv()` + `into_frame_number requires inv()` | Covered |
| 4 | `from_number` total (no `Result`) | return type `Self` | Covered |
| 5 | `from_number` result = `frame * FRAME_SIZE` (base addr) | `result@ == spec_from_number(spec_frame_raw_value(frame))` | Covered |
| 6 | `from_number` result FRAME_SIZE-aligned | derivable from #5 (exact mul) | Covered (consequence) |
| 7 | Round trip `from_number(n).into_frame_number()==n` | derivable from #5 + #9 | Covered (consequence) |
| 8 | `into_frame_number` total / no panic for valid value | `requires self.inv()` underwrites `unwrap` | Covered |
| 9 | `into_frame_number` result = `self@ >> FRAME_SHIFT == self@/FRAME_SIZE` | `spec_frame_raw_value(result)==spec_frame_number(self@)` | Covered |
| 10 | `into_frame_number` same-frame/distinct-frame injectivity | derivable from #9 (exact division) | Covered (consequence) |
| 11 | `into_frame_number` inverse of `from_number` for aligned | derivable from #5 + #9 | Covered (consequence) |
| 12 | `from_mmio` identity `Ok ⇒ result@==addr@` | `(result->Ok_0)@ == addr@` | Covered |
| 13 | `from_mmio` succeeds where range-checked ctors reject | `result is Ok` (no range check; only frame-repr `requires`) | Covered |
| 14 | `from_mmio` unsafe: caller responsible for validity | `unsafe fn` + `requires` formalizing it | Covered |
| 15 | `from_mmio` `Err` arm never occurs | `ensures result is Ok` (proves `Err` unreachable) | Covered |

**Missing properties: NONE.** Items #6/#7/#10/#11 are intentionally and correctly left as derivable
consequences (Subsumed-Properties anti-pattern) rather than redundant ensures.

---

## 4. Guardrails — Exact Counts (module = the three phys files)

| Construct | Count | Threshold | Result |
|-----------|-------|-----------|--------|
| `admit()` | 0 | >0 = BLOCKER | PASS |
| `assume(...)` | 0 | >0 = BLOCKER | PASS |
| `external_body` | 0 (1 comment-only match) | not in tcb-allowed = BLOCKER | PASS |
| `assume_specification` | 1 (`phys.spec.rs:74`) | must be in tcb-allowed | PASS (listed) |
| cfg-gated exec | 0 (only `#[cfg(verus_keep_ghost)]` on `include!` of spec/proof — the standard, non-exec pattern) | exec cfg-gate = cheating | PASS |
| `#[verifier::trusted]` | 0 (comment-only matches) | banned | PASS |
| `exec_allows_no_decreases` | 0 | banned | PASS |
| `uninterp` | 0 | banned | PASS |

Module-introduced trust constructs total: exactly **1** `assume_specification`, in `tcb-allowed.md`. The
crate-wide cheating counters (assume=0, external_body=11, admit=27, cfg_gate=14) are inherited from other
already-verified kernel modules; the phys module adds 0 admit / 0 external_body / 0 exec cfg-gate.
Independently confirmed by grep. PASS.

---

## 5. Bug Reconciliation

**B1 — `make verify-sys` regression (FIXED): CONFIRMED genuine + still fixed.**
`src/libs/sys/src/sys/mm/address/virt.rs:176` `impl Address for VirtualAddress` is **un-annotated**
(verified: only an explanatory NOTE at lines 167-175 precedes it; the `#[verus_verify]` at lines 35/46
are on the struct and a different inherent impl). The block contains `as_ptr`/`as_mut_ptr`
(`virt.rs:270/274`) whose `usize as *const u8` / `usize as *mut u8` casts Verus cannot translate, and the
whole-impl-must-verify rule means annotating it breaks `make verify-sys`. This is a *real, fixed* defect
(an over-eager annotation that made a verification target un-buildable), correctly auto-fixed by reverting
the attribute. Classification per bug-reporting skill is correct.

**Retained `assume_specification` is a genuine Verus limitation, not a masked code defect.** The two
underlying errors (whole-impl rule; `usize`→pointer cast) are front-end limitations, with isolated
reproducers cited (`specification/whole_impl_rule.rs`, `specification/ptr_cast.rs`). The trusted contract
(`result as int == addr@`) is trivially satisfied by the body `self.0`. Verifying it would *expand* the
TCB (two int-to-ptr casts via `external_body`) to remove one trivial assumption — so the assumption is the
smaller, more honest boundary. Correctly classified as a verus limitation.

**Target functions:** bugs.md states no code bugs in the in-scope functions; I found none. Every surviving
gap is classified: the single trust point is a verus limitation (tracked in tcb-allowed.md), not a missing
spec or a true bug. No undiscovered/unrecorded bugs surfaced during this review.

---

## 6. Issues (highest priority first)

1. **[INFORMATIONAL — not a blocker]** `PhysicalAddress::clone_address` (`phys.rs:278-280`) exists in the
   current tree but not in the pre-verus original `88ad0ffcc`, and is outside the in-scope function set
   (no `#[verus_spec]`). `git log -S clone_address` attributes it to the *separate* prior commit
   `40a4c4b60` (`kernel::all`), which added a kernel-wide `Address::clone_address` trait method — i.e. it
   is a pre-existing trait-conformance delegation (`PhysicalAddress(self.0)`), **not** a modification
   introduced by the phys verification under review. It does not weaken any contract and is semantically
   trivial. No action required for this module; flagged only for completeness of the scope audit.

No other issues. **No BLOCKERS.**

---

## 7. Final Verdict

**PASS** — All in-scope contracts are correct, complete, caller-driven, and bug-rejecting (15/15 caller
expectations covered); exec changes are limited to two pre-approved, semantically-equivalent
intermediate-value bindings; the module adds 0 admit / 0 assume / 0 external_body and exactly one
`assume_specification` that is in `tcb-allowed.md` and justified by a reproduced Verus limitation; no
contract drift; `make verify-kernel` exit 0.
