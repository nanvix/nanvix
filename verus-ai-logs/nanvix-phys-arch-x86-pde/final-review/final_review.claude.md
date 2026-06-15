# Final Verification Review — `arch::x86::mem::paging::pde`

- **Reviewer:** Independent strict final verification (Claude)
- **Date:** 2026-06-15
- **Repo:** `/home/ruize/nanvix-phy-specs` — branch `verus-ai-prove`
- **Files reviewed:**
  - `src/libs/arch/src/x86/mem/paging/pde.rs`
  - `src/libs/arch/src/x86/mem/paging/pde.spec.rs`
  - `src/libs/arch/src/x86/mem/paging/pde.proof.rs`
- **In-scope functions (5):** `PageDirectoryEntryFlags::new`, `PageDirectoryEntryFlags::is_present`,
  `PageDirectoryEntry::new`, `PageDirectoryEntry::is_present`, `PageDirectoryEntry::frame_address`

---

## Spec Quality

All five in-scope functions carry external-top API contracts via `#[verus_spec]`. Assessed against
`spec-design`:

| Function | Contract | Verdict |
|---|---|---|
| `PageDirectoryEntryFlags::new` | `ensures result@ == spec_pde_flags_new(<8 args>)` | **Good.** Records all 8 flag arguments faithfully; declarative; uses View; not one-sided (a `new` dropping any argument would fail). |
| `PageDirectoryEntryFlags::is_present` | `ensures result == self@.present` | **Good.** Pure projection onto the View `present` bit. |
| `PageDirectoryEntry::new` | `ensures result@ == spec_pde_new(flags@, frame@), result.inv()` | **Good.** Pairs exact flags+frame; `inv()` is *meaningful* (`0 <= frame <= FrameNumber::spec_max()`), not tautological — it is what underwrites `frame_address` totality. |
| `PageDirectoryEntry::is_present` | `ensures result == self@.flags.present` | **Good.** Encodes the presence-delegation identity. |
| `PageDirectoryEntry::frame_address` | `ensures result as int == self@.frame * FRAME_SIZE; result as int % FRAME_SIZE == 0` | **Good.** Product form + alignment. |

Observations:
- **Mathematical types:** flags abstracted as `bool` (eight projection helpers `spec_*_set`), frame as
  `int`. No exec enums or `PteWord` leak into spec world. ✔
- **View usage:** both `view()` impls are `closed`, hiding the bit-packing encoding (realizes caller
  invariant 6, encoding independence). Specs are written in terms of `self@`. ✔
- **`inv()` meaningfulness:** `PageDirectoryEntry::inv` = `0 <= frame <= spec_max()` — substantive and
  load-bearing. `PageDirectoryEntryFlags::inv` = `true` — vacuous, but *correctly* so (hardware imposes
  no cross-field flag constraint; all 2⁸ combinations are legal). Justified in `view_design.md`. ✔
- **Alignment clause in `frame_address`:** `result % FRAME_SIZE == 0` is mathematically derivable from
  the product clause, i.e. technically redundant. It is **not** a bad tautology (`x == x`); it is a
  caller-facing convenience that `verify_kernel_mappings`/`ensure_pt` consume directly (a clause "the
  caller writes directly into their proof", per spec-design §8). Acceptable — not flagged as a defect.
- **No floating/orphan specs:** every spec fn (`spec_pde_flags_new`, `spec_pde_new`, the eight
  `spec_*_set` helpers) and the proof lemma `lemma_frame_address` connect to an exec `ensures`. ✔

**Spec Quality verdict: PASS.**

---

## Caller Coverage (Covered 6/6 + per-function)

Source: `caller_analysis.md` (numbered invariants 1–6 + per-function expectations).

| # | Caller invariant | Backing contract | Covered |
|---|---|---|---|
| 1 | Constructor fidelity (flags): `f.is_present() == (present == Present)` | `Flags::new` ensures `result@ == spec_pde_flags_new(..)` (⇒ `result@.present == (present is Present)`) ∘ `Flags::is_present` ensures `== self@.present` | ✔ |
| 2 | Constructor fidelity (entry): `e.is_present() == flags.is_present()` **and** `e.frame_address() == frame << FRAME_SHIFT` | `PDE::new` ensures `result@ == spec_pde_new(flags@, frame@)` ∘ `is_present`/`frame_address` ensures (FRAME_SIZE = 1<<FRAME_SHIFT) | ✔ |
| 3 | Presence delegation: `PDE::is_present ⟺ Flags::is_present` | `PDE::is_present` ensures `== self@.flags.present`; view gives `self@.flags == self.flags@`; `Flags::is_present` ensures `== self@.present` | ✔ |
| 4 | Frame alignment: `frame_address()` page-aligned | `frame_address` ensures `result % FRAME_SIZE == 0` | ✔ |
| 5 | Purity/totality: queries `&self` read-only, constructors total, none panic/mutate | All four queries/constructors have **no `requires`** (total); `&self`/by-value; `frame_address` proven overflow-free via `inv()`+`lemma_frame_address`; verification passes with no failed asserts (no panic) | ✔ |
| 6 | Encoding independence: bit layout not observable | both `view()` are `closed` | ✔ |

Per-function expectations (`Flags::new`, `PDE::new`, `PDE::is_present`, `Flags::is_present`,
`frame_address`) each map onto the contracts above. Downstream `assume_specification` placeholders in
`identity_map.spec.rs` are superseded without weakening (frame address = inverse of `new`'s frame via
`frame@ * FRAME_SIZE`).

**Covered: 6/6 invariants + 5/5 per-function expectations. Missing: none.**

---

## Proof Completeness

Grep over the three pde files:

| Construct | pde.rs | pde.spec.rs | pde.proof.rs | Total |
|---|---|---|---|---|
| `admit()` | 0 | 0 | 0 | **0** |
| `external_body` | 0 | 0 | 0 | **0** |

`lemma_frame_address` (pde.proof.rs) is fully discharged with vstd lemmas
(`lemma_usize_shl_is_mul`, `lemma2_to64`, `lemma_fundamental_div_mod`, `lemma_mod_bound`,
`lemma_mod_multiples_basic`, `lemma_mul_inequality`, one `nonlinear_arith` block) — no placeholder.

**No `admit()`. No `external_body` in pde. PASS.**

---

## TCB Compliance

`make verify-arch` cheating-detail (`verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`) reports
exactly four trusted items across the whole `arch` crate:

| Item | Location | In `tcb-allowed.md`? | In pde? |
|---|---|---|---|
| `invlpg` (external_body) | `x86/mem/paging/mod.rs:80` | ✔ yes | no |
| `lemma_entry_roundtrip` (assume) | `x86/mem/paging/table.proof.rs:16` | ✔ yes | no |
| `Table::read` (external_body) | `x86/mem/paging/table.rs:209` | ✔ yes | no |
| `Table::write` (external_body) | `x86/mem/paging/table.rs:246` | ✔ yes | no |

`grep -rn "external_body" src/libs/arch/src` confirms the only `external_body` annotations are
`mod.rs:79`, `table.rs:202`, `table.rs:241` (all TCB-listed). **None reside in any pde file.** Every
verifier-reported trust boundary in the arch crate is accounted for in `tcb-allowed.md`.

**TCB Compliance: PASS.**

---

## Guardrails Compliance (exact counts — three pde files)

| Dimension | Count | Locations |
|---|---|---|
| `admit()` | **0** | — |
| `assume(...)` | **0** | — |
| `external_body` | **0** | — |
| `assume_specification` | **0** | — |
| `uninterp` spec fn | **0** | — |
| cfg-gated **exec** code | **0** | — |

Note: `pde.rs:9` and `pde.rs:11` carry `#[cfg(verus_keep_ghost)]` — but these gate the two
`include!("pde.spec.rs")` / `include!("pde.proof.rs")` lines only. This is the standard mechanism for
including spec/proof bodies under Verus and erasing them in normal `cargo build`; it is **not** exec
gating (no exec branch, expression, match arm, or function body is gated). Consistent with the
`verus-constraints` guardrails.

**Guardrails: PASS (admit=0, assume=0).**

---

## AST Consistency (PASS)

`ast_consistency.py … summary`: `matched=22 mismatched=1 missing=0 extra=0` — exactly **one MISMATCH**:
`PageDirectoryEntry::frame_address`.

Diff (exec-only):
```
-        self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
+        let raw: usize = self.frame.into_raw_value();
+        raw << crate::mem::FRAME_SHIFT
```

**Judgment — pre-approved, semantically-equivalent deviation:**
- Matches the `ast-consistency` pre-approved deviation row
  `f(complex_expr)` → `let x = complex_expr; f(x)` ("Intermediate value for assertions").
- Rationale is sound: the `FrameNumber` bound (`self@ <= spec_max()`) is exposed *only* through the
  postcondition of the exec call `into_raw_value()`; the bridging lemma `lemma_frame_address(raw)` must
  run **after** the call but **before** the overflow-bearing shift. An exec call cannot appear inside
  `proof!`, so the operand must be named to give the lemma a reference point. Same value, same
  operations, same time/space complexity — semantically identical.
- A `// VERUS REWRITE` documenting comment is present at the fix site (pde.rs:420–428) and references a
  reproducer: `verus-ai-logs/nanvix-phys-arch-x86-pde/cheating-elimination/repro/frame_address.rs`
  (confirmed present, 4436 bytes; `bad` fails / `good` passes, demonstrating the necessity).

Minor nit (non-blocking): the comment prefix is `// VERUS REWRITE`; the codified vocabulary is
`VERUS DEVIATION` (for documented limitations) / `VERUS BUG FIX`. Because this change is a *pre-approved*
intermediate-binding deviation (not a workaround for an unsupported construct), the specific prefix is
not mandated and the substance is correct. No action required.

**AST Consistency: PASS** (one justified, pre-approved, semantically-equivalent deviation).

---

## Verification (PASS)

`make verify-arch VERUS_EXECUTABLE_DIR=$HOME/toolchain/verus` → **exit code 0**.
- Latest compiling run logged: **`verification results:: 48 verified, 0 errors`**.
- Cheating summary: `assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2` — all
  `external_body`/`cfg_gate` outside pde and TCB-listed.
- Spec drift (`spec_drift.py git-diff … --before HEAD`): **exit 0**, 0 functions changed, 0 contract
  drift — no ensures removed, no requires added, no contract weakening.

**Verification: PASS.**

---

## Bug Summary

`bugs.md` states "No bugs found" for the five in-scope functions. **Reconciliation against final code:
accurate.**
- The four query/constructor functions are pure and total; the only overflow-bearing path
  (`frame_address`) is proven safe by `inv()` (`frame <= FrameNumber::spec_max()`) plus
  `lemma_frame_address` — a genuine proof, not a masked obligation.
- pde contains **zero `external_body`**, so no defect can be hidden behind a trust boundary in this
  module.
- No true bug, context-dependent issue, or auto-fixable defect (overflow/off-by-one/bounds/cast) was
  discovered. The "no bugs" claim is consistent with the verified contracts and the absence of any
  spec weakening.

---

## Issues (highest priority first)

1. **(Informational / non-blocking)** `frame_address` documenting comment uses the `// VERUS REWRITE`
   prefix instead of the codified `VERUS DEVIATION`. The change is a pre-approved intermediate-binding
   deviation, so this is a nomenclature nit only — no correctness or soundness impact.

No blocking issues found.

---

## Result: **PASS**

| Gate | Result |
|---|---|
| `admit()` in pde | 0 — PASS |
| `assume(...)` in pde | 0 — PASS |
| `external_body` in pde | 0 (none) — PASS |
| External `external_body` in scope | none in pde; all arch-wide TCB items in `tcb-allowed.md` — PASS |
| AST mismatch | 1, justified pre-approved semantically-equivalent deviation — PASS |
| Verification | exit 0, **48 verified, 0 errors** — PASS |
| Spec drift | exit 0, no weakening — PASS |
| Caller coverage | 6/6 invariants + 5/5 per-function — PASS |

All strict PASS criteria are satisfied.
