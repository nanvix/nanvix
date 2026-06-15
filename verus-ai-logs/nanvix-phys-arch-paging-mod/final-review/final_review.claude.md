# Final Verification Review — `arch-paging-mod` (`invlpg`)

Independent, strict final review of the `arch::x86::mem::paging` module (`mod.rs`).
In-scope target: the single function `invlpg` in
`src/libs/arch/src/x86/mem/paging/mod.rs`.

## Checklist

### Caller Analysis
- [x] `caller_analysis.md` present, identifies real callers (LSP false-negative corrected by manual search: `identity_map.rs:668`, `page_table.rs:210/329/385/433/498`, `page_directory.rs:170`).
- [x] Trait obligations: none (free `unsafe fn`).
- [x] All caller expectations enumerated.

### View Design
- [x] `view_design.md` present; deliberate empty/degenerate `PagingView` justified (TLB is hardware state outside Verus' memory model).
- [x] Substitution test applied to every candidate field; all rejected with rationale.
- [x] No abstract state to carry across the call — correct for a side-effect-only hardware shim.

### Specification
- [x] Contract is faithfully empty: no `requires`, trivial `ensures` — matches the inherited upstream `assume_specification[ ::arch::mem::paging::invlpg ]`.
- [x] `mod.spec.rs` / `mod.proof.rs` are empty (`verus! { }`) — consistent with the no-View design.
- [x] Trust-boundary comment at `mod.rs:69-77` documents the boundary and cross-references `tcb-allowed.md` and `verus-unsupported.md`.

### Proving
- [x] Verification passes: exit 0, 0 errors.
- [x] No remaining `admit()` in the in-scope module.

### Cheating Elimination
- [x] `admit=0`, `assume=0`, `assume_specification=0`, cfg-gated exec = 0 for the in-scope module.
- [x] Single `external_body` (`invlpg`, `mod.rs:80`) is listed in `tcb-allowed.md`.
- [x] The two `#[cfg(verus_keep_ghost)]` directives are the standard `include!()` of spec/proof files — NOT cfg-gated exec code.

### Bug Recording
- [x] No `bugs.md` exists; the only obstruction is a Verus limitation (inline-asm unsupported), which is NOT a bug. No real code defect found or unrecorded.

---

## Spec Quality

`invlpg`'s sole effect is invalidating a CPU TLB entry — hardware microarchitectural
state that lives entirely outside Verus' memory model and is invisible to every
caller's Rust-visible state. The function:
- takes any `usize` (the instruction is defined for every operand; a no-op when no
  matching TLB entry exists),
- returns `()` with no error path,
- touches no page tables, frames, or any Rust-visible state.

Therefore the **faithful and complete** contract is empty: no `requires`, trivial
`ensures`. This is not under-specification — there is genuinely no caller-observable
abstract state or failure mode to describe, and `view_design.md` exhaustively
rejects every candidate field (TLB set/map, flush log, last-vaddr, status bool,
ring-0 precondition) as abstraction leaks or over-faithful modeling. The empty
contract is identical to the inherited upstream spec. The ring-0 safety obligation
is correctly left to the `unsafe` caller (not Verus-checkable here) and documented
in the `# Safety` doc comment. The trust-boundary comment block (`mod.rs:69-77`) is
clear and self-contained. **Spec quality: PASS.**

## Caller Coverage

Callers: 7 call sites across 3 files (identity_map ×1, page_table ×5, page_directory ×1).
All follow one pattern: after writing/clearing a PTE/PDE, flush the TLB for the address.

Caller expectations vs. contract:

| # | Caller expectation | Covered? |
|---|--------------------|----------|
| 1 | Pure side-effecting op on TLB; returns `()`, no error path | ✅ trivial `ensures`, `-> ()` |
| 2 | Accepts any `usize`; no range-check relied upon | ✅ no `requires` |
| 3 | Does not read/modify page tables, kernel memory, or any Rust-visible state → preserves all caller invariants | ✅ empty footprint (external_body, no abstract mutation) |
| 4 | Infallible; success conveys only "instruction issued" | ✅ trivial `ensures(true)` |
| 5 | Ring-0 safety is caller's responsibility | ✅ left to `unsafe` caller; documented in `# Safety` |

**Covered: 5/5. Missing: none.** (Usage-ordering — call *after* updating the PTE/PDE
— is a caller-enforced discipline, not enforceable by `invlpg`, and correctly not
modeled.)

## Proof Completeness

- `admit()` count in in-scope module: **0** (locations: none).
- `external_body` NOT in `tcb-allowed.md`: **0**. The single `external_body`
  (`invlpg`, `mod.rs:80`) IS listed in `tcb-allowed.md` (line 70).

## TCB Compliance

**YES.** Every `external_body` in the in-scope module is pre-approved.
`invlpg` (`mod.rs:80`) is explicitly listed in `tcb-allowed.md` under "external_body
introduced while speccing `arch::x86::mem::paging` (`mod.rs`)" (lines 68–83) with the
inline-asm hardware trust-boundary rationale. No new or unlisted trust boundary.
Not-in-approved-TCB list: none.

## Guardrails Compliance

For the in-scope module (`mod.rs` + `mod.spec.rs` + `mod.proof.rs`):

```
admit:0  assume:0  external_body:1  assume_specification:0  cfg-gated-exec:0
```

- `external_body:1` → `invlpg` at `mod.rs:80` (TCB-listed; not a blocker).
- `cfg-gated-exec:0` → the two `#[cfg(verus_keep_ghost)]` at `mod.rs:8,10` gate
  `include!("mod.spec.rs")` / `include!("mod.proof.rs")` — the standard allowed
  pattern, not exec code.
- No BLOCKERs: `admit=0`, `assume=0`, and the only `external_body` is approved.

(Whole-crate cheating reported by `make verify-arch`: `assume=0 external_body=3
admit=0 trusted=0 no_decreases=0 cfg_gate=2`. The other two `external_body` are
`table::read`/`table::write` — out of scope and separately TCB-listed.)

## AST Consistency

**PASS.** `ast_consistency.py ... count` → `✅ Consistent: 1 functions, 0 structs match.`
The `invlpg` exec body is unchanged (confirmed: AST hash match + `body_removed_source.rs`
strips only the body). No `// VERUS REWRITE`, `VERUS DEVIATION`, or `VERUS BUG FIX`
comments exist in the three module files — there is nothing to inspect for semantic
equivalence because the exec code was never rewritten.

## Verification

**verus PASS — exit code 0, 0 errors** (`48 verified, 0 errors` historically; current
run cached, exit 0).

Cheating-pattern line:
```
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```

## Bug Summary

- `bugs.md`: does not exist.
- Total bugs: **0**. True bugs: **0**.

The only obstruction to in-body verification is that Verus does not support inline-asm
expressions (`core::arch::asm!`). Per the `bug-reporting` skill, a verifier limitation
is explicitly NOT a bug; it was correctly handled as an external-bottom hardware trust
boundary (`external_body` + `tcb-allowed.md` listing) and documented in
`verus-unsupported.md`. No real code defect was discovered or left unrecorded. A
literal "None" `bugs.md` is absent, which is a trivial process nit, not a defect.

## Issues

1. (Cosmetic / process) No `bugs.md` file exists. The `bug-reporting` skill says to
   write "None" when no bugs are found. Absence is harmless here (no bugs exist) and
   not a blocker. No action strictly required.

No correctness, spec, TCB, AST, or guardrail issues found.

## Result: PASS

All checklist items pass. In-scope counts: `admit=0 assume=0 external_body=1
(TCB-approved) assume_specification=0 cfg-gated-exec=0`. Verus exit 0 / 0 errors. AST
consistent. Empty contract is faithful and complete; all 5 caller expectations covered.
No BLOCKERs.
