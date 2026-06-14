## Response to Turn 2

### What I Changed

Fixed dual compilation so the kernel builds under BOTH the normal (`./z build`) and the
Verus (`--cfg verus_keep_ghost`) configurations. Applied all four requested edits.

**File:** `src/kernel/src/mm/phys/kframe.rs`
1. Removed the `#[cfg(verus_keep_ghost)]` gate on the `vstd` prelude import so it is
   unconditional (matching the `page.rs` / `hal/.../address/frame.rs` precedent). This makes
   `#[verus_verify]` resolve as a no-op attribute in normal builds instead of "cannot find
   attribute":
   ```
   -use ::sys::error::Error;
   -#[cfg(verus_keep_ghost)]
   -use ::vstd::prelude::*;
   +use ::sys::error::Error;
   +use ::vstd::prelude::*;
   ```

**File:** `src/kernel/src/mm/phys/manager.rs`
2. Deleted the duplicate `use ::vstd::prelude::*;` (line 39); line 8 already does
   `use vstd::prelude::*;`. Removes the `unused import` (warnings-as-errors) failure:
   ```
   use ::sys::error::{
       Error,
       ErrorCode,
   };
   -use ::vstd::prelude::*;
   ```

3 & 4. Gated both `#[allow(verus_impl_method_marker)]` lint-allows (on `init` and
   `check_user_watermark`) behind `cfg_attr(verus_keep_ghost, ...)`, since that lint name is
   registered only by the Verus tooling and is an `unknown lint` (fatal under
   warnings-as-errors) in a normal build:
   ```
   -    #[allow(verus_impl_method_marker)]
   +    #[cfg_attr(verus_keep_ghost, allow(verus_impl_method_marker))]
   ```
   (applied at both `PhysMemoryManager::init` and `PhysMemoryManager::check_user_watermark`)

No spec content changed — these are build-configuration-only edits. The turn-1 watermark
contracts are untouched.

### Verification

Both commands run, both pass — exit codes pasted below.

**`./z build -- all`** (normal, non-Verus build) → **EXIT 0**, no `error:` lines:
```
    Finished `dev` profile [optimized + debuginfo] target(s) in 11.06s
    ... (all crates Finished)
Warning: Sysroot directory '/home/ruize/nanvix-phy/sysroot-debug' not found; skipping symlink update.
EXIT=0
```
(The 4 previous errors — `cannot find attribute verus_verify`, `unused import`, two
`unknown lint: verus_impl_method_marker` — are all gone. The remaining `Warning:` is a
pre-existing unrelated sysroot-symlink notice, not a compile error.)

**`make verify-kernel MODULE=mm::phys`** → **EXIT 0**:
```
verification results:: 11 verified, 0 errors (partial verification with `--verify-*`)
  verification: 11 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=22 admit=2 trusted=0 no_decreases=0 cfg_gate=9
  status: CHEATING_DETECTED
```
No new cheating items: `external_body=22 admit=2` unchanged vs. the prior turn. `cfg_gate`
rose 7 → 9, reflecting exactly the two new `cfg_attr(verus_keep_ghost, ...)` lint gates
introduced by this fix (a benign configuration gate, not an unsoundness primitive).
`CHEATING_DETECTED` continues to reflect only the pre-existing, tcb-allowed shims and the
spec-phase `admit()` lemmas.

### Result: FIXED
