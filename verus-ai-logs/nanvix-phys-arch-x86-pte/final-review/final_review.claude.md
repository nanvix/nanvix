# Final Verification Review — `arch::x86::mem::paging::pte`

**Reviewer:** Independent strict final verification (Claude)
**Date:** 2026-06-15
**Module:** `src/libs/arch/src/x86/mem/paging/pte.rs` (+ `pte.spec.rs`, `pte.proof.rs`)
**In-scope functions (verification order):**
`PageTableEntry::new`, `PageTableEntryFlags::new`,
`PageTableEntry::is_present`, `PageTableEntryFlags::is_present`

All findings below were re-derived with tools (grep, view, the AST-consistency and
spec-drift scripts), not taken from the provided summary on trust.

---

## Spec Quality

**Verdict: PASS.**

### The four in-scope contracts (read from `pte.rs`)

| Function | Contract | Assessment |
|----------|----------|------------|
| `PageTableEntryFlags::new` (L86–97) | `ensures result@ == spec_pte_flags_new(present, read_write, user_supervisor, page_write_through, page_cache_disable, accessed, dirty)` | Pins all **seven** argument bits faithfully and defaults `cow == false`. Total/infallible (no `requires`, returns `Self`). Correct & complete. |
| `PageTableEntryFlags::is_present` (L179–181) | `ensures result == self@.present` | Pure projection of the present bit. Correct. |
| `PageTableEntry::new` (L308–312) | `ensures result@ == spec_pte_new(flags@, frame@), result.inv()` | Constructor fidelity (flags+frame) **and** well-formedness (frame bound). `result.inv()` discharged in-body from `use_type_invariant(frame)`. Correct & complete. |
| `PageTableEntry::is_present` (L406–408) | `ensures result == self@.flags.present` | Presence delegation expressed as a one-line identity through the composed view. Correct. |

These are external-top API contracts: `closed` views hide the bit-packing, `requires`
are empty (the constructors are genuinely total), `ensures` capture exactly the
caller-observable facts. No error-path / code-as-spec anti-patterns (the specs say
*what* — "records each bit", "cow defaults to false", "present delegates to flags" —
never *how* bits are packed). Consistent with the spec-design skill.

### View design (`PteView` / `PteFlagsView`)

- **Mathematical types:** `PteFlagsView` is eight `bool`s; `PteView` is
  `{ flags: PteFlagsView, frame: int }`. No exec enums or `usize` leak into spec
  world (the two-valued flag enums are projected to `bool` via the `spec_*_set`
  helpers). ✔
- **Caller-observable state only:** present, cow, the other six permission/state
  bits, and the frame index — exactly the caller mental model. No `PteWord`, no
  inner `FrameNumber` representation leaks (`view()` is `closed`). ✔
- **Substitution test:** `view_design.md` documents a per-field substitution table;
  every field survives a hypothetical re-encoding (present bit, cow AVL bit,
  permissions, frame index are all architectural, not impl artifacts). ✔
- **`inv()` encodes real constraints (not trivially true):**
  - `PageTableEntry::inv` = `0 <= self@.frame <= FrameNumber::spec_max()` — a **real,
    non-trivial** bound inherited verbatim from the `FrameNumber` type invariant. It
    underwrites the (out-of-scope) `frame_address` totality and keeps `new`'s result
    well-formed. ✔
  - `PageTableEntryFlags::inv` = `true` — **justified.** A flags bundle has no
    cross-field constraint: all 2⁸ combinations are constructible via `new` + the
    `set_*` mutators; `unmap` legitimately builds an all-off set, and `cow` is set
    independently of `present`. A non-vacuous invariant (e.g. `cow ⇒ present`) would
    reject legal callers. `view_design.md` explicitly lists this in *Rejected
    Alternatives*. Vacuous-but-documented is the correct choice here, not a defect.

`spec_pte_flags_new` / `spec_pte_new` are reusable spec helpers living with the View
(per skill), not extra `pub spec fn`s on the exec impl. The seven shared flag
projections are reused from the `pde` sibling; the PTE-specific `spec_cow_set` is
defined locally (`pte.spec.rs:22`) — no duplication.

---

## Caller Coverage

**Covered: 4/4.** No missing properties.

Source: `caller_analysis.md` (real call sites in `kernel`, since the LSP run reported
a documented false-negative of 0 callers).

| In-scope fn | Caller expectation | Mapped to |
|-------------|--------------------|-----------|
| `PageTableEntryFlags::new` | Returned set reflects **exactly** the 7 args; `is_present() == (present==Present)` | `result@ == spec_pte_flags_new(..)` sets each of the 7 bits via `spec_*_set` |
| | `cow` defaults to `NotCopyOnWrite` | `spec_pte_flags_new` fixes `cow: false` (load-bearing; lets `unmap` conclude `is_cow()==false`) |
| | Total/infallible, no panic | No `requires`, returns `Self` (no `Result`) |
| | Don't care about raw layout | `view()` is `closed` |
| `PageTableEntry::new` | Stores flags+frame faithfully: `is_present()==flags.is_present()`, `frame_number()==frame`, `flags()` equivalent | `result@ == spec_pte_new(flags@, frame@)` ⇒ `result@.flags==flags@ ∧ result@.frame==frame@` |
| | Infallible; immediately serializable; well-formed | No `Result`; `result.inv()` (frame bound) |
| | Don't care about internal packing | `closed` view |
| `PageTableEntry::is_present` | `true` iff present bit set, mirrors `self.flags()`; pure | `ensures result == self@.flags.present`; `&self` query, no `&mut` |
| `PageTableEntryFlags::is_present` | `true` iff constructed with `Present` (the `fill` guard) | `ensures result == self@.present` |

**Boundary obligation (not an in-scope gap):** the `TableEntry` round-trip
(`from_raw`/`into_raw_value` recover `is_present`/`frame`/flags) is explicitly
out-of-scope per `caller_analysis.md`; `from_raw_value`/`into_raw_value` are not in
the four-function scope and correctly carry no contract. Derived caller expectations
that route through out-of-scope accessors (`flags()`, `frame_number()`) are captured
at the view level by `new`'s `result@` equality, which is the in-scope guarantee.

---

## Proof Completeness

**Verdict: PASS — no proof gaps.**

| Artifact | Count | Locations |
|----------|-------|-----------|
| `admit()` in pte files | **0** | — |
| `external_body` in pte files | **0** | — |

- `pte.proof.rs` is empty (`verus! { }`) — no lemmas, hence no admits.
- `pte.spec.rs` contains only `open spec fn`s and `View`/`inv` impls — no admits.
- `pte.rs`'s sole proof block is `proof! { use_type_invariant(frame); }`
  (`PageTableEntry::new`, L314) — a sound in-body discharge of `result.inv()`, **not**
  an `admit`.

Any `admit()` would be a BLOCKER; there are none. Any `external_body` not in
`tcb-allowed.md` would be a BLOCKER; there are none in the pte files at all.

---

## TCB Compliance

**Verdict: PASS.**

The pte module introduces **zero** `external_body`, so there is nothing to list and
nothing unlisted. `tcb-allowed.md` correctly contains **no** pte-module entries. The
only `arch` crate `external_body` boundaries (`mod.rs::invlpg`, `table.rs::read`,
`table.rs::write`) are out of pte scope and are already TCB-listed. No new boundaries
were added or justified.

---

## Guardrails Compliance

Exact counts across `pte.rs`, `pte.spec.rs`, `pte.proof.rs`:

| Dimension | Count | Locations |
|-----------|-------|-----------|
| `admit` | **0** | — |
| `assume` | **0** | — |
| `external_body` | **0** | — |
| `assume_specification` | **0** | — |
| cfg-gated exec | **0** | — |

Notes:
- The two `#[cfg(verus_keep_ghost)] include!("pte.spec.rs"/"pte.proof.rs")` lines
  (`pte.rs:9–12`) are the **standard** ghost-include pattern — **not** cfg-gated exec.
- The two `#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]`
  attributes (`pte.rs:85, 307`) are Verus marker attributes on the two constructors —
  they gate *attributes*, not exec bodies. Not cfg-gated exec.

`admit > 0` or `assume > 0` would be a BLOCKER; both are 0.

---

## AST Consistency

**Verdict: PASS.**

`python3 ast_consistency.py src/libs/arch/src/x86/mem/paging/pte.rs` →
`✅ All exec functions consistent`, **23/23 matched, 0 mismatched, 0 missing, 0 extra**,
exit 0. No `// VERUS REWRITE` comments exist in the file (grep returns none), so there
are no rewrites whose semantic equivalence needs auditing. No MISMATCH.

---

## Spec Drift

**Verdict: PASS.**

`python3 spec_drift.py git-diff src/libs/arch/src/x86/mem/paging/pte.rs --before HEAD`
→ `✅ No contract drift detected` (0 functions changed, 0 ensures removed, 0 requires
added), exit 0. The pte spec work is already committed at HEAD (`9c2fffa28`), and the
pre-spec baseline carried **no** contracts on these functions, so the current state
strictly **adds** `ensures` (strengthening) — no guarantee was weakened.

Placeholder removal confirmed in `src/kernel/src/mm/virt/identity_map.spec.rs`:
- L123–127: the `ExPageTableEntry` / `ExPageTableEntryFlags` external type specs were
  removed (now real `#[verus_verify]` modeling in `arch`).
- L171–176: the former placeholder `assume_specification`s for
  `PageTableEntryFlags::new`, `PageTableEntry::new`, and `PageTableEntry::is_present`
  were removed — real arch contracts supersede them.
- A grep of that file shows **no** surviving `assume_specification` for any of the
  four in-scope functions (the only remaining `assume_specification` is for
  `bump_allocator::FixedSizeBumpAllocator::new`, unrelated).

---

## Verification

**Verdict: PASS** (using the orchestrator-provided results; not re-run here to avoid
cargo-lock contention).

- `make verify-arch` = PASS, exit 0 (cached).
- `./z build -- all` = PASS.
- `make verify` (full cross-module) = PASS, exit 0; all verified modules pass.
- pte module cheating counts: `admit=0 assume=0 external_body=0 assume_specification=0`
  — independently re-derived via grep on the pte files (matches).
- Crate-wide `external_body=3` (mod.rs `invlpg`, table.rs `read`/`write`) — all out of
  pte scope and TCB-listed. HEAD commit message corroborates:
  `arch::all (48 verified, 0 errors, external_body=3 cfg_gate=2)`.

---

## Bug Summary

`bugs.md` states **"None"**, and that reconciles with the final code state:

- All four in-scope functions verify cleanly (`6 verified, 0 errors`, `admit=0`,
  `assume=0`), no proof gaps, no `external_body`.
- The only non-trivial obligation — `PageTableEntry::new`'s `result.inv()` frame
  bound — is discharged soundly from the `FrameNumber` type invariant via
  `use_type_invariant(frame)`; no overflow, off-by-one, or unchecked cast.
- There are **no surviving unresolved verification failures** to classify under the
  bug-reporting skill (`make verify` passes). No bug was discovered during proving
  that went unrecorded.

"None" is a **valid** conclusion.

---

## Issues

None. Every dimension is clean.

(For completeness, the one judgement-call item — `PageTableEntryFlags::inv() == true`
— is **not** an issue: it is correctly justified by the absence of any cross-field
hardware/OS constraint and is documented with rejected alternatives in
`view_design.md`.)

---

## Result: **PASS**

| Dimension | Result |
|-----------|--------|
| Spec quality (4 contracts + views + inv) | PASS |
| Caller coverage | PASS (4/4, none missing) |
| Proof completeness (admit=0, external_body=0) | PASS |
| TCB compliance | PASS (no pte external_body) |
| Guardrails (admit/assume/external_body/assume_spec/cfg-exec all 0) | PASS |
| AST consistency (23/23) | PASS |
| Spec drift (no weakening; placeholders removed) | PASS |
| Verification (`make verify` exit 0) | PASS |
| Bug reconciliation ("None" valid) | PASS |

**Final: PASS.** The `arch-x86-pte` module is clean on every strict dimension: no
`admit`, no `assume`, no unlisted `external_body`, no `assume_specification`, no
cfg-gated exec, no AST mismatch, no spec weakening, verification passes, and all four
in-scope caller expectations are covered by real `requires`/`ensures` contracts.
