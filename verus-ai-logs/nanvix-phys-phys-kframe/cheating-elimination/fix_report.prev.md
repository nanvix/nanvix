# Cheating Elimination Report: phys-kframe

Scope (verification-order target functions): `KernelFrame::new`, `KernelFrame::drop`,
`KernelFrame::base`.
Files: `kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`.

## Cheating Counts (before → after)

In-scope (kframe module) counts:

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 1* | 1* | 0 (TCB-allowed) |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 1** | 1** | 0 (pre-approved logging gate) |

\* `KernelFrame::new` — explicitly listed in `verus-ai-logs/tcb-allowed.md`
("Allowed `external_body`" and the cross-module dependency section). Not a blocker.

\** One `#[cfg(not(verus_keep_ghost))]` guarding the `error!("failed to free kernel
frame…")` log line in `KernelFrame::drop`. Logging macros use a constant type Verus
cannot model; identical, established convention as the verified sibling `UserFrame::drop`
(`upool.rs:205`). Not exec logic; semantics / time / space preserved.

(The whole-crate harness reports `external_body=17 admit=24 cfg_gate=15`, but those belong
to OTHER `mm::phys` modules — `frame`, `manager`, `mod`, `upool` — and to `mm::virt`,
`hal`, etc. They are out of scope for phys-kframe. The only kframe entry in
`cheating-detail.txt` is `mm/phys/kframe.rs:94 new: external_body`, which is TCB-allowed.)

## Items Eliminated

None required elimination. Detailed disposition of every cheating-class item touching the
kframe module:

- **`KernelFrame::new` — `external_body` (kframe.rs:94):** KEPT. Explicitly authorized in
  `verus-ai-logs/tcb-allowed.md`. Body calls `crate::mm::virt::identity_map_page`, whose
  precondition is the global `identity_map_view().inv()` ghost token owned by `mm::virt` and
  not realizable in `mm::phys`; therefore the callee `requires` cannot be discharged in-scope
  from `base.inv()`. Same cross-module global-token deferral as `frame::book`/`frame::alloc`.
  Removing it is impossible without realizing the `mm::virt` token (out of this module's
  scope) and would only be replaced by an `admit()`/`assume()` — a strictly worse cheat. The
  contract is non-trivial and sound: `requires base.inv()`,
  `ensures Ok(kf) => kf@ == base@ && kf.inv()`.

- **`KernelFrame::drop` — `#[cfg(not(verus_keep_ghost))]` on the `error!` log
  (kframe.rs:199):** KEPT. Pre-approved logging-macro gate (the same class accepted in the
  `phys-upool` and `phys-manager` cheating-elimination reports). The gate only removes a
  best-effort log line under the ghost build; the deallocation (`super::frame::free`) and all
  Rust-visible behavior are identical across builds. `drop` itself verifies in-body
  (`opens_invariants none`, `no_unwind`).

- **`KernelFrame::base`:** verifies fully in-body — no cheating.

- **`kframe.spec.rs` / `kframe.proof.rs`:** no `admit`/`assume`/`external_body`;
  `kframe.proof.rs` is empty (`verus! { }`).

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-kframe/verification_todo.md)

- No genuine proof gaps. The only recorded item is the TCB-allowed cross-module trusted
  boundary `KernelFrame::new` (`external_body`), which is eliminated only when `mm::virt`'s
  identity-map ghost token is realized — outside `mm::phys`'s scope. No `admit()`/`assume()`
  remain anywhere in the module.

## AST Consistency

- `git diff verus-ai-prove -- kframe.rs kframe.spec.rs kframe.proof.rs` → **empty**. The
  in-scope source is byte-identical to the base branch; no exec code was changed, no cfg
  gate was added, no `external_body` was introduced. Semantics, time complexity, and space
  complexity are trivially preserved.
- Zero mismatches confirmed: **YES**

## Verification

- `make verify-kernel MODULE=mm::phys` → exit 0, no verification errors (kframe module
  verifies; only whole-crate cheating gate fires, from out-of-scope modules).
- `make verify` (full crate) → exit 0, no verification errors. No regressions (zero source
  changes were made).

## Result: PASS

All cheating attributable to the phys-kframe target functions is either non-existent
(`admit`/`assume`/`assume_specification` = 0; `base`/`drop` verify in-body) or explicitly
TCB-authorized (`KernelFrame::new` `external_body`, listed in `tcb-allowed.md`). The single
`drop` logging cfg-gate is a pre-approved deviation matching the verified `UserFrame::drop`.
Zero eliminable cheating remains in scope.
