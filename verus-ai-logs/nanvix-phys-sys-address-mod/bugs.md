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
  matching `vstd = 0.0.0-2026-05-31-0205` is what `Cargo.toml` requires (plain
  registry dependency, **no** `[patch.crates-io]`). This is the canonical setup
  produced by `./scripts/setup/verus.sh <dir>`.

### `make verify` gate failure (2026-06-15) — toolchain/`vstd` mismatch (RESOLVED)

- **Symptom:** `make verify` failed while compiling `vstd 0.0.0-2026-05-31-0205`
  with `error: expected generics to match … found u8` in `std_specs/atomic.rs`
  (`ExAtomic` newtype), then later `feature ‘…’ is declared but not used` errors
  in the `error` crate (`warnings = "deny"`).
- **Root cause:** the active toolchain at `~/toolchain/verus` had been clobbered
  to a newer build `0.2026.06.14.4ea7d0f` (backup left as
  `~/toolchain/verus-06.14-clobber-bak`), which does **not** match the project
  pin `0.2026.05.31`. A prior automated prover commit (`70a9ec9d0`) then changed
  `Cargo.toml` to `vstd = 0.0.0-2026-06-14-0213` + a `[patch.crates-io]`
  redirect to `/tmp/verus-crates` to chase the wrong toolchain, leaving an
  inconsistent state that fails to build for the `x86-kernel` target (which
  rebuilds `vstd` from source via `-Z build-std`).
- **Fix (config restore, no spec change):**
  1. Repointed `~/toolchain/verus` → `verus-pinned-0531`
     (`0.2026.05.31.5dd6d83`), the pinned release already cached locally.
  2. Reverted `Cargo.toml`/`Cargo.lock` to the matched config #1:
     `vstd = 0.0.0-2026-05-31-0205`, no `[patch.crates-io]`.
- **Result:** `make verify-sys` → `6 verified, 0 errors` (CLEAN). `make verify`
  → bitmap `70`, sys `6`, nanvix-slab `35`, kernel `47` verified, `0 errors`,
  exit `0`. The kernel `external_body=24` / `cfg_gate=6` counts are pre-existing
  TCB items, unrelated to this module.
- With this Verus build, module paths passed to `make … MODULE=` must be
  crate-qualified as seen by Verus (`sys::mm::address`). The `scripts/verify.sh`
  helper strips one leading `<crate>::`, so module-scoped runs were invoked as
  `MODULE=sys::sys::mm::address` (the helper strips the first `sys::`, leaving
  `sys::mm::address`). Whole-crate `make verify-sys` needs no `MODULE`.
