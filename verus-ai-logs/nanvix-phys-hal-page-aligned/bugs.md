# Bugs / Verification Blockers: `PageAligned` (hal/mem/types/address/aligned/page.rs)

## VERUS-TOOL-1 — Verus panic verifying a generic trait impl whose trait method has `#[verus_spec]`

- **Type**: Verus tool limitation (not a Nanvix code bug). Recorded, not auto-fixed.
- **Symptom**: Marking `impl<T: Address> Address for PageAligned<T>` with
  `#[verus_verify]` makes Verus panic during compilation:

  ```
  thread 'rustc' panicked at vir/src/traits.rs:511:13:
  assertion failed: !method_impls.contains(&p)
  ...
  vir::traits::inherit_default_bodies
  ```

  The duplicated `(impl_path, method)` is `into_raw_value` — the one `Address` method
  that carries a `#[verus_spec]` on its **trait declaration** (`result as int == self@`,
  in `src/libs/sys/src/sys/mm/address/mod.rs`).

- **Isolation performed**:
  - Trivial vs. real `from_address` spec: irrelevant (panic is from the trait impl,
    not `from_address`, which lives in the separate inherent impl).
  - Removing the trait-decl `#[verus_spec]` on `into_raw_value`: panic persists when the
    generic trait impl is `#[verus_verify]`'d (duplicate appears for the generic impl).
  - **Non-generic control**: adding `#[verus_verify]` to `impl Address for PhysicalAddress`
    (non-generic) does **not** panic (it proceeds to an unrelated `Unsupported constant
    type` error). ⇒ the trigger is the **generic** trait impl, not the spec itself.

- **Impact**: `PageAligned::into_raw_value`'s impl body cannot be machine-verified in this
  Verus version. Its **contract is still provided** to callers via the trait declaration
  spec (trait-method ensures are inherited by all impls). The impl is left unverified
  (trusted) — deliberately **not** `external_body` (which is forbidden here and not in
  `tcb-allowed.md`). Body is the trivial projection `self.0.into_raw_value()`.

- **Discharge plan**: the proving phase (or an upgraded Verus that fixes the
  `inherit_default_bodies` duplicate-registration assertion) should re-add
  `#[verus_verify]` to the `Address for PageAligned<T>` impl and confirm
  `result as int == self@` proves from `view() == self.0@` plus the inner
  `T::into_raw_value` trait spec.

- **Status**: open (tool bug). No Nanvix source logic is wrong.

---

## Proving-phase update (2026-06-15)

### VERUS-TOOL-1 — re-confirmed (still open)

Reproduced the `vir/src/traits.rs:511` panic with Verus `0.2026.05.24.ecee80a`
(the pinned, newest-available binary). Both alternatives were tried and rejected:

- Annotating only the method (`#[verus_verify]` on `fn into_raw_value`) is
  rejected: *"In order to verify any items of this trait impl, the entire impl
  must be verified."*
- Annotating the whole `impl` block panics Verus (duplicate `TraitMethodImpl`
  registration for `into_raw_value`).

I also empirically confirmed the body is currently **unverified**: replacing it
with `{ let _ = self.0.into_raw_value(); 0 }` (which violates `result as int ==
self@`) still reports `2 verified, 0 errors`. The body is therefore trusted via
the *trait-declaration* contract only — the impl body is not machine-checked.

Outcome: left unchanged and unannotated (no `admit`/`assume`/`external_body`).
Full reproduction, isolation, and mitigation moved to
`verus-unsupported.md` in this directory. This remains a Verus tool bug; no
Nanvix source logic is wrong.

## Improvement — eliminated an out-of-TCB trust boundary (not a bug)

**What**: `page.spec.rs` previously modeled `::arch::mem::PAGE_ALIGNMENT` with an
`assume_specification` (ensures `spec_align_value(PAGE_ALIGNMENT) ==
spec_page_size()`). `PAGE_ALIGNMENT` is **not** in `tcb-allowed.md`, so this was
an unlisted trust axiom that `from_address` depended on.

**Fix**: `PAGE_ALIGNMENT` is the concrete constant `Alignment::Align4096`. Adding
`#[verus_verify]` to its definition in
`src/libs/arch/src/x86/mem/constants.rs` (matching the existing pattern on
`PAGE_SIZE` / `FRAME_SIZE`) lets Verus resolve the value directly, so
`spec_align_value(Align4096) == 4096 == PAGE_SIZE == spec_page_size()` is proved
rather than assumed. The `assume_specification` block was removed. `from_address`
still verifies (now with no trust axiom). This strengthens — never weakens — the
spec.

**Auto-Fixed**: Yes — added `#[verus_verify]` to the `PAGE_ALIGNMENT` constant
and deleted the `assume_specification` from `page.spec.rs`. Module verifies
`2 verified, 0 errors`; full `make verify` reports `0 errors` across all crates.
