# Cheating Elimination Report: arch-paging-mod

## Scope

In-scope function (per verification-order target): `invlpg` only.
Files: `mod.rs`, `mod.spec.rs`, `mod.proof.rs` under
`src/libs/arch/src/x86/mem/paging/`.

Out-of-scope cheating items reported crate-wide by `make verify-arch`
(`table.rs::read`, `table.rs::write`, `table.proof.rs::lemma_entry_roundtrip`)
belong to the `table` module and were **not touched** (hard rule: do not touch
unlisted functions). They are tracked under their own module's elimination phase.

## Cheating Counts (before → after) — in-scope (mod.* files only)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 1 (allowlisted) | 1 (allowlisted) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

`mod.spec.rs` and `mod.proof.rs` are empty (`verus! { }`). The only cheating
construct in `mod.rs` is the `external_body` on `invlpg`.

## Items Eliminated

None required. The single in-scope cheating construct is **explicitly
allowlisted** and therefore not a blocker:

- `src/libs/arch/src/x86/mem/paging/mod.rs::invlpg` — `#[verus_verify(external_body)]`.
  The body is a single `core::arch::asm!` block issuing the `invlpg` instruction
  (flushes the CPU TLB entry for `vaddr`). Verus does not support inline-asm
  expressions, so the body cannot be verified — an **external-bottom hardware
  trust boundary** (same class as `table::read`/`write` volatile access and
  `frame::instance` int-to-pointer materialization). It is listed verbatim in
  `verus-ai-logs/tcb-allowed.md` under "*`external_body` introduced while speccing
  `arch::x86::mem::paging` (`mod.rs`)*" and documented in
  `verus-ai-logs/nanvix-phys-arch-paging-mod/verus-unsupported.md`.
  The faithful contract is empty (no `requires`, trivial `ensures`): the effect is
  purely on hardware TLB state, outside Verus' memory model and invisible to every
  caller's Rust-visible state, so every caller-side invariant is preserved. This
  matches the inherited upstream
  `assume_specification[ ::arch::mem::paging::invlpg ]` (no `requires`/`ensures`).

The two `#[cfg(verus_keep_ghost)]` attributes in `mod.rs` gate only the
`include!("mod.spec.rs")` / `include!("mod.proof.rs")` lines — the project-standard
spec/proof inclusion pattern, **not** exec-code gating — so they are not a cfg-gated
exec deviation.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-paging-mod/verification_todo.md)

None. No proof gaps (`admit`/`assume`) exist in the in-scope files, so no
verification TODO was recorded.

## AST Consistency

- Zero mismatches confirmed: YES.
  `git diff verus-ai-prove-bottom-up -- <mod.rs|mod.spec.rs|mod.proof.rs>` is empty;
  all three files are byte-identical to the base branch. No exec signature, body,
  semantics, time complexity, or space complexity changed.

## Result: PASS

The only in-scope cheating construct (`invlpg` `external_body`) is on the
TCB allowlist (`tcb-allowed.md`), which the task explicitly permits. No
`admit`/`assume`/non-allowlisted `external_body`/`assume_specification`/cfg-gated
exec code remains in scope. `make verify-arch` exits 0 (verification passes);
the crate-wide `CHEATING_DETECTED` status is driven solely by out-of-scope
allowlisted/other-module items.
