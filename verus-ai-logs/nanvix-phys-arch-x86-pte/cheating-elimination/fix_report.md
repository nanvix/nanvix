# Cheating Elimination Report: arch-x86-pte

## Scope

In-scope module: `src/libs/arch/src/x86/mem/paging/pte.rs` (+ `pte.spec.rs`, `pte.proof.rs`).
Verification-order target functions: `PageTableEntry::new`, `PageTableEntryFlags::new`,
`PageTableEntry::is_present`, `PageTableEntryFlags::is_present`.

## Cheating Counts (before → after)

Counts below are for the **in-scope pte module** (the cheating gate runs crate-wide; the
crate-wide residual items are accounted for under "Out-of-scope" below).

| Item                | Before | After | Eliminated |
|---------------------|--------|-------|------------|
| admit()             | 0      | 0     | 0          |
| assume()            | 0      | 0     | 0          |
| external_body       | 0      | 0     | 0          |
| assume_specification| 0      | 0     | 0          |
| cfg-gated exec      | 0      | 0     | 0          |

## Items Eliminated

None required. The `arch-x86-pte` module was already cheating-free:

- `pte.rs` — no `admit`/`assume`/`external_body`/`assume_specification`. The four target
  functions (`PageTableEntryFlags::new`, `PageTableEntry::new`,
  `PageTableEntryFlags::is_present`, `PageTableEntry::is_present`) carry real
  `#[verus_spec]` contracts and verify in-body (`PageTableEntry::new` discharges its
  `inv()` via `use_type_invariant(frame)`; the `is_present` pair and `new` flag-bundle
  prove against the `closed` `View`/`spec_pte_new`/`spec_pte_flags_new` specs).
- `pte.spec.rs` — only `open spec`/`closed spec view`/`struct` definitions; no cheating.
- `pte.proof.rs` — empty (`verus! { }`); no cheating.

The two `#[cfg(verus_keep_ghost)]` attributes at `pte.rs:9,11` gate the `include!` of the
`.spec.rs`/`.proof.rs` ghost files. This is the repository-standard spec/proof split used by
every paging module — it gates **ghost** (spec/proof) inclusion only, not exec logic, so it is
not "cfg-gated exec code" and introduces no AST divergence in executable code.

## Out-of-scope (crate-wide cheating residual — NOT in arch-x86-pte)

`make verify-arch` reports crate-wide `external_body=3` (+ the `table.proof.rs`
`lemma_entry_roundtrip` broadcast axiom). All are in sibling modules and every one is listed
in `verus-ai-logs/tcb-allowed.md`; per the hard rules they are TCB-allowed and must not be
touched from this module:

- `x86/mem/paging/mod.rs:80 invlpg` — external_body (inline `asm!`, external-bottom HW boundary).
- `x86/mem/paging/table.rs:209 read` — external_body (`usize`→ptr volatile page-table read).
- `x86/mem/paging/table.rs:246 write` — external_body (`usize`→ptr volatile page-table write).
- `x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip` — trusted broadcast codec axiom.

## Verification

- `make verify-arch`: Exit code 0 — crate verifies (cached, no recompilation). No pte function
  appears in `coverage-unverified.txt`'s blocking set; the four target functions verify.
- `git diff verus-ai-prove -- src/libs/arch/src/x86/mem/paging/`: empty — files are byte-identical
  to the base branch.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-x86-pte/verification_todo.md)

None. No proof gaps in the pte module.

## AST Consistency

- Zero mismatches confirmed: YES — the in-scope files are unchanged from `verus-ai-prove`
  (empty git diff), so exec/spec AST is identical to base. No exec code was modified; no
  cfg gates affect executable code.

## Result: PASS
