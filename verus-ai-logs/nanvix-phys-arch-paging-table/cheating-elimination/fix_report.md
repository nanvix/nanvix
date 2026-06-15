# Cheating Elimination Report: arch-paging-table

## Scope

In-scope module: `src/libs/arch/src/x86/mem/paging/table.rs` (+ `table.spec.rs`,
`table.proof.rs`). In-scope functions: `Table::write`, `TableIndex::into_raw`,
`raw`, `Table::read`, `from_raw`, `pt_index`, `TableIndex`, `pd_index`,
`Table::from_address`.

`make verify-arch` reports crate-wide cheating counters; the table below
separates the crate-wide figure from the in-scope figure. Out-of-scope items
(`mod.rs::invlpg`, `pte.rs`/`pde.rs` markers) were left untouched per the
"do not touch unlisted functions" rule.

## Cheating Counts (before → after)

| Item                | Before (in-scope) | After (in-scope) | Eliminated | Crate-wide (before→after) |
|---------------------|-------------------|------------------|------------|---------------------------|
| admit()             | 0                 | 0                | 0          | 0 → 0                     |
| assume()            | 0                 | 0                | 0          | 0 → 0                     |
| external_body       | 3 (all TCB)       | 3 (all TCB)      | 0          | 4 → 4 (all TCB)           |
| assume_specification| 0                 | 0                | 0          | 0 → 0                     |
| cfg-gated exec      | 0                 | 0                | 0          | 4 → 4 (out of scope)      |

`make verify-arch`: **exit 0** (verification passes). Status line shows
`assume=0 external_body=4 admit=0 cfg_gate=4`; every counted item is TCB-allowed
or out of scope (see below).

## Items Eliminated

None required elimination. There were **zero disallowed cheating items** in
scope:

- No `admit()`, `assume()`, or `assume_specification` anywhere in the module
  (source, spec, or proof).
- The only `external_body` items in scope are all listed in
  `verus-ai-logs/tcb-allowed.md`, so they are permitted to keep `external_body`
  per the task's explicit exception.

### In-scope `external_body` (all TCB-sanctioned — kept by allowance)

1. `table.rs::Table::<E>::read` — materializes a raw `*const PteWord` from the
   integer base (`(self.base + offset) as *const PteWord`) and performs a
   volatile load. Verus does not support `usize → *const T`
   (int-to-ptr provenance / no `PointsTo` for externally-owned volatile
   page-table memory). Escalation ladder followed: vstd has no permission token
   for an int-derived pointer; isolated reproducer reproduces the exact error
   `Verus does not support this cast: usize to *const u32`
   (`verus-unsupported.md §1`); no equivalent rewrite exists without cascading a
   ghost permission parameter (which would change the exec signature and break
   AST consistency). Carries full contract:
   `requires index@ < PAGE_TABLE_LENGTH`,
   `ensures result == spec_table_read::<E>(self@.addr, index@)`.
2. `table.rs::Table::<E>::write` — same int-to-ptr boundary (`usize → *mut T`)
   plus a volatile store. Carries only the sound
   `requires index@ < PAGE_TABLE_LENGTH`; a contents `ensures` would be unsound
   for an assumed contract (documented in `tcb-allowed.md` / `verus-unsupported.md`).
3. `table.proof.rs::lemma_entry_roundtrip` — foundational codec axiom
   `spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e)` over `uninterp`
   generic `E`; not derivable in-module (no structure on `E`). Idiomatic Verus
   axiom form (`broadcast proof fn` + `external_body`). Not in the in-scope
   function list and left unchanged.

### Out-of-scope counted items (untouched)

- `mod.rs::invlpg` — `external_body` (inline `asm!`, TCB-allowed). Out of scope.
- `pte.rs:85,307`, `pde.rs:83,307` — `#[cfg_attr(verus_keep_ghost, allow(unused,
  verus_impl_method_marker))]` lint markers counted as `cfg_gate=4`. These gate
  an `allow` lint attribute (verus tooling marker), not exec semantics, and are
  out of scope.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-paging-table/verification_todo.md)

None. There are no proof gaps (zero `admit`/`assume`). The three in-scope
`external_body` items are irreducible trust boundaries (int-to-ptr hardware
page-table access; a generic-codec axiom), not verifiable-but-unproven gaps, so
no `verification_todo.md` was created.

## AST Consistency

- `ast_consistency.py --base-ref verus-ai-prove table.rs summary`:
  `Consistent: ✅ YES (matched=7 mismatched=0 missing=0 extra=0)`.
- `git diff verus-ai-prove -- table.rs table.spec.rs table.proof.rs`: empty
  (byte-identical to base).
- Zero mismatches confirmed: **YES**. No exec code changed.

## Result: PASS

All in-scope cheating is either absent (`admit`/`assume`/`assume_specification` =
0) or TCB-sanctioned (`external_body` for `read`/`write`/`lemma_entry_roundtrip`,
all listed in `tcb-allowed.md`). `make verify-arch` exits 0, and AST consistency
is clean. No disallowed cheating remains.
