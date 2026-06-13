# Verus Unsupported Constructs — `virt` (`src/libs/sys/src/sys/mm/address/virt.rs`)

Genuine Verus front-end limitations encountered while trying to attach an
impl-level `#[verus_spec]` to `<VirtualAddress as Address>::into_raw_value`.
Per the **verus-constraints** skill these are recorded here and **not** worked
around (no exec rewrite, no `external_body`, no `assume_specification`).

Verus version: `0.2026.05.24.ecee80a` (toolchain 1.95.0).

## 1. `usize as *const u8` / `usize as *mut u8` casts

- **Functions:** `<VirtualAddress as Address>::as_ptr` (`virt.rs:268`) and
  `as_mut_ptr` (`virt.rs:272`).
- **Exact errors:**
  ```
  error: Verus does not support this cast: `usize` to `*const u8`
     --> src/libs/sys/src/sys/mm/address/virt.rs:268:9
      |
  268 |         self.0 as *const u8
      |         ^^^^^^^^^^^^^^^^^^^

  error: Verus does not support this cast: `usize` to `*mut u8`
     --> src/libs/sys/src/sys/mm/address/virt.rs:272:9
      |
  272 |         self.0 as *mut u8
      |         ^^^^^^^^^^^^^^^^^
  ```
- **Why it blocks `into_raw_value`:** Verus verifies trait impls **all-or-nothing**
  ("In order to verify any items of this trait impl, the entire impl must be
  verified"). Annotating the `impl Address for VirtualAddress` block to give
  `into_raw_value` a body-checked spec therefore forces Verus to translate
  `as_ptr`/`as_mut_ptr` too, which hit the unsupported cast above.
- **Only Verus-supported alternative:** `vstd::raw_ptr::with_exposed_provenance`,
  which takes an extra `Tracked<IsExposed>` argument — i.e. it requires changing
  the **signature/body** of the out-of-scope `as_ptr`/`as_mut_ptr` functions.
  That is an exec rewrite (forbidden) on unlisted functions (forbidden).

## 2. Trait methods delegating to non-Verus inherent methods

After the casts are bypassed, the same all-or-nothing rule surfaces a second
cascade: the trait `align_up`/`align_down`/`is_aligned` bodies call the inherent
`VirtualAddress::align_up`/`align_down`/`is_aligned`, which live in a plain
(non-`verus`) inherent `impl` block (they are out of scope per the task and the
reviewer's own instruction).

- **Exact errors** (observed after temporarily neutralizing the casts):
  ```
  error: cannot use function `sys::sys::mm::address::virt::VirtualAddress::align_down`
  which is ignored because it is either declared outside the verus! macro or it is
  marked as `external`.
     --> src/libs/sys/src/sys/mm/address/virt.rs:221:12
  error: cannot use function `sys::sys::mm::address::virt::VirtualAddress::is_aligned` ...
     --> src/libs/sys/src/sys/mm/address/virt.rs:239:12
  error: cannot use function `sys::sys::mm::address::virt::VirtualAddress::align_up` ...
     --> src/libs/sys/src/sys/mm/address/virt.rs:202:9
  ```
- Resolving these would require speccing the unlisted `align_up`/`align_down`/
  `is_aligned` (and transitively `mm::align_up`/`align_down`/`is_aligned` in
  `sys::mm::alignment`) — explicitly forbidden by the task ("Do not touch
  unlisted functions") and by the reviewer ("Do NOT add a spec to the
  alignment/`checked_*` methods in this phase").

## Resolution

`into_raw_value`'s contract is provided at the **trait declaration** in
`src/libs/sys/src/sys/mm/address/mod.rs:51-55`:

```rust
#[verus_spec(result =>
    ensures
        result as int == self@,
)]
fn into_raw_value(self) -> usize;
```

Every implementer (including `VirtualAddress`) is bound by this `ensures`, and
callers obtain `result as int == self@` whenever they call `into_raw_value`
through the `Address` trait. Combined with the inherent `from_raw_value` ensures
(`result@ == raw_addr as int`) the round-trip `from_raw_value(x).into_raw_value()
== x` is provable by callers. The impl block is therefore left unannotated so the
crate keeps verifying with `0 errors` and `make verify` stays green.
