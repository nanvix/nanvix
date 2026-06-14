## Turn 1: Full checklist walkthrough — phys-kframe cheating-elimination

### Progress
- Done (all PASS, with evidence below):
  1. Zero admit() — PASS
  2. Zero assume() — PASS
  3. Zero trusted functions — PASS
  4. Zero exec_allows_no_decreases_clause — PASS
  5. Zero cfg-gated exec code (imports/derives/logging only) — PASS
  6. Zero external_body unless TCB-allowed — PASS
  7. AST consistency — PASS
  8. All exec rewrites have VERUS REWRITE comment + reproducer — PASS (vacuous: 0 rewrites)
  9. Each surviving external_body listed in tcb-allowed.md — PASS
  10. No specs weakened (spec-drift) — PASS
  11. Cross-module regression (verify-kernel, all modules) — PASS
  12. Verification + build (0 errors, 0 warnings) — PASS
- Current: none remaining
- Remaining: none

### Verification

Scope: `src/kernel/src/mm/phys/kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`.
Branch: `verus-ai-prove` @ `90bdbda93`.

**Cheating-pattern scan (scoped to kframe files):**
```
admit            : 0  (only match is the prose word "assume" in spec.rs:8)
assume()         : 0
external_body    : 1  -> kframe.rs:81 KernelFrame::new
trusted          : 0  (only match is prose "is trusted" in comment)
no_decreases     : 0
cfg gates        : lines 15,17 (include! spec/proof — ghost), 45 (verus! View — ghost),
                   199 (#[cfg(not(verus_keep_ghost))] on error! log — logging-allowed)
```
Confirmed against `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`: the only
kframe entry is `mm/phys/kframe.rs:94 new: external_body`. The whole-crate totals
(external_body=17, admit=24, cfg_gate=15) belong to OTHER modules (frame, manager, mod,
upool, mm::virt, hal) — out of scope for phys-kframe.

**Item 5 (cfg-gated exec):** The only cfg-gated divergence in exec is the `error!` log line
in `KernelFrame::drop` (kframe.rs:199), gated by `#[cfg(not(verus_keep_ghost))]`. This is
logging (explicitly allowed) and matches the verified sibling `UserFrame::drop`. The other
cfg gates are ghost-only spec/proof `include!`s and the `verus!` `View` block — not exec.

**Item 6 / 9 (external_body):** `KernelFrame::new` (kframe.rs:94) is the single external_body.
It is explicitly listed in `verus-ai-logs/tcb-allowed.md` ("Allowed `external_body`" +
dedicated cross-module section). Rationale: body calls `crate::mm::virt::identity_map_page`
whose precondition is the global `identity_map_view().inv()` ghost token owned by `mm::virt`,
not realizable from `base.inv()` in `mm::phys`. Contract is non-trivial and sound:
`requires base.inv()`, `ensures Ok(kf) => kf@ == base@ && kf.inv()`. PASS.
Note: tcb-allowed.md also lists `deref`/`deref_mut`/`clear`, but in the current source these
are plain (non-verus) exec fns, not external_body — so no extra external_body exists.

**Item 7 (AST consistency):** `git diff verus-ai-prove -- kframe.rs kframe.spec.rs
kframe.proof.rs` is empty (HEAD == working tree). `body_removed_source.rs` confirms the exec
structure is unchanged; the only verify/non-verify difference is ghost includes + the logging
gate, which are semantically equivalent. Zero mismatches.

**Item 8 (rewrites):** `grep "VERUS REWRITE"` in kframe files → none. No exec rewrites exist,
so the requirement is vacuously satisfied.

**Item 10 (spec drift):** Current specs are genuine guarantees, not weakened:
- `inv`: `self@ % spec_page_size() == 0` (page alignment)
- `new`: `Ok(kf) => kf@ == base@ && kf.inv()`
- `base`: `result@ == self@ && result.inv()`
- `drop`: `opens_invariants none`, `no_unwind`
The only `true`/empty postconditions are `new`'s `Err(_)` arm (cross-module deferral) and
`drop` (best-effort dealloc) — both documented and matching the speccing baseline
(`body_removed_source.rs` specs are byte-identical to current). Diff vs `verus-ai-prove`
empty → no drift.

**Item 11 (cross-module regression):** `make verify-kernel` → exit 0. Modules verified:
mm::phys, mm::phys::frame, mm::phys::kframe, mm::phys::manager, mm::phys::upool. No
verification errors. (The `CHEATING_DETECTED` status is the whole-crate cheating gate firing
on out-of-scope modules, not a verification failure.)

**Item 12 (verification + build):**
- `make verify-kernel` → exit 0, 0 verification errors.
- `make check-kernel` → exit 0, `build-finished success:true`.
- Strict warning grep (`compiler-message|warning:`) over a forced kernel recompile → empty.
  0 warnings.

### Fix Request
None. All checklist items PASS with concrete tool evidence. No code changes required.
Proceeding to STOP = RESOLVED.
