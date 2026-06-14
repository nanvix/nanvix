## Turn 1: Full checklist walkthrough — phys-mod (`src/kernel/src/mm/phys/mod.rs`)

### Progress
- Done (PASS, with evidence below): all 12 checklist items.
- Current: full review of every checklist item.
- Remaining: none.

### Scope note
The review target is **phys-mod** = `mod.rs` + `mod.spec.rs` + `mod.proof.rs`.
Per `verus-ai.toml`, the in-scope functions are exactly `init`,
`book_mmio_regions`, `book_physical_memory_regions`. Sibling files
(`frame.rs`, `manager.rs`, `upool.rs`, `kframe.rs`) are **separate** review
targets ("Do not touch unlisted functions") and account for the global
`external_body=17` / `cfg_gate=5` counts that drive the crate-wide
`CHEATING_DETECTED` status. Those are out of scope here; this review evaluates
only the phys-mod files.

### Verification commands run
- `make verify-kernel MODULE=mm::phys` → exit 0; `assume=0 external_body=17
  admit=0 trusted=0 no_decreases=0 cfg_gate=5` (global), latest verus log
  `verification results:: 9 verified, 0 errors`.
- `make verify-kernel` (all) → exit 0, cached, 9 verified, 0 errors.
- `./z build -- check-kernel` → `build-finished success:true`,
  `0` `"level":"error"`, `0` `"level":"warning"`, `Build complete.`
- Read `mod.rs`, `mod.spec.rs`, `mod.proof.rs`, `verus-ai-logs/tcb-allowed.md`,
  `scripts/verify.sh::count_cfg_gates`, `cheating-detail.txt`.

### Item-by-item verdict

1. **Zero admit() remaining — PASS.**
   `grep -nE 'admit\('` over the three mod files → no matches. Global `admit=0`.

2. **Zero assume() remaining — PASS.**
   `grep -nE 'assume\('` over the three mod files → no matches. Global `assume=0`.

3. **Zero trusted functions — PASS.**
   No `#[trusted]` / `verus_verify(trusted)` in any mod file. Global `trusted=0`.

4. **Zero exec_allows_no_decreases_clause — PASS.**
   Global `no_decreases=0`; no exec loops in scope use a no-decreases escape.

5. **Zero cfg-gated exec code (only imports/derives/debug_assert/logging) — PASS.**
   Ran the `count_cfg_gates` logic from `scripts/verify.sh` restricted to the
   three mod files → **0**. The only `#[cfg(verus_keep_ghost)]` uses in `mod.rs`
   gate `use ::vstd::prelude::*;` (line 36) and `include!("mod.spec.rs")` /
   `include!("mod.proof.rs")` (lines 40, 42) — all imports/includes, which the
   detector excludes. `#[cfg(feature = "test")]` is not counted (detector only
   keys on `verus_keep_ghost`). The crate-wide `cfg_gate=5` is entirely in
   out-of-scope sibling files.

6 & 9. **External_body only if listed in `tcb-allowed.md` — PASS.**
   In-scope `external_body` functions:
   - `mod.rs:82 book_physical_memory_regions` → **listed** in `tcb-allowed.md`
     (std `LinkedList::iter()` ghost-iterator/orphan-rule limitation; abstract
     effect discharged by `lemma_book_region_reserves_region_frames`, no admit).
   - `mod.rs:117 book_mmio_regions` → **listed** in `tcb-allowed.md` (same
     LinkedList limitation; discharged by `lemma_book_mmio_skip_untracked` /
     `lemma_book_mmio_books_tracked`).
   - `mod.spec.rs:73 ExLinkedList` → `#[verifier::external_type_specification]`
     for the foreign `alloc::collections::LinkedList`. This is a **type
     declaration**, not a faked-body verified function; it is the standard,
     required Verus idiom to make the foreign type known and is explicitly
     justified by the LinkedList entries in `tcb-allowed.md` and in
     `mod.spec.rs` / `verus-unsupported.md`. Acceptable.
   No unlisted external_body in scope.

7. **AST consistency: zero mismatches — PASS.**
   No `ast-consistency` skill/script exists in this repo, so verified manually:
   AST mismatches arise from `#[cfg(not(verus_keep_ghost))]` exec divergence or
   non-equivalent exec rewrites. The mod files contain **no** `cfg(not(
   verus_keep_ghost))` exec code and **no** exec rewrites — the verus-gated cfgs
   are import/include only (item 5). Verified-vs-compiled ASTs are identical.

8. **All exec rewrites have VERUS REWRITE comment + minimal reproducer — PASS
   (vacuous).**
   `grep 'VERUS REWRITE'` → none, because there are **no** exec rewrites in
   `mod.rs`. The two helpers are `external_body` (documented in-file and in
   `tcb-allowed.md`), and the LinkedList limitation has its reproducer in
   `verus-ai-logs/nanvix-phys-phys-mod/verus-unsupported.md`.

10. **No specs weakened — PASS.**
    The spec functions named "do not modify" in `verus-ai.toml`
    (`byte_at_address`, `FrameAllocView`/`::wf`, `Inner::inv`, `frame_addr_of`,
    `View for Inner`, `Inner::internal_inv`) are intact; `git diff HEAD` on the
    three files is empty (committed state). `init`'s contract still delivers the
    real guarantees: `phys_view().inv()` on all paths and, on `Ok`,
    `initialized` **and** `allocated_frames.disjoint(free_frames)`. The `book_*`
    contracts preserve `inv()` and `initialized`. No guarantee was reduced to
    `true`/vacuity beyond the conventional terminal-`Err` arm. Proof obligations
    are discharged in `mod.proof.rs` without admit/assume.

11. **Cross-module regression — PASS.**
    `make verify-kernel` (whole crate) → exit 0; latest run
    `9 verified, 0 errors`. All verified `mm::phys` modules pass.

12. **Verification + build, 0 errors / 0 warnings — PASS.**
    `make verify-kernel MODULE=mm::phys` → exit 0, 9 verified, 0 errors.
    `./z build -- check-kernel` → `success:true`, 0 errors, 0 warnings,
    `Build complete.`

### Fix Request
None. Every checklist item is PASS with concrete tool/code evidence for the
phys-mod scope. The crate-wide `CHEATING_DETECTED` banner is driven solely by
out-of-scope sibling files and does not reflect a phys-mod defect.

### Result
RESOLVED — see `STOP`.
