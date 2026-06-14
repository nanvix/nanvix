# Final Independent Review — `mm::phys::kframe`

Reviewer: Claude (independent, from-scratch static analysis)
Date: 2026-06-15
Scope: in-scope functions only — `KernelFrame::new`, `KernelFrame::base`, `KernelFrame::drop`.
Out of scope (not flagged): `clear`, `deref`, `deref_mut`, and all sibling modules.
Method: grep/view over `kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`; static read of
`cheating-detail.txt`; cross-check of `tcb-allowed.md`, `caller_analysis.md`, `view_design.md`,
`bugs.md`. `make verify-kernel` was NOT re-run (per instructions; claim verified statically).

---

## Spec Quality

**`KernelFrame::new`** (`kframe.rs:81-110`, `external_body`)
- `requires base.inv()` — page-alignment precondition on the input frame address. Real, caller-met
  constraint (addresses come from `frame::alloc`/`alloc_contiguous`). ✅
- `ensures Ok(kf) => kf@ == base@ && kf.inv()` — preserves frame identity (the manager's
  `lemma_kernel_alloc_one` needs `kf@ == frame_addr@`) and re-exposes page-alignment for `base()`
  consumers. Non-tautological, written for the caller, stated on the mathematical `int` view. ✅
- `ensures Err(_) => true` — **weak/tautological arm.** The caller (`alloc_kernel_frame`) genuinely
  relies on `new` NOT consuming/freeing the frame on error (it frees `base` itself). That guarantee
  is allocator-view state, not expressible from the only in-scope precondition `base.inv()`, and is
  instead provided by the `external_body` trust boundary (the body never touches the global
  allocator on `Err`). Documented in `view_design.md` (Rejected Alt #6) and `tcb-allowed.md`.
  Acceptable given layering, but it is a real spec-completeness gap, not a strength. ⚠️ (non-blocking)

**`KernelFrame::base`** (`kframe.rs:128-137`)
- `requires self.inv()`, `ensures result@ == self@ && result.inv()`. Pure accessor (`&self`), exact
  identity + page-alignment carried to `kpage::into_page_address`. Complete and non-tautological. ✅

**`KernelFrame::drop`** (`kframe.rs:193-202`)
- `opens_invariants none`, `no_unwind`, no abstract postcondition. The "frees the frame exactly
  once" effect callers depend on is allocator-view state; the underlying `frame::free` is itself a
  best-effort `external_body` with `ensures true` (per `tcb-allowed.md`). So `drop` cannot assert
  more without the allocator token in scope. Consistent with the design; a known deferral. ⚠️ (non-blocking)

**`inv()`** (`kframe.spec.rs:11-13`): `self@ % spec_page_size() == 0`. Encodes a real, caller-visible
constraint (page-alignment), stated purely on the abstract `int` view via the shared
`spec_page_size()`; leaks no implementation detail. Mirrors `UserFrame::inv`. ✅

**`View`** (`kframe.rs:49-55`): `type V = int; closed view == self.base@`. Caller-abstract physical
address; passes the substitution test against every caller. ✅

Overall spec quality: **good**, with two documented, layering-justified deferrals (the `new` Err
no-consume guarantee and the `drop` free-once effect).

---

## Caller Coverage

Enumerating caller-expectations from `caller_analysis.md` that are spec-expressible in-scope:

| # | Caller expectation | Spec element | Status |
|---|---|---|---|
| 1 | `new`: `Ok(kf) => kf@ == base@` | `ensures kf@ == base@` | ✅ Covered |
| 2 | `new`: input page-aligned | `requires base.inv()` | ✅ Covered |
| 3 | `new`: `kf.inv()` for downstream | `ensures kf.inv()` | ✅ Covered |
| 4 | `base`: `result@ == self@` | `ensures result@ == self@` | ✅ Covered |
| 5 | `base`: page-aligned result | `ensures result.inv()` | ✅ Covered |
| 6 | `base`: pure / no mutation | `&self` accessor, no `&mut` | ✅ Covered |

**Covered: 6 / 6** spec-expressible expectations.

Two further caller assumptions are **covered-by-trust (deferred), not by an ensures**:
- `new`: `Err(_)` does not consume/free the frame — provided by `external_body` + caller's manual
  `frame::free(base)`. Not in an `ensures` arm (`Err(_) => true`).
- `drop`: frees the underlying frame exactly once — provided by `frame::free` (best-effort,
  `ensures true`), not by a `drop` postcondition.

**Missing (formal ensures):** none that are expressible in-scope. The two trust deferrals above are
documented allocator-token limitations, not omissions, and are tracked in `tcb-allowed.md` /
`view_design.md`.

---

## Proof Completeness

Grep over `kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`:

- **`admit()` count: 0.** No locations. (The string "assume" found at `kframe.spec.rs:8` is prose in
  a doc comment — "the `KernelStack` index arithmetic behind it **assume** the returned ..." — not a
  Verus `assume`.)
- **`external_body` count: 1.** Location: `kframe.rs:81` (attribute) on `KernelFrame::new`
  (fn declared at `kframe.rs:94`). `kframe.proof.rs` is empty (`verus! { }`).

No `admit()` anywhere → no proof BLOCKER from admits.

---

## TCB Compliance

**YES — compliant.**
- The single `external_body` (`KernelFrame::new`) IS listed in `tcb-allowed.md` (both the "Allowed
  `external_body`" section, lines 16-25, and the cross-module deferral section, line 102-105) with a
  full justification: its body calls `mm::virt::identity_map_page`, whose precondition
  `identity_map_view().inv()` is a global `mm::virt` ghost token not realized in `mm::phys`.
- The out-of-scope `deref`/`deref_mut`/`clear` external_bodies are also allowlisted but are not in
  review scope.
- No `external_body` in kframe is absent from the allowlist → **no TCB BLOCKER.**

---

## Guardrails (exact counts, kframe-local)

| Dimension | Count | Locations |
|---|---|---|
| `admit` | **0** | — |
| `assume` | **0** | (only a doc-comment word at `kframe.spec.rs:8`) |
| `external_body` | **1** | `kframe.rs:81` → `new` (allowlisted) |
| `assume_specification` | **0** | — |
| cfg-gated exec | **0 exec-gating** | `kframe.rs:199` gates a logging `error!` only (see AST) |

`admit == 0` and `assume == 0` → no guardrail BLOCKER. The lone `external_body` is in
`tcb-allowed.md`. The cfg gate does not gate executable logic.

---

## AST Consistency

**PASS.**

- **`// VERUS REWRITE` comments: none** in any kframe file. No rewrites to verify for semantic
  equivalence.
- **`drop` cfg gate (`kframe.rs:197-202`):**
  ```rust
  fn drop(&mut self) {
      if let Err(e) = super::frame::free(self.base) {
          #[cfg(not(verus_keep_ghost))]
          error!("failed to free kernel frame: {:?}", e);
      }
  }
  ```
  The executable effect — `super::frame::free(self.base)` and the `if let Err` branch — runs in
  **both** the Verus (`verus_keep_ghost`) and normal builds; it is NOT gated. The
  `#[cfg(not(verus_keep_ghost))]` attribute removes ONLY the `error!` logging statement under
  verification. This is **logging-only** and matches the identical, verified pattern in
  `UserFrame::drop` (`upool.rs:203-210`). It does **not** gate exec logic. **Not a blocker.**

---

## Verification

**PASS (confirmed statically, as instructed — `make` not re-run).**

- `cheating-detail.txt:21` lists exactly one kframe entry: `mm/phys/kframe.rs:94 new: external_body`.
  No kframe `admit`, no other kframe `external_body`.
- My independent grep of the three kframe files agrees precisely: 0 admit, 0 assume, 1 external_body
  (`new`), 0 assume_specification.
- The global mm::phys counts (admit=24, external_body=17) are dominated by sibling modules
  (`frame.rs`, `manager*.rs`, `mod.rs`, `phys.proof.rs`, HAL address layer) — confirmed by reading
  the full cheating-detail list; **none** of those admits/external_bodies are in `kframe`. kframe's
  own footprint is exactly one allowlisted `external_body`.
- The maintainer's `make verify-kernel MODULE=mm::phys => 0` and `./z build -- all-kernel => PASS`
  are consistent with this footprint and with the duplicate-import fix being in place (see Bugs).

---

## Bug Summary

**Total recorded: 1 actionable (auto-fixed) + 1 note (not a bug).**

1. **Duplicate `vstd::prelude::*` import** (`kframe.rs`) — *cosmetic / build-hygiene*, **auto-fixed**.
   - Verified fixed: `grep "vstd::prelude"` returns only the single top-of-file
     `use vstd::prelude::*;` (`kframe.rs:14`); the redundant `use ::vstd::prelude::*;` is gone.
   - Real code defect (broke `./z build -- all-kernel` under `-D warnings` via `unused import`),
     surfaced by the dual-compilation requirement. Entry has What/Why/Verification-Failure/How-Verus-
     Helped/Severity/Suggested-Fix. **Still valid as a historical record; defect is resolved.**
   - Severity: **cosmetic** (no runtime effect).

2. **Note (not a bug):** `KernelFrame::new` retains `external_body` after a strip attempt left it
   non-building/unverifiable (the `identity_map_page` global-token deferral). Correctly classified as
   a verification limitation, not a code defect; restored attribute + allowlist entry. ✅

**Unrecorded bugs discovered in this review: none.** No correctness/logic defect exists in the three
in-scope functions. (The `if let Err(e)` binding an unused `e` under `verus_keep_ghost` is benign and
matches the verified `UserFrame::drop`; not a defect.)

---

## Issues (highest priority first)

1. *(Non-blocking, spec completeness)* `new`'s `Err(_) => true` arm does not formally encode the
   "frame not consumed on failure" guarantee callers rely on. Deferred to the `external_body` trust
   boundary; eliminable only when `mm::virt`'s identity-map token is realized. Documented.
2. *(Non-blocking, spec completeness)* `drop` carries no abstract "freed exactly once" postcondition;
   bounded by `frame::free`'s best-effort `ensures true`. Allocator-token deferral; documented.
3. *(Informational)* `view_design.md`/`bugs.md` describe kframe as a "mirror of `UserFrame`": the
   `inv`/`drop` shapes do mirror, but `KernelFrame::view = int` while `UserFrame::view =
   FrameAllocView`, and `KernelFrame::new` is `external_body` while `UserFrame::new` verifies in-body
   (kframe additionally identity-maps). These are intended differences, not defects.

None of the above are blockers.

---

## Result

**PASS.**

Zero blockers:
- `admit() == 0`, `assume == 0`.
- The single `external_body` (`KernelFrame::new`) is present in `tcb-allowed.md`.
- No `assume_specification`; no exec-gating cfg (the `drop` cfg gate is logging-only).
- AST consistent (no `// VERUS REWRITE`; drop gate logging-only).
- Verification footprint confirmed: exactly `kframe.rs:94 new: external_body`, matching
  `cheating-detail.txt` and an independent grep; `make`/build reported green by the maintainer.
- Specs are caller-correct and use mathematical types; the two open items are documented,
  layering-justified trust deferrals, not omissions.
