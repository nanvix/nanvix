# Cheating Elimination Report: arch-paging-mod

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 1 (TCB-allowed) | 1 (TCB-allowed) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Scope note: the in-scope module is `src/libs/arch/src/x86/mem/paging/mod.rs`, whose
only verification-order target function is `invlpg`. The whole-crate `make verify-arch`
cheating summary reports `external_body=3` and `cfg_gate=2`, but the two extra
`external_body` entries (`table.rs::read`, `table.rs::write`) and the
`table.proof.rs::lemma_entry_roundtrip` entry belong to the sibling `table` module — out
of scope here and independently TCB-listed. The `cfg_gate=2` count is the standard
`#[cfg(verus_keep_ghost)] include!("mod.spec.rs"/"mod.proof.rs")` spec/proof inclusion
pattern (mod.rs lines 8–11), not cfg-gated exec code.

## Items Eliminated
- None required. The single in-scope cheating item is `invlpg`'s `external_body`, which is
  explicitly permitted by `verus-ai-logs/tcb-allowed.md`:
  > `src/libs/arch/src/x86/mem/paging/mod.rs::invlpg` — the body is a single
  > `core::arch::asm!` block issuing the `invlpg` instruction ... Verus does not support
  > inline-asm expressions, so the body cannot be verified — an external-bottom hardware
  > trust boundary ... the faithful contract is **empty**.
- Verus genuinely cannot translate inline assembly. Reproducer + exact error are recorded
  in `verus-ai-logs/nanvix-phys-arch-paging-mod/verus-unsupported.md` §1
  (`error: The verifier does not yet support the following Rust feature: inline-asm
  expressions`). The escalation ladder (search vstd / isolated reproducer / equivalent
  rewrite) terminates at the trusted hardware boundary: there is no Verus-expressible
  rewrite of a `invlpg` TLB flush, and the contract is already empty (no `requires`,
  trivial `ensures`), so no proof obligation remains to discharge.
- The `mod.spec.rs` and `mod.proof.rs` files are empty (`verus! { }`); there is no
  spec/proof-level cheating to remove.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-paging-mod/verification_todo.md)
- None. No proof gaps, no `admit()`, no `assume()` remain in scope.

## AST Consistency
- Zero mismatches confirmed: YES. `git diff verus-ai-prove -- mod.rs mod.spec.rs
  mod.proof.rs` is empty — the in-scope files are byte-identical to the base branch, so no
  exec code, signatures, or complexity were altered.

## Result: PASS
- `make verify-arch` exits 0 (verification passes). The only in-scope cheating item
  (`invlpg` `external_body`) is TCB-allowed for an unsupported-by-Verus inline-asm hardware
  boundary; zero non-allowed cheating remains in scope.
