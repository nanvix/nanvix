## Turn 1: Full Checklist Walkthrough — Cheating Elimination (phys-upool)

Scope: `src/kernel/src/mm/phys/upool.rs` (+ `upool.spec.rs`, `upool.proof.rs`).
In-scope functions: `UserFrame::{new, address, leak, share, refcount}`,
`<UserFrame as Drop>::drop`, `Upool::{new, alloc}`.

### Progress
- Done (PASS/FIXED, all verified this turn):
  1. Zero admit() — PASS
  2. Zero assume() — PASS
  3. Zero trusted functions — PASS
  4. Zero exec_allows_no_decreases_clause — PASS
  5. Zero cfg-gated exec code — PASS
  6. Zero unlisted external_body — PASS
  7. AST consistency (zero mismatches) — PASS
  8. All exec rewrites have VERUS REWRITE comment — PASS (N/A, no rewrites)
  9. Each surviving external_body listed in tcb-allowed — PASS
  10. No specs weakened — PASS
  11. Cross-module regression — PASS
  12. Verification + build (0 errors, 0 warnings) — PASS
- Current: none — all items resolved.
- Remaining: none.

### Verification

Evidence gathered by reading the three source files end-to-end and running tools.

**1–4, 6 — per-function cheating patterns (admit/assume/trusted/no_decreases/external_body).**
`grep -rn` over `upool*.rs` for `admit`, `assume`, `external_body`, `trusted`,
`exec_allows_no_decreases_clause` → all empty (the single `trusted` hit was the
word "external construction" in a doc comment on `Upool._private`, not an
annotation). Independent confirmation via the detector:
`make verify-kernel MODULE=mm::phys::upool` →
`✅ No cheating detected in module mm::phys::upool`, `status: CLEAN`, exit 0.
The module-scoped detail file (`cheating-detail.txt`) lists 24 `external_body`
sites — every one in `frame.rs` / `manager.rs` / `mod.rs` (other modules), plus
one `external_type_spec` (`ExLinkedList`). **None in `upool`.** Notably,
`<UserFrame as Drop>::drop` verifies *directly* (no `external_body`) despite
calling `frame::free` and the `error!` macro — better than the tcb-allowed entry,
which permitted an `external_body` drop.

**5 — cfg-gated exec code.** The only `#[cfg(verus_keep_ghost)]` gates in
`upool.rs` (lines 9–12, 20–21) target `include!("upool.spec.rs")`,
`include!("upool.proof.rs")`, and `use ::vstd::prelude::*` — imports/includes
only, which the detector correctly excludes. Module-scoped `cfg_gate` count is 0
(global dropped 10→9 after the `View for UserFrame` impl was relocated into
`upool.spec.rs`). PASS.

**7 — AST consistency.**
`ast_consistency.py --base-ref verus-ai/phys-manager src/kernel/src/mm/phys/upool.rs count`
→ `✅ Consistent: 8 functions, 2 structs match` (exit 0). All exec code is
identical to the base branch; only ghost contracts were added.

**8 — exec rewrites.** `grep "VERUS REWRITE"` over `upool*.rs` → empty. Consistent
with the AST check showing zero exec divergence: no rewrites exist, so none need a
comment/reproducer. PASS (vacuously).

**9 — external_body ↔ tcb-allowed.** `upool` has zero `external_body`, so nothing
to list. The 24 cross-module `external_body` sites are each enumerated in
`verus-ai-logs/tcb-allowed.md` (frame.rs `alloc/free/share/refcount` shims and
`book/is_covered/alloc_range`; manager.rs `init/alloc_user_frame/`
`check_user_watermark/alloc_many_*/alloc_kernel_frame`; mod.rs `book_*`; kframe
`deref/deref_mut/clear`) or in its skip list (`frame::init`, `manager::get_mut`).
Out of scope for this module; PASS.

**10 — no specs weakened.**
`spec_drift.py git-diff --before verus-ai/phys-manager --after verus-ai/phys-upool`
→ Summary: **Ensures removed: 0**, Functions removed: 0, Requires added: 9,
Functions added: 2. No guarantee (ensures) was weakened or dropped. The 9 added
`requires` (`phys_view().initialized`, `phys_view().inv()`, `self.inv()` on
`alloc`/`share`/`refcount`) are the legitimate subsystem preconditions required to
call the do-not-modify `frame::*` shims, mirroring their contracts — not artificial
weakenings to dodge obligations. Reviewed and accepted. PASS.

**11 — cross-module regression.** `make verify` (all crates) → every crate exit 0;
kernel `20 verified, 0 errors`. CHEATING_DETECTED appears only for crates carrying
tcb-allowed shims / other-module cfg-gates; no verification failures anywhere. PASS.

**12 — verification + build.** `make verify-kernel` → exit 0, module `upool`
verifies (8 functions + `drop`). `./z build -- all-kernel` → exit 0. Forced
recompile of the kernel crate (`touch upool.rs` + cargo build) → no `warning`/
`error` lines, rc=0. 0 errors, 0 warnings. PASS.

### Fix Request
None. Every checklist item is PASS with concrete tool evidence. No code changes
requested.

### Result
ALL 12 checklist items PASS. Writing STOP = RESOLVED.
