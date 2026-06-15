# Cheating Elimination Report: hal-page-aligned

Module: `hal::mem::types::address::aligned::page`
Files: `page.rs`, `page.spec.rs`, `page.proof.rs`
Verification: `make verify-kernel MODULE=hal::mem::types::address::aligned::page`
→ **11 verified, 0 errors** (exit 0).

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 2 | 2 | 0 |
| cfg-gated exec | 1 | 1 | 0 |

Notes:
- The gate-counted cheating categories (`admit`, `assume`, `external_body`,
  `trusted`, `no_decreases`) are **0** in this module's own source/spec/proof —
  confirmed by the module-scoped cheating scan (no `aligned/page` entries appear in
  `verus-logs/cheating-detail.txt`; all global `external_body=19 admit=16` come from
  out-of-scope modules: `mm/phys/*`, `mm/virt/*`, other `hal` address files).
- The 2 `assume_specification` and 1 `cfg-gate` are **not** counted by the cheating
  gate (`guardrails.has_cheating()`); they are legitimate, irreducible artifacts
  (see below).

## Items Eliminated
None were eliminable: the module entered this phase with **zero** gate-counted
cheating. Each remaining non-gate item was investigated via the verus-constraints
escalation ladder and proven required:

- **`assume_specification[ ::arch::mem::PAGE_ALIGNMENT ]`** (external-bottom).
  Escalation: (1) vstd — N/A (arch-crate const, not a vstd item); (2) isolated
  removal reproducer — removed the declaration and re-ran the module verify, which
  failed with
  `error: cannot use function arch::x86::mem::constants::PAGE_ALIGNMENT which is
  ignored because it is either declared outside the verus! macro or marked external`;
  (3) equivalent rewrite — the only rewrite is adding a verified spec inside the
  external `arch` crate (out of scope). **Conclusion: required external trust
  boundary; restored.**

- **`assume_specification[ <PageAligned<T> as core::ops::Deref>::deref ]`**
  (external-top). `core::ops::Deref` is a std/core trait with no Verus contract;
  Verus treats external-trait impls as external, so a trusted spec is the only
  mechanism. Consumed by verified callers in the HAL frame layer / `mm::phys`
  (identical "cannot use function ... external" class as PAGE_ALIGNMENT). Verifying
  `deref` in-body would require annotating the `deref` method, which is **not** in
  the module's allowed target set (`into_raw_value`, `from_address`, `PageAligned`)
  and is barred by the "do not touch unlisted functions" hard rule.
  **Conclusion: required external trust boundary; kept.**

- **`#[cfg(verus_keep_ghost)]` at `page.rs:219`** (gates ghost code, not exec).
  The block is `verus! { ... pub open spec fn inv ... }` and references
  `spec_page_size()`, itself a `spec fn` gated behind `#[cfg(verus_keep_ghost)]`
  in `frame.rs:43`. In a normal (non-ghost) build that symbol does not exist, so the
  gate is mandatory for compilation. Standard Nanvix idiom (cf. `frame.rs:36`,
  `alignment.rs:151`, `region.spec.rs`). **Conclusion: required ghost-code gate; kept.**

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-page-aligned/verification_todo.md)
- No proof gaps (zero `admit()`/`assume()`).
- Recorded the 2 `assume_specification` external trust boundaries and the 1
  ghost-code `cfg`-gate as irreducible, with the reproduced Verus error evidence.

## AST Consistency
- Zero mismatches confirmed: **YES**.
- Exec source `page.rs`, plus `page.spec.rs` and `page.proof.rs`, are byte-identical
  to the verified phase-start baseline (`git diff 24143f263 HEAD` is empty for all
  three files). No exec code was changed, no cfg gates added, no semantic / time /
  space complexity change. (Two transient experimental edits to `page.spec.rs` —
  removing each `assume_specification` to gather the "required" evidence above — were
  fully reverted; the final committed content equals the baseline.)

## Result: PASS
Rationale: the module's gate-counted cheating (admit/assume/external_body) is 0 and
the module verifies with 0 errors. The remaining `assume_specification` (×2) and
`cfg`-gate (×1) are legitimate external trust boundaries / ghost-code gating that are
not soundness escapes, are not counted by the cheating gate, and cannot be removed
without modifying out-of-scope crates/functions or breaking the build.
