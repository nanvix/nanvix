# Cheating Elimination Report: phys-manager

Module: `src/kernel/src/mm/phys/manager.rs` (+ `manager.spec.rs`, `manager.proof.rs`)
Scope: target methods `init`, `alloc_user_frame`, `check_user_watermark`,
`alloc_many_user_frames`, `alloc_many_kernel_frames`, `alloc_kernel_frame`.

## Cheating Counts (before → after)

Counts below are for the in-scope `manager.*` files. The `external_body` items
are all whitelisted in `verus-ai-logs/tcb-allowed.md` (the task's sole exception).

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 6 | 6 (all whitelisted) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

No **disallowed** cheating exists: 0 admit, 0 assume, 0 assume_specification,
0 cfg-gated exec code, and every `external_body` is on a function explicitly
listed in `tcb-allowed.md`. The two `#[cfg(verus_keep_ghost)]` lines in
`manager.rs` (9, 11) only guard `include!("manager.spec.rs")` /
`include!("manager.proof.rs")` — the standard split-file layout, not cfg-gating
of exec code.

## Items Eliminated

None required elimination. All 6 `external_body` items are on whitelisted
functions and 0 admit/assume/assume_specification/exec-cfg-gate were present.

The 6 `external_body` functions were each evaluated against the
**verus-constraints escalation ladder** and confirmed to be genuine,
sanctioned trust boundaries (not removable without touching unlisted functions):

1. **Searched vstd / upstream specs.** The methods' `ensures` (e.g.
   `phys_view().frames.allocated_frames.contains(frame@)`,
   `spec_watermark_ok(...)`, `kernel_frames_contiguous(...)`) can only be
   produced by their callees — `Upool::alloc`, `frame::alloc`,
   `frame::alloc_contiguous`, `frame::free`, `frame::free_count`,
   `KernelFrame::new`. These callees are **unlisted** (forbidden to modify) and
   carry **no `ensures`** (confirmed: they appear in the verifier's "Unverified
   functions" list). So no postcondition fact flows into the method bodies.

2. **Isolated reproducer.** Removed `external_body` from `check_user_watermark`
   and ran `make verify-kernel MODULE=mm::phys`. Result:
   `error: Unsupported constant type` originating from the `error!` → `write!`
   macro expansion (`crate::klog::KlogLevel::Error`), `0 verified`,
   "verus did not run" (exit 101). The `error!`/`warn!` logging macros are not
   ghost-gated and have no vstd specs. Restored afterward.

3. **Equivalent rewrites considered.** Even with the `error!`/`warn!` macros
   cfg-gated and `ok_or_else`/`inspect_err` closures rewritten (ast-consistency
   pre-approved deviations), the postconditions remain undischarge-able:
   `config::kernel::KERNEL_WATERMARK` is not linked to the `uninterp
   spec_kernel_watermark()`, and `frame::free_count()` (unlisted, unspecced) is
   not linked to `phys_view().frames.free_frames.len()`. Linking these requires
   editing unlisted functions or adding `assume_specification`/`axiom`
   (human-approved-only) — both forbidden.

Conclusion: `external_body` is the correct trust boundary for all 6, exactly as
documented in `tcb-allowed.md` under "Allowed `external_body` —
`PhysMemoryManager`". The abstract laws backing the caller-facing guarantees are
discharged with **real proofs** (no admit/assume) in `manager.proof.rs`
(`lemma_watermark_monotone`, `lemma_contiguous_run_distinct`).

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-manager/verification_todo.md)

- No proof gaps (no admit/assume/assume_specification anywhere in scope).
- The 6 trust-boundary `external_body` items are recorded with their specific
  blocking construct / missing upstream spec, plus the out-of-scope work that
  would be needed to eliminate them (speccing the unlisted upstream primitives
  and linking `KERNEL_WATERMARK` to `spec_kernel_watermark`).

## AST Consistency

- Tool: `scripts/ast_consistency.py --base-ref verus-ai/phys-mod manager.rs summary`.
- Result: **matched=7 mismatched=0 missing=0 extra=0** — Consistent ✅ YES.
- All exec functions (`init`, `alloc_user_frame`, `check_user_watermark`,
  `alloc_many_user_frames`, `alloc_many_kernel_frames`, `alloc_kernel_frame`,
  `get_mut`) and the `PhysMemoryManager` struct MATCH the base branch.
- Zero mismatches confirmed: YES. (No exec-code changes were made; only the
  in-progress `check_user_watermark` `external_body` annotation — required for
  the module to compile under Verus and whitelisted — was retained.)

## Verification Results

- `make verify-kernel MODULE=mm::phys`: **11 verified, 0 errors** (exit 0).
- `make verify` (full crate): **exit 0**, no regressions. Global cheating counts
  unchanged (`external_body=22 cfg_gate=9`) — all pre-existing and outside this
  module's scope (frame.rs, mod.rs, upool.rs, plus whitelisted manager items).

## Result: PASS

All in-scope cheating is either absent (admit/assume/assume_specification/
exec-cfg-gate = 0) or a whitelisted trust boundary justified in
`tcb-allowed.md` and empirically confirmed unremovable under the task's hard
rules. Verification passes with 0 errors at both module and full-crate scope;
AST consistency is clean.
