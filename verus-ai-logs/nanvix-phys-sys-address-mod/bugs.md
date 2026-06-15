# Bugs — `sys::mm::address` (Address trait)

Specification phase for `is_aligned`, `into_raw_value`, `from_raw_value`.

## Code bugs

**None found.** The three in-scope `Address` trait methods are declarations
(no bodies); their specs verify cleanly. `make verify-sys` and `make verify`
both report `0 errors` with no `admit`/`external_body`/`assume` introduced by
this module (`sys::mm::address` cheating counts are all zero).

## Notable non-bug findings (for the proving phase)

### 1. Definition cycle when `spec_addr` is bounded by `Address` (resolved)

- **Symptom:** `error: found a cyclic self-reference in a definition` —
  `pub uninterp spec fn spec_addr<T: Address>` → referenced by the trait-method
  `ensures` clauses → which are part of the `Address` trait definition →
  back to the `T: Address` bound. Verus rejects the 3-node cycle
  (trait decl ⇄ method contract ⇄ `spec_addr`).
- **Why the kernel's identical `spec_addr<T: Address>` does *not* cycle:** the
  kernel projection lives in a *downstream* crate, so the `Address` trait it
  bounds is already fully defined and does not reference it back.
- **Resolution (spec design, not a code change):** declared the in-crate
  projections with an *unbounded* generic — `spec_addr<T>` and `addr_inv<T>` —
  in `mod.spec.rs`. Every use site instantiates `T = Self` inside an `Address`
  context, so no guarantee is lost. This is the only way to attach `int`-valued
  contracts to the trait's own method declarations without a `View` supertrait
  (which is itself impossible here; see the header note in `mod.spec.rs`).

### 2. Kernel `assume_specification[<VirtualAddress as Address>::into_raw_value]`

- The kernel pins `into_raw_value` via `assume_specification` in
  `hal/mem/types/address/phys.spec.rs` (ensures `result as int == addr@`).
- **No regression:** with the new trait-level `ensures result as int ==
  spec_addr(&self)` in place, `make verify` re-verified the kernel crate
  (`47 verified, 0 errors`). The two contracts coexist; the kernel
  `assume_specification` is the documented follow-on to be dropped when this
  module is fully wired downstream (see `frame.proof.rs`
  `lemma_phys_view_is_spec_addr`, "Removed when sys::mm::Address is verified").

## Environment notes (infrastructure, not project bugs)

- The pinned Verus is `build/verus-version = 0.2026.05.31.5dd6d83`, whose
  matching `vstd = 0.0.0-2026-05-31-0205` is what `Cargo.toml` requires.
  The pre-existing local install at `~/toolchain/verus` was a newer build
  (`0.2026.06.14`) that cannot compile the pinned `vstd`. Installing the
  correct pinned release via `./scripts/setup/verus.sh <dir>` and running with
  `VERUS_EXECUTABLE_DIR=<dir>` resolves it. All results above use the pinned
  `0.2026.05.31` Verus.
- With this Verus build, module paths passed to `make … MODULE=` must be
  crate-qualified as seen by Verus (`sys::mm::address`). The `scripts/verify.sh`
  helper strips one leading `<crate>::`, so module-scoped runs were invoked as
  `MODULE=sys::sys::mm::address` (the helper strips the first `sys::`, leaving
  `sys::mm::address`). Whole-crate `make verify-sys` needs no `MODULE`.
