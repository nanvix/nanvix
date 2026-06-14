# Cheating Elimination Report: phys-kframe

## Cheating Counts (before → after)

Counts below are scoped to the phys-kframe files (`kframe.rs`, `kframe.spec.rs`,
`kframe.proof.rs`). "Allowed" items are enumerated in
`verus-ai-logs/tcb-allowed.md`.

| Item                | Before | After | Eliminated | Notes |
|---------------------|--------|-------|------------|-------|
| admit()             | 0      | 0     | 0          | none present |
| assume()            | 0      | 0     | 0          | none present |
| external_body       | 1      | 1     | 0          | `clear` — TCB-allowed, out of scope |
| assume_specification| 1      | 1     | 0          | `from_raw_value` — TCB-allowed external trait method |
| cfg-gated exec      | 0      | 0     | 0          | only ghost-only `#[cfg(verus_keep_ghost)]` scaffolding (spec/proof `include!`, vstd import, `View` block) — no exec-behavior gating |

In-scope functions `KernelFrame::new`, `KernelFrame::drop`, `KernelFrame::base`
contain **zero** cheating of any kind and are fully proven.

Module verification: `make verify-kernel MODULE=mm::phys` → **22 verified, 0
errors, exit 0**. Full kernel: `make verify-kernel` → **exit 0**. The harness
prints `status: CHEATING_DETECTED` only because it counts the **globally**
TCB-allowed `external_body` shims across the whole `mm::phys` subsystem (28, all
in `tcb-allowed.md`); the only kframe contributor is `clear`.

## Items Eliminated

No new cheating was introduced and none was eliminable within scope: the in-scope
functions were already proven with real contracts (address identity for `new`,
pure-read for `base`, invariant-preservation for `drop`) discharged directly from
the `View` impl and the `frame::free` shim contract — no lemmas, `admit`, or
`assume` required.

The two remaining items are required external-library trust boundaries, both
listed in `tcb-allowed.md`, neither eliminable without verifying out-of-scope
dependency modules:

- `KernelFrame::clear` (`external_body`, `kframe.rs:141`) — raw-memory `memset`
  through a `usize as *mut u8` pointer; Verus cannot model it. Also outside the
  in-scope `new`/`drop`/`base` set, so must not be touched.
- `<PageAligned<T> as Address>::from_raw_value` (`assume_specification`,
  `kframe.spec.rs:33`) — external `sys::mm::Address` trait method in the
  not-yet-verified `hal::mem` module; escalation ladder (vstd search, isolated
  reproducer, equivalent-rewrite) exhausted, see `verification_todo.md`.

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-kframe/verification_todo.md)

- `from_raw_value` assume_specification — blocked on verification of `hal::mem` /
  the `Address` trait (external trait method, no standalone-contract mechanism).
- `KernelFrame::clear` external_body — TCB raw-memory op; out of in-scope set.

No genuine proof gaps remain (no `admit()` / `assume()` anywhere in kframe).

## AST Consistency

Diff of `kframe.rs` against `verus-ai/phys-upool` shows only:
- added ghost-gated imports (`use vstd::prelude::*;`,
  `#[cfg(verus_keep_ghost)] include!("kframe.spec.rs"/"kframe.proof.rs")`,
  `#[cfg(verus_keep_ghost)]` on the existing vstd import),
- added verification annotations (`#[verus_verify]`, `#[verus_spec(...)]`).

Every exec function body (`new`, `base`, `clear`, `deref`, `deref_mut`, `drop`) is
**byte-identical** to base — semantics, time complexity, and space complexity
preserved. The `#[cfg(verus_keep_ghost)]` gates only add ghost (spec/proof/`View`)
code under verification; they do not alter exec behavior and match the established
pattern in every sibling verified file (`frame.rs`, `manager.rs`, `upool.rs`).

- Zero mismatches confirmed: YES

## Result: PASS
