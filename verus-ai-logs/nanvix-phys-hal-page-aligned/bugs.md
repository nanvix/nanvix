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
