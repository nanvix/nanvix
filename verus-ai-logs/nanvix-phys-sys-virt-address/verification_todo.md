# Verification TODOs: sys-virt-address

## Status

`make verify-sys` is **CLEAN** — zero cheating items
(`assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`).
The verification-target constructors `VirtualAddress::new` and
`VirtualAddress::from_raw_value` are body-verified with full `#[verus_spec]`
contracts (`result@ == value as int`, `result.inv()`). `VirtualAddress` carries
its `View`/`inv` material.

There are **no cheating items** (no `admit`/`assume`/`external_body`/
`assume_specification`) in the `sys` crate to eliminate. The single item below
is a *coverage* gap blocked by a genuine Verus front-end limitation — it is
**not** a proof gap and does **not** trip the cheating gate.

## Genuinely-blocked item

### `<VirtualAddress as Address>::into_raw_value` — cannot be body-verified in-place

- **Function:** `into_raw_value(self) -> usize { self.0 }`
  (trait method of `impl Address for VirtualAddress`,
  `src/libs/sys/src/sys/mm/address/virt.rs`).
- **Desired contract:** `result as int == self@` (pure newtype identity, the
  inverse of `new` / `from_raw_value`).
- **Blocking Verus error (reproduced in isolation):**

  ```
  error: Verus does not support this cast: `usize` to `*const u8`
  ```

  Verus checks a trait `impl` as a unit: to verify *any one* method of
  `impl Address for VirtualAddress`, the **entire** impl block is pulled into
  scope, including the sibling methods `as_ptr`/`as_mut_ptr`, whose bodies are
  exactly `self.0 as *const u8` / `self.0 as *mut u8`. That pointer cast is an
  unsupported Verus front-end construct (confirmed with a minimal reproducer:
  the same error fires both for a free function and for a trait-impl sibling).
- **Why it cannot be worked around within scope:**
  - Rust forbids splitting one trait `impl` into multiple `impl` blocks, so
    `into_raw_value` cannot be isolated from `as_ptr`/`as_mut_ptr`.
  - `as_ptr`/`as_mut_ptr` are **out of scope** (unlisted functions; must not be
    touched) and their cast *is* their purpose, so it cannot be rewritten.
  - `#[verifier::external]`/`external_body` on the siblings is forbidden by the
    verus-constraints skill (not a True Limitation escape, and a banned
    error-fix workaround).
- **Current handling (no cheating introduced):** `into_raw_value` is left
  unverified in the `sys` crate (no contract, no cheating marker). The newtype
  identity fact is carried by a consumer-side `assume_specification` in the
  `kernel` crate, which is explicitly governed by
  `verus-ai-logs/tcb-allowed.md` (entry
  `<VirtualAddress as Address>::into_raw_value`). That boundary is out of scope
  for `make verify-sys`.
- **Removal condition:** verifiable once the Verus front-end supports
  `usize as *const u8` casts (or once `as_ptr`/`as_mut_ptr` gain `vstd` pointer
  specs), at which point the whole `impl Address for VirtualAddress` block can
  be verified and the kernel-side `assume_specification` removed.
