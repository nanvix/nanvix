## Turn 1: Full Checklist Walkthrough — proving phase, phys-mod

Scope (from `verus-ai.toml`): proving target functions are **only** `init`,
`book_mmio_regions`, `book_physical_memory_regions` in
`src/kernel/src/mm/phys/mod.rs`. Hard rules: "Do not touch unlisted functions";
"Do not modify existing spec/view definitions" (`FrameAllocView`,
`FrameAllocView::wf`, `Inner::inv`, `frame_addr_of`, `View for Inner`,
`Inner::internal_inv`). Spec/proof live in `mod.spec.rs` / `mod.proof.rs`.

### Progress
- Done (PASS): all 10 checklist items (see below).
- Current: final verdict.
- Remaining: none.

### Verification

Commands run:
- `make verify-kernel MODULE=mm::phys` → exit 0; module result **9 verified, 0
  errors** (worktree byte-identical to committed PASS commit `3c3b2ce6b`
  "9 verified, 0 errors"; `git status -s src/kernel/src/mm/phys/` is empty).
- `make verify-kernel` (all modules) → exit 0 (cached, no recompilation).
- `./z build -- all-kernel` → "Compiling kernel v0.16.17 … Finished … Build
  complete." — **0 warnings, 0 errors**.
- Diffed `mod.spec.rs` / `mod.proof.rs` / `mod.rs` from proving-start commit
  `5f74ad00e` to worktree.
- Reproduced the claimed orphan-rule limitation in a standalone crate.

Item-by-item:

1. **No specs weakened — PASS.**
   `git diff 5f74ad00e -- mod.spec.rs`: purely **additive** — zero deletion
   lines. The only pre-existing spec (`FrameAllocView` + `FrameAllocView::wf`,
   lines 1–48) is byte-for-byte unchanged (diff hunk starts at line 49).
   `mod.proof.rs`: 271 insertions, 1 deletion — the single deletion is the empty
   scaffold `verus! { }` block replaced by real proof content (additive). The
   new `init` `ensures` matches caller expectations in `caller_analysis.md`
   (after `Ok`: `instance().inv()`/`wf`; reserved frames disjoint from free):
   `phys_view().inv()` on all paths and on `Ok` `initialized &&
   allocated_frames.disjoint(free_frames)`. No guarantee weakened.

2. **Zero admit() — PASS.** `grep -nE '\badmit\s*\('` on the three files →
   none. Tool: `cheating: admit=0` globally.

3. **Zero external_body unless in `tcb-allowed.md` — PASS (in scope).**
   In-scope external_body functions (per-function):
   - `mod.rs:82 book_physical_memory_regions` → **listed** in tcb-allowed.md.
   - `mod.rs:117 book_mmio_regions` → **listed** in tcb-allowed.md.
   `mod.spec.rs:73 ExLinkedList` is an `#[verifier::external_type_specification]`
   wrapper (classified by the scanner as `external_type_spec`, not a function
   `external_body`) — an allowed external trust boundary for the foreign std
   `LinkedList` type. The other 15 external_body the kernel-wide scan reports
   (`frame.rs` ×12, `manager.rs:86 init`, `upool.rs:148 new`) are in **other
   modules** (`mm::phys::frame` / `::manager` / `::upool`), outside the phys-mod
   proving target; the hard rule forbids touching unlisted functions, so they
   are not in scope here and are pre-existing TCB.

4. **Zero assume/assume_specification — PASS.**
   `grep -nE '\bassume\s*\(|assume_specification'` on the three files → none.
   Tool: `assume=0`. Only external boundary is the `ExLinkedList` external-type
   spec for foreign std `LinkedList` (allowed external-bottom).

5. **No cfg-gated exec code — PASS.** Ran the scanner's `count_cfg_gates` logic
   on `src/kernel/src/mm/phys/` → **0**. The three `#[cfg(verus_keep_ghost)]`
   in `mod.rs` gate `use` / `include!` (lines 36/40/42), explicitly excluded.
   Global `cfg_gate=5` are all **outside** phys-mod (hal/mem `verus! {}` blocks
   ×4 + `macros.rs` ×1) — spec-block markers, not exec branches/match-arms.

6. **Cheating audit (in-scope, with locations) — PASS.**
   - `admit`: 0.
   - `assume`/`assume_specification`: 0.
   - `external_body`: 2 — `mod.rs:82` & `mod.rs:117` (both tcb-allowed).
   - cfg-gated exec code: 0 in `mm::phys`.

7. **Claimed Verus limitation has isolated reproducer — PASS.** The limitation
   (orphan rule E0117 forbids `impl vstd::view::View for
   alloc::collections::linked_list::Iter` from the kernel crate) was reproduced
   in isolation: a minimal downstream crate with
   `impl Display for std::collections::linked_list::Iter<'_, T>` →
   `error[E0117]: only traits defined in the current crate can be implemented
   for types defined outside of the crate`. Confirms the foreign-trait-on-
   foreign-type construct is the real, specific blocker — not just "the for loop
   fails." Documented in `verus-unsupported.md`.

8. **Exec rewrites minimal & semantically equivalent — PASS.**
   `grep 'VERUS REWRITE'` → none. `git diff 5f74ad00e -- mod.rs` shows **no
   exec-body lines changed** (no `-`/`+` on `let`/`for`/`while`/`frame::*`/etc.);
   only `#[verus_spec]` contracts and explanatory comments were added. Exec
   bodies of `init`/`book_*` are unchanged → trivially semantically equivalent.

9. **Cross-module regression — PASS.** `make verify-kernel` (no MODULE) → exit 0;
   all kernel modules pass (cached, source unchanged since committed PASS).

10. **Verification + build — PASS.** `make verify-kernel MODULE=mm::phys`:
    9 verified, 0 errors. `./z build -- all-kernel`: 0 errors, 0 warnings.

### Fix Request
None. Every checklist item is PASS with concrete tool/diff/reproducer evidence,
within the phys-mod proving scope. The 15 out-of-scope `external_body` in
`frame.rs`/`manager.rs`/`upool.rs` belong to other module targets and are
governed by the "do not touch unlisted functions" hard rule — not actionable in
this phase.

Verdict: **RESOLVED.**
