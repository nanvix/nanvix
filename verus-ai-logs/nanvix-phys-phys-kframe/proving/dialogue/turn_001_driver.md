## Turn 1: Full proving-phase checklist walkthrough — phys-kframe

### Progress
- Done (PASS/FIXED, all items verified with tool output):
  1. No specs weakened (spec drift) — PASS
  2. Zero remaining `admit()` — PASS
  3. Zero `external_body` unless TCB-allowed — PASS
  4. Zero `assume`/`assume_specification` (only external-bottom) — PASS
  5. No cfg-gated exec code — PASS
  6. Cheating audit (exact counts + locations) — PASS
  7. Claimed Verus limitation has isolated reproducer — PASS (n/a, no limitation claim)
  8. Exec rewrites minimal/semantically equiv (`// VERUS REWRITE`) — PASS (none)
  9. Cross-module regression (`make verify-kernel`, all modules) — PASS
  10. Verification + build: 0 errors / 0 warnings — PASS
- Current: none remaining
- Remaining: none

### Verification

Commands run from repo root (`/home/ruize/nanvix-phy`):

**`make verify-kernel`** → exit 0. Verified modules: `mm::phys`, `mm::phys::frame`,
`mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool`. Summary line:
`cheating: assume=0 external_body=28 admit=0 trusted=0 no_decreases=0 cfg_gate=9`,
`status: CHEATING_DETECTED` (crate-wide counts — scoped analysis below shows all
kframe-attributable items are TCB-allowed).

**`./z build -- all-kernel`** → `Finished dev profile ... [OK] Build complete.`
0 errors, 0 warnings.

---

**Item 1 — Spec drift (no guarantees weakened).** No spec-drift script exists in
the repo, so I diffed the spec-bearing files between the specification-phase END
commit (`5e6e5c2b3 [verus-ai] specification END: phys-kframe`) and HEAD
(`5d0a99a13`):
```
git diff 5e6e5c2b3 HEAD -- kframe.rs kframe.spec.rs kframe.proof.rs  → EMPTY
```
The proving phase produced **zero net change** to all three files. The
intermediate `strip-external-body` commit (`e890d1974`) removed
`#[verus_verify(external_body)]` from `clear`, but it was restored by HEAD (net
diff empty), i.e. the prover confirmed `clear` cannot be verified without the
trust boundary and reinstated it. Contracts (`new`: `frame@ == base@`; `base`:
`result@ == self@`; `Drop`: `phys_view().inv()` + `opens_invariants none` +
`no_unwind`) are byte-identical to the specification phase. No weakening. **PASS.**

**Item 2 — Zero `admit()`.** `cheating-detail.txt` reports `admit=0` crate-wide.
Direct grep of `kframe.rs`/`kframe.spec.rs`/`kframe.proof.rs`: no `admit`.
`kframe.proof.rs` is an empty `verus! { }` block. **PASS.**

**Item 3 — `external_body` only if TCB-allowed.** Cheating detail, kframe-relevant
entries:
- `mm/phys/kframe.rs:141 clear: external_body` — listed in `tcb-allowed.md` L25.
- `hal/mem/types/address/frame.rs:107 into_raw_value: external_body` — dependency,
  listed `tcb-allowed.md` L128.
- `mm/virt/identity_map.rs:649 identity_map_page: external_body` — dependency,
  listed `tcb-allowed.md` L133.

`clear` is the only `external_body` defined inside the kframe module itself; it
materializes a `*mut u8` from `usize` and `memset`s through the identity-map
backend — a raw-memory op Verus cannot model (no `PointsTo` for externally-owned
memory). Genuine TCB boundary, explicitly allowed. `deref`/`deref_mut` are
plain (non-`verus!`) impls, not in verification scope, also documented in TCB.
**PASS.**

**Item 4 — `assume`/`assume_specification`.** `assume=0` (no `assume(...)` calls).
`kframe.spec.rs:33` declares one `assume_specification` for
`<PageAligned<T> as Address>::from_raw_value`. `Address` is the **external
`sys::mm::Address` trait** (below this module's verification boundary) — an
external-bottom trust boundary, explicitly allowed in `tcb-allowed.md` L139–153.
A trait-impl method cannot take a standalone `external_body` contract without
verifying the whole `impl` block, so `assume_specification` is the correct
mechanism; it mirrors existing `sys`/`arch`-edge boundaries. **PASS.**

**Item 5 — No cfg-gated exec code.** kframe.rs `#[cfg(verus_keep_ghost)]` sites
(lines 15, 17, 32, 47) gate only: `include!("kframe.spec.rs")`,
`include!("kframe.proof.rs")`, `use ::vstd::prelude::*`, and the
`verus! { impl View for KernelFrame { closed spec fn view ... } }` block — i.e.
ghost/spec code. None gate an exec branch, expression, or match arm. The exec
fns (`new`, `base`, `clear`, `deref`, `deref_mut`, `drop`) contain no internal
cfg gates, so verified code == built code. (`verify.sh`'s `cfg_gate` heuristic
counts the line-47 `verus!`-block gate because it can't tell it wraps ghost code,
but the checklist rule targets cfg-gated *exec* code — none exists.) **PASS.**

**Item 6 — Cheating audit (kframe-scoped exact counts + locations).**
- `admit`: 0.
- `external_body`: 1 in-module — `kframe.rs:141 clear` (TCB-allowed). Plus 2
  cross-module dependencies (`into_raw_value`, `identity_map_page`, both
  TCB-allowed).
- `assume`: 0. `assume_specification`: 1 — `kframe.spec.rs:33 from_raw_value`
  (external `sys::mm::Address` trait, TCB-allowed).
- cfg-gated exec code: 0 (the one heuristic-counted cfg gate, `kframe.rs:47`,
  wraps the ghost `View` impl).
All items individually identified and each maps to an explicit TCB-allowed entry.
**PASS.**

**Item 7 — Claimed Verus limitation needs isolated reproducer.** kframe makes **no
"Verus front-end limitation" claim** (unlike `mod.rs`'s LinkedList case, which has
`verus-unsupported.md`). `clear`/`from_raw_value`/`identity_map_page` are genuine
*trust boundaries* (raw memory / not-yet-verified upstream modules), documented in
`tcb-allowed.md`, not front-end bugs requiring a reproducer. **PASS (n/a).**

**Item 8 — Exec rewrites minimal/semantically equivalent.** `grep "VERUS REWRITE"`
in kframe files → none. Spec-END→HEAD diff is empty → the proving phase performed
no exec rewrites. Nothing to review. **PASS.**

**Item 9 — Cross-module regression.** `make verify-kernel` verifies the whole
kernel crate (all 5 phys modules listed above), exit 0 — not just kframe. **PASS.**

**Item 10 — Verification + build: 0 errors / 0 warnings.** `make verify-kernel`
exit 0; `./z build -- all-kernel` finished clean, no warnings. **PASS.**

### Fix Request
None. Every checklist item is PASS with tool-backed evidence. The proving phase
introduced no net change to the kframe sources, verification passes crate-wide,
the build is clean, and every `external_body` / `assume_specification` /
ghost-cfg-gate is individually accounted for and explicitly TCB-allowed.

Marking the proving phase **RESOLVED**.
