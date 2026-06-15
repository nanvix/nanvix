# Cheating Elimination Report: virt-identity-map

Module: `mm::virt::identity_map`
Files: `identity_map.rs`, `identity_map.spec.rs`, `identity_map.proof.rs`
Verify command: `make verify-kernel MODULE=mm::virt::identity_map` (module) and
`make verify` (full crate suite). Both exit 0.

## Cheating Counts (before → after)

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 3      | 3     | 0 (all TCB-allowed) |
| assume_specification | 2      | 1     | 1          |
| cfg-gated exec       | 0      | 0     | 0          |

Notes:
- `external_body = 3` are the three in-scope exec functions `ensure_pt`, `ensure_pte`,
  `identity_map_page`. All three are explicitly TCB-listed in
  `verus-ai-logs/tcb-allowed.md` (section "`external_body` introduced while speccing
  `mm::virt::identity_map`"), i.e. they are the codebase-sanctioned exception, not a blocker.
- One additional TCB-listed item exists in the spec file: `ExPageTableBss`
  (`#[verifier::external_type_specification] + #[verifier::external_body]`), also listed in
  `tcb-allowed.md`. It registers the opaque external `PageTableBss` BSS-pool type; counted by
  the gate as `external_type_spec`, kept as a sanctioned exception.
- The module cheating gate reports `assume=0 admit=0` for this module; the only flagged items
  are the 3 TCB `external_body` functions and the 1 TCB `external_type_spec`. No non-TCB
  cheating remains.

## Items Eliminated

- **`assume_specification` for `<[T]>::as_ptr`** (was `identity_map.spec.rs:179`) — **REMOVED.**
  This placeholder was only needed to translate the body of `ensure_pt`
  (`slot.as_ptr() as usize`). Since `ensure_pt` is `#[verus_verify(external_body)]` (TCB-listed),
  its body is no longer translated by Verus, making the declaration dead. It is the only
  `<[T]>::as_ptr` `assume_specification` in the kernel crate, and the remaining `.as_ptr()`
  callers in the crate (in `hal/...`) are not in verified scope. Removal verified: module
  `6 verified, 0 errors`; full `make verify` exit 0 (no regression). Replaced with an
  explanatory comment.

## Items kept (sanctioned, not blockers)

- **3 `external_body`**: `ensure_pt`, `ensure_pte`, `identity_map_page`. TCB-listed. The proving
  phase exhaustively established these cannot be verified in-body under the hard rules (fixed
  exec signatures, no new trust boundaries, no spec weakening). Full analysis in
  `verus-ai-logs/nanvix-phys-virt-identity-map/verification-todo.md` and `bugs.md`. The
  irreducible blockers are: (a) `alloc_as`'s `bump_view(self).inv()` precondition has no
  establishing lemma/type-invariant anywhere in `src`; (b) `Table::write` is deliberately
  contents-free (a contents postcondition is documented-unsound), so no PTE-write → `mapped`
  link exists; (c) `KERNEL_PD_PADDR.load()` carries no atomic→view spec. These are the
  `mm::virt` counterparts of the already-TCB-listed `mm::phys` wrappers
  (`frame::alloc`/`book`/`instance`, `kframe::new`) — `kframe::new` is in fact TCB-listed
  *because* it calls `identity_map_page`.
- **`ExPageTableBss`** external_type_specification — TCB-listed; opaque external BSS-pool type.
- **`assume_specification` for `FixedSizeBumpAllocator::<N,A,S>::new`**
  (`identity_map.spec.rs`) — required to translate the out-of-scope `#[verus_verify]` static
  `PAGE_TABLE_ALLOCATOR` (`page_table_allocator.rs:100`), whose initializer calls `new()`.
  `new` lives in the `bump_allocator` crate and currently has no `#[verus_spec]`, so this is a
  "not-yet-verified callee" placeholder (the documented class in `tcb-allowed.md` that is
  superseded once the dependency module is verified). It cannot be eliminated without touching
  out-of-scope code (the `bump_allocator` crate or the `page_table_allocator` static), and it is
  not counted as cheating by the gate (`assume=0`).

## Documentation accuracy fixes

- `identity_map.proof.rs` header: updated a stale comment that claimed the lemma bodies were
  `admit()` placeholders. The lemmas (`lemma_install_page_maps`,
  `lemma_install_page_monotone`, `lemma_install_page_preserves_inv`, `lemma_map_page_accessible`,
  `lemma_map_page_preserves_inv`) carry fully discharged proof bodies — no `admit()`/`assume()`.
- `identity_map.spec.rs` header: updated a stale comment that said the intra-call obligations are
  "currently `admit()`-ed in the exec bodies" — they are now covered by the
  `#[verus_verify(external_body)]` TCB boundary.

## Verification TODOs (verus-ai-logs/nanvix-phys-virt-identity-map/verification-todo.md)

- No remaining `admit()`/`assume()` proof gaps — there are **zero** in this module.
- The honest hand-off for the three TCB `external_body` functions (the deferred `mm::virt`
  identity-map permission-token realization) is already recorded in `verification-todo.md`.
  Realizing that cross-cutting ghost/permission token is outside the fixed-signature, in-body
  scope of this task and would require touching out-of-scope callers (`init`,
  `KernelFrame::new`). Until then these three remain in the identical sanctioned state as their
  `mm::phys` analogues (`frame::alloc`/`book`/`instance`), per `tcb-allowed.md`.

## AST Consistency

- Exec source `identity_map.rs` is **byte-identical** to base `verus-ai-prove`
  (`git diff verus-ai-prove -- src/kernel/src/mm/virt/identity_map.rs` → empty).
- No exec code was changed; no `cfg`-gated exec divergence was introduced. The only `cfg`s in
  the module are the standard ghost includes (`#[cfg(verus_keep_ghost)] include!(...)`) and a
  pre-existing `#[cfg(feature = "test")]`, all unchanged from base.
- Changes were confined to the spec file (removed one dead `assume_specification` + comments) and
  comment-only edits in the proof file. Semantics, time complexity, and space complexity of exec
  code are preserved (unchanged).
- Zero mismatches confirmed: **YES**

## Result: PASS

All non-TCB cheating in `mm::virt::identity_map` is eliminated. Remaining cheating consists
solely of TCB-sanctioned `external_body` (3 functions + `ExPageTableBss`) listed in
`verus-ai-logs/tcb-allowed.md`, plus one required not-yet-verified-callee `assume_specification`
that the gate does not count. Zero `admit()`/`assume()`, zero proof gaps. Module verifies
`6 verified, 0 errors`; full `make verify` exits 0 with no regressions.
