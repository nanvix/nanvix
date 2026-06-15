# Verification TODOs: hal-page-aligned

## Proof gaps (admit/assume)

None. The module contains zero `admit()` and zero `assume()` in its source,
spec, and proof files. `make verify-kernel MODULE=hal::mem::types::address::aligned::page`
reports **11 verified, 0 errors**.

## Irreducible trust boundaries (not proof gaps)

These are `assume_specification` declarations for **genuinely external** entities.
They are the minimal trusted surface (spec-design: external-bottom / external-top)
and cannot be turned into in-body proofs without modifying out-of-scope crates or
unlisted functions. They are NOT counted by the cheating gate
(guardrails `has_cheating()` only counts assume()/external_body/admit/trusted/no_decreases).

- `page.spec.rs:7` — `assume_specification[ ::arch::mem::PAGE_ALIGNMENT ]`
  (external-bottom). `PAGE_ALIGNMENT` is a `pub const` in the **external `arch` crate**
  (`Alignment::Align4096`), declared outside any `verus!` block. Removing the spec
  yields a hard compile error:
  `error: cannot use function arch::x86::mem::constants::PAGE_ALIGNMENT which is
  ignored because it is either declared outside the verus! macro or marked external`
  (reproduced this turn). Eliminating it would require adding a verified Verus spec
  inside the `arch` crate — out of scope for this module.

- `page.spec.rs:32` — `assume_specification[ <PageAligned<T> as core::ops::Deref>::deref ]`
  (external-top). `core::ops::Deref` is a std/core trait with no Verus contract to
  inherit; Verus treats impls of external traits as external, so a trusted spec is the
  only mechanism. It is consumed by verified callers in the HAL frame layer / `mm::phys`
  (same "cannot use function ... external" class as PAGE_ALIGNMENT above). Verifying
  `deref` in-body would require `#[verus_verify]`/`#[verus_spec]` on the `deref` method,
  which is **not** in this module's allowed target set
  (`into_raw_value`, `from_address`, `PageAligned`) — forbidden by the "do not touch
  unlisted functions" hard rule.

## cfg-gate (false positive — gates ghost code)

- `page.rs:219` — `#[cfg(verus_keep_ghost)]` on the `verus! { ... pub open spec fn inv ... }`
  block. This gates **pure ghost/spec code**, not exec code. The gate is required because
  the block references `spec_page_size()`, which is itself a `spec fn` defined inside a
  `#[cfg(verus_keep_ghost)]` block (`hal/mem/types/address/frame.rs:43`) and therefore
  does not exist in non-ghost (normal `cargo build`) builds. This is the standard,
  pervasive Nanvix idiom for ghost material (cf. `frame.rs:36`, `alignment.rs:151`,
  `region.spec.rs`). Removing it would break the non-verus kernel build.
