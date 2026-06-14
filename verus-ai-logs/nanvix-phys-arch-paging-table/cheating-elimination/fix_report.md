# Cheating Elimination Report: arch-paging-table

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 2 (TCB-allowed) | 2 (TCB-allowed) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Counts are for the in-scope module `arch::x86::mem::paging::table` (source
`table.rs`, spec `table.spec.rs`, proof `table.proof.rs`). The full `arch`
crate reports `external_body=3`; the third is `paging/mod.rs::invlpg`, which is
out of scope for this module and is itself TCB-allowed.

## Items Eliminated
None required. No unauthorized cheating exists in this module.

Detailed findings:

- **`Table::<E>::read` — `external_body` (table.rs:202/209).** Listed in
  `verus-ai-logs/tcb-allowed.md` ("`external_body` introduced while speccing
  `arch::x86::mem::paging::table`"). Materializes a raw `*const PteWord` from
  the integer base address (`usize as *const`) and performs a volatile load —
  an int-to-ptr / volatile externally-owned-memory boundary Verus cannot model
  (same class as `bump_allocator::alloc` and `frame::instance`). It is **not**
  contract-free: `requires index@ < PAGE_TABLE_LENGTH`,
  `ensures result == spec_table_read::<E>(self@.addr, index@)`. Allowed to keep
  `external_body`.

- **`Table::<E>::write` — `external_body` (table.rs:241/246).** Listed in
  `tcb-allowed.md`. Same int-to-ptr / volatile-store boundary. Carries the
  sound `requires index@ < PAGE_TABLE_LENGTH`; the slot-update transition is
  deliberately deferred to the proving-phase page-table permission token (a
  contents `ensures` on an assumed `external_body` would be unsound — documented
  exploit in `tcb-allowed.md`). Allowed to keep `external_body`.

- **Spec-phase axiom already removed.** `table.proof.rs` documents that the
  former `lemma_entry_roundtrip` placeholder (an unproven axiom) was already
  removed rather than left as cheating; no proof depends on it. Confirmed: no
  `admit`/`assume`/`assume_specification` in any of the three files.

- **cfg gates.** The only `#[cfg(...)]` in `table.rs` are the standard
  `#[cfg(verus_keep_ghost)] include!("table.spec.rs"/"table.proof.rs")` ghost
  includes — not cfg-gated exec divergence.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-paging-table/verification_todo.md)
None. There are no `admit()`/`assume()` proof gaps in this module. The
read-after-write slot transition is an intentional, documented proving-phase
deferral (page-table permission token), not a stuck proof — consistent with the
`identity_map_view()` deferral convention.

## AST Consistency
- Zero mismatches confirmed: YES.
- `table.rs`, `table.spec.rs`, and `table.proof.rs` are byte-identical to
  `verus-ai-prove-bottom-up` (`git diff` empty; working tree clean). No exec
  signatures, semantics, time complexity, or space complexity changed.

## Verification
- `make verify-arch`: exit 0 (verification passed; cheating check reports only
  the 3 TCB-allowed `external_body`). No regressions possible — no files were
  modified relative to the base branch.

## Result: PASS
