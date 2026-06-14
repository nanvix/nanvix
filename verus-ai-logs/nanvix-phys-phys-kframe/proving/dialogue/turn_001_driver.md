## Turn 1: Full checklist walk-through — proving phase, `mm::phys::kframe`

Scope (per `verus-ai.toml` background): target functions are `KernelFrame::new`,
`KernelFrame::drop`, `KernelFrame::base`. Do-not-touch spec/view defs noted there.
All evidence below is from reading the source and running the verifier/build.

### Progress
- Done (PASS): all 10 checklist items (see verdicts below).
- Current: none remaining.
- Remaining: none.

### Verification (commands run)
- `make verify-kernel MODULE=mm::phys` → Exit 0. kframe module verifies.
- `make verify-kernel` (whole crate, cross-module) → Exit 0, all modules verify,
  0 errors. Log: `verus-ai-logs/verify-kernel/verus-logs/verus_2026-06-15_04-17-44.log`.
- `./z build -- ` (builds `all`, incl. `-p kernel`) → Exit 0; kernel cargo line
  `Finished dev profile`. Re-ran the kernel cargo invocation with grep for
  `warning|error` → no compiler warnings/errors. (Only output: a benign
  z-script `Sysroot directory ... not found; skipping symlink update` note —
  infrastructure, unrelated to kframe code.)
- Read `kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`,
  `verus-ai-logs/tcb-allowed.md`, `body_removed_source.rs` (frozen spec-phase
  snapshot), `scripts/verify.sh` (cfg-gate counter), and the proving commit diffs.

### Per-item verdicts

1. **No specs weakened (spec drift) — PASS.**
   Compared current contracts against the frozen spec-phase snapshot
   `body_removed_source.rs`. Identical, byte-for-byte on every contract:
   - `new`: `requires base.inv()`; `ensures Ok(kf) => kf@ == base@ && kf.inv()`.
   - `base`: `requires self.inv()`; `ensures result@ == self@ && result.inv()`.
   - `drop`: `opens_invariants none`, `no_unwind` (no abstract postcondition).
   - `inv` (spec.rs): `self@ % spec_page_size() == 0`; `View::view` unchanged.
   Git confirms the proving phase only stripped then re-added `external_body` on
   `new` (`9671457d6` → HEAD diff is the lone `#[verus_verify(external_body)]`
   line); no `requires` strengthened, no `ensures` weakened.

2. **Zero remaining `admit()` — PASS.** kframe contributes 0 admits.
   `kframe.proof.rs` is empty (`verus! { }`); no `admit` in `kframe.rs`.
   `cheating-detail.txt` lists no kframe admit (only frame/manager/identity_map,
   which are out of scope).

3. **Zero `external_body` unless in `tcb-allowed.md` — PASS.**
   Only kframe `external_body` is `kframe.rs:94 new` (cheating-detail line 21),
   and it is explicitly listed in `tcb-allowed.md` ("Allowed `external_body`" and
   the cross-module-dependency section) with the cross-module `identity_map_view()`
   global-token deferral rationale. `base` and `drop` are verified in-body (not in
   the cheating list; both carry contracts in coverage 40/45).

4. **Zero `assume`/`assume_specification` — PASS.** Whole-crate `assume=0`; kframe
   declares none.

5. **No cfg-gated exec code — PASS.** The only `#[cfg(not(verus_keep_ghost))]` in
   kframe is `kframe.rs:199`, gating an `error!(...)` log statement inside `drop`.
   The project's own counter (`scripts/verify.sh` `count_cfg_gates`, line 505)
   excludes logging macros (`error!`/`info!`/`warn!`/…), so kframe contributes 0
   to the crate `cfg_gate` total. This is the accepted logging-suppression pattern,
   not a cfg-gated branch/expression/match arm that alters verified control flow.

6. **Cheating audit (exact counts/locations) — PASS.** kframe-scoped:
   - `admit`: 0.
   - `external_body`: 1 → `kframe.rs:94 KernelFrame::new` (allowed, in TCB list).
   - `assume`/`assume_specification`: 0.
   - cfg-gated exec (per tool, logging excluded): 0 counted (the one cfg line at
     `kframe.rs:199` gates an `error!` log).
   All other crate-level counts (admit=24, external_body=17, cfg_gate=15) belong to
   out-of-scope modules (`frame`, `manager`, `mod`, `upool`, `virt::identity_map`).

7. **Claimed Verus limitation has isolated reproducer — PASS (N/A).** kframe makes
   no "Verus limitation" claim and has no `verus-unsupported` entry. Its sole trust
   boundary (`new` `external_body`) is a documented cross-module global-token
   deferral (callee `mm::virt::identity_map_page` requires `identity_map_view().inv()`,
   not realizable in `mm::phys`), recorded in `tcb-allowed.md`. `kframe.proof.rs`
   is empty — no lemmas, no admitted limitations.

8. **Exec rewrites minimal & semantically equivalent; `// VERUS REWRITE` — PASS.**
   `grep "VERUS REWRITE"` in `kframe.rs` → none. The proving diff
   (`9671457d6..HEAD`) shows zero body edits to `base`/`drop`/`new`; the only change
   is the `external_body` attribute toggle on `new`. No rewrites to review.

9. **Cross-module regression (`make verify-kernel`) — PASS.** Whole-crate run exit
   0; modules `mm::phys`, `mm::phys::frame`, `mm::phys::kframe`, `mm::phys::manager`,
   `mm::phys::upool` all verify with 0 errors. `CHEATING_DETECTED` status reflects
   in-progress proving of other modules, not a verification error.

10. **Verification + build, 0 errors / 0 warnings — PASS.** `make verify-kernel`
    exit 0, 0 errors, no `warning:` lines. `-p kernel` cargo build finishes clean
    with no compiler warnings/errors. (Lone z-script sysroot symlink note is benign
    infrastructure output, not a code warning.)

### Fix Request
None. Every checklist item is PASS with tool-backed evidence. The proving phase for
`mm::phys::kframe` is complete: `base` and `drop` verified in-body, `new` remains an
allowed (TCB-listed) cross-module `external_body`, specs are unchanged from the frozen
snapshot, and the full kernel verifies and builds cleanly.

Marking review RESOLVED.
