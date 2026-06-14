# Cheating Elimination Report: hal-frame-address

## Cheating Counts (before → after)
Scope: the `hal::mem::types::address` module files flagged for this task
(`frame.rs`, `frame.spec.rs`, `frame.proof.rs`, and the sibling proof helpers in
`phys.proof.rs` / `phys.rs` on which the frame conversions depend).

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 2 | 0 | 2 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 1 | 1 | 0 (TCB-approved; gate does not count it) |
| cfg-gated exec | 0 | 0 | 0 |

(Round 1 had additionally eliminated 1 cfg-gated-exec hit in `frame.rs`.)

Crate-global totals fell from `admit=29` to `admit=27`; the remaining 27 admits and
the `external_body=11` / `cfg_gate=14` belong to other, out-of-scope kernel modules.
Verification: `make verify-kernel` exit 0 (0 errors); `make verify` all crates exit 0.

## Items Eliminated
- **`phys.proof.rs:10` `lemma_from_number_no_overflow` — admit → real proof.**
  Proves `frame@ * PAGE_SIZE <= usize::MAX`. The frame-index bound
  `0 <= frame@ <= FrameNumber::spec_max()` is supplied by the caller through
  `FrameNumber::into_raw_value`'s postcondition; the nonlinear product bound is
  discharged with `vstd::arithmetic` lemmas (`lemma_mod_bound`,
  `lemma_fundamental_div_mod`, `lemma_mul_inequality`) plus a small
  `by (nonlinear_arith)` block — the exact structure of `arch`'s already-verified
  `pde.proof.rs::lemma_frame_address`.
- **`phys.proof.rs:32` `lemma_frame_index` — admit → real proof.**
  Proves `(raw_addr >> shift) == addr@ / PAGE_SIZE` and `<= spec_max_frame_number()`.
  Uses `vstd::bits::lemma_usize_shr_is_div` for the shift-equals-divide identity and
  `addr.inv()` for the bound. The `requires` was corrected from `shift < 64` to
  `shift < usize::BITS` because the kernel target is 32-bit (`usize::BITS == 32`),
  which is what `lemma_usize_shr_is_div` requires; the caller `into_frame_number`
  passes `shift == FRAME_SHIFT == 12`, satisfying it.

## Supporting exec deviation (required, documented)
- **`phys.rs::PhysicalAddress::from_number`** — the base-address multiply was bound to
  a `let` first:
  `let raw_value = frame.into_raw_value(); proof!{ lemma... } let addr = raw_value * FRAME_SIZE;`
  This is the **pre-approved "intermediate value" deviation** (ast-consistency skill:
  `f(complex_expr)` → `let x = complex_expr; f(x)`). It is **required** because:
  (1) cross-crate `use_type_invariant` on `arch`'s `FrameNumber` raises
  `Verus Internal Error: missing type invariant function` (evidence captured during
  iteration), and (2) parameter type-invariants are not auto-assumed
  (`precondition not satisfied` on the frame bound). The only sound source of the
  bound is `into_raw_value`'s postcondition, which must be in scope before the lemma
  call. Evaluation order, effects, time and space complexity are identical to the
  original `frame.into_raw_value() * mem::FRAME_SIZE`. A `VERUS DEVIATION` comment
  documents this in-source. Non-ghost `cargo build` succeeds unchanged.

## Verification TODOs
See `verus-ai-logs/nanvix-phys-hal-frame-address/verification_todo.md`. No proof gaps
remain. One TCB-approved `assume_specification` (`PhysicalAddress::from_raw_value`)
stays per `tcb-allowed.md`; it is not counted by the cheating gate.

## AST Consistency
- `frame.rs`: zero mismatches (9 functions, 1 struct MATCH vs task-start `2140b513e`).
- `phys.rs`: one MISMATCH — `from_number` — which is the documented, pre-approved
  "intermediate value" deviation above (required by a genuine Verus cross-crate
  limitation; semantics/time/space preserved; non-ghost build verified). All other
  16 functions and the struct MATCH.

## Result: PASS
