# Bug Report — `sys::mm::address` (`Address` trait), proving phase

None.

## Scope

In-scope file: `src/libs/sys/src/sys/mm/address/mod.rs` (plus `mod.spec.rs`,
`mod.proof.rs`). In-scope functions: `is_aligned`, `into_raw_value`,
`from_raw_value`.

## Findings

All three target functions are **trait method declarations** on the `Address`
trait. They carry verification contracts (`#[verus_spec]` ensures) but have no
executable bodies — the bodies live in the concrete implementors (e.g.
`VirtualAddress` in the out-of-scope `sys::mm::address::virt` module). A trait
declaration with no default body has no proof obligation of its own, so there
were no proof bodies to fill, no loop invariants to write, and no `admit()` /
`assume()` placeholders to remove.

The contracts themselves were checked for well-formedness and soundness:

- `from_raw_value` — `Ok(a) => a@ == raw_addr as int` and
  `Err(e) => e.code == BadAddress`. Satisfiable by every implementor; the
  stricter (sparse) validity of `PhysicalAddress` only narrows the `Err` arm,
  which the trait leaves open, so there is no contradiction.
- `into_raw_value` — `result as int == self@`. Lossless projection consistent
  with the `int` view of every implementor.
- `is_aligned` — `Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)`,
  where `spec_addr_is_aligned(v, a) == (v % spec_align_value(a) == 0)`. This is a
  declarative alignment predicate independent of how the bitmask check is
  computed; it matches implementor behavior and caller expectations.

## Verification Result

Module `sys::mm::address` verifies cleanly: `2 verified, 0 errors`,
status CLEAN, with no cheating patterns in the module
(`assume=0 external_body=0 admit=0 trusted=0 no_decreases=0`).

Note: the crate-wide cheating scan reports `cfg_gate=1`, which originates from a
pre-existing `#[cfg(verus_keep_ghost)] verus! { ... }` spec block in
`src/libs/sys/src/sys/mm/alignment.rs` (the `spec_align_value` definition). That
is in the out-of-scope `mm::alignment` module and is a legitimate cfg-gated spec
block, not introduced by this work.

Status: clean.
