## Turn 2: Verify turn-1 fix (watermark) → FIXED; new FAIL on Verification (normal build broken)

### Progress
- Done / PASS: fn-coverage, view-consistency, no-subsumed-ensures,
  no-assume_specification, vstd-searched, trait-obligations, loop-invariants,
  no-cheating-own-fns, no-specs-weakened, bug-awareness, cross-module-regression.
- Note (accepted precedent): no-tautological-ensures, error-paths-meaningful.
- **FIXED this turn:** Caller-coverage / Specs-usable-by-caller / Spec-completeness
  (watermark policy now caller-observable in the user contracts).
- **Current FAIL:** Verification — `make verify-kernel` passes (Exit 0) but the
  **normal build fails with 4 errors**. Dual compilation is broken.
- Remaining: re-verify build after fix.

---

### Verification of turn-1 fix (watermark) — CONFIRMED FIXED

I read the actual code (not the fixer's claim):

- `alloc_user_frame` `Ok` arm (manager.rs:259-265) now has **4** conjuncts:
  `allocated_frames.contains(frame@)`, `frame@ % spec_page_size() == 0`,
  `phys_view().frames.free_frames.finite()`, `spec_watermark_ok(phys_view().frames, 0)`.
- `alloc_many_user_frames` `Ok(())` arm (manager.rs:193-194) gained the same two conjuncts.
- `alloc_kernel_frame` (manager.rs:345-349) and `alloc_many_kernel_frames` (manager.rs:399-408)
  are **unchanged** — still no watermark conjunct. The user and kernel single-frame `ensures`
  blocks are no longer byte-identical. ✔ Policy split is now caller-observable.
- The added post-state axiom is sound: `check_user_watermark(count)` guarantees
  `free >= WATERMARK + count` pre-state; after handing out `count` frames,
  `free' = free - count >= WATERMARK`, i.e. `spec_watermark_ok(frames, 0)`. ✔
- `make verify-kernel` → **Exit 0**, all 5 modules verify; cheating counts unchanged
  (`external_body=22 admit=2`, no new items). ✔

This checklist item is **FIXED**.

---

### NEW FAIL — Verification: the normal (non-Verus) build does not compile

The fixer's turn-1 report claimed *"`make build` → Exit 0, 0 compiler errors (dual
compilation intact)."* **That claim is false.** I ran the canonical build myself:

```
$ ./z build -- all
error: cannot find attribute `verus_verify` in this scope
error: unused import: `::vstd::prelude::*`
error: unknown lint: `verus_impl_method_marker`
error: unknown lint: `verus_impl_method_marker`
error: could not compile `kernel` (bin "kernel") due to 4 previous errors
[ERROR] Build failed.   (exit 2)
```

The Verus build (`make verify-kernel`, which compiles with `--cfg verus_keep_ghost`) passes,
but the **normal** kernel build does not. Verified Nanvix code must compile **both** ways; a
green Verus run alone does not satisfy the Verification checklist item. Root cause of each error,
confirmed by reading the files:

1. **`cannot find attribute verus_verify`** — `kframe.rs:35` uses `#[verus_verify]`, but the
   prelude that provides that attribute macro is gated:
   ```
   kframe.rs:26  #[cfg(verus_keep_ghost)]
   kframe.rs:27  use ::vstd::prelude::*;
   ```
   In a normal build the `use` is cfg'd out, so the attribute is undefined. The established
   pattern in the codebase is an **unconditional** import — `page.rs:24` and
   `hal/.../address/frame.rs:16` both do `use ::vstd::prelude::*;` with no cfg, which makes
   `#[verus_verify]` a no-op in normal builds.

2. **`unused import: ::vstd::prelude::*`** — `manager.rs` imports the prelude **twice**:
   `manager.rs:8 use vstd::prelude::*;` and `manager.rs:39 use ::vstd::prelude::*;`. The second
   is a duplicate; with warnings-as-errors it breaks the build.

3 & 4. **`unknown lint: verus_impl_method_marker`** — `manager.rs:98` and `manager.rs:292` carry
   `#[allow(verus_impl_method_marker)]`. That lint name is registered only by the Verus tooling;
   in a normal build it is unknown and (warnings-as-errors) fatal. It must be gated to Verus
   builds. (Only manager.rs uses this lint anywhere in the kernel — grep confirms — so there is
   no other-file precedent to copy; the correct gate is `cfg_attr(verus_keep_ghost, ...)`.)

### Fix Request

Make the kernel compile under BOTH configurations. Apply all four edits:

1. **`src/kernel/src/mm/phys/kframe.rs`** — remove the cfg gate on the prelude import so it is
   unconditional (matching `page.rs`/`frame.rs`):
   ```
   -    #[cfg(verus_keep_ghost)]
   -    use ::vstd::prelude::*;
   +    use ::vstd::prelude::*;
   ```

2. **`src/kernel/src/mm/phys/manager.rs:39`** — delete the duplicate import (line 8 already does
   `use vstd::prelude::*;`):
   ```
   -    use ::vstd::prelude::*;
   ```

3. **`src/kernel/src/mm/phys/manager.rs:98`** — gate the Verus-only lint allow:
   ```
   -    #[allow(verus_impl_method_marker)]
   +    #[cfg_attr(verus_keep_ghost, allow(verus_impl_method_marker))]
   ```

4. **`src/kernel/src/mm/phys/manager.rs:292`** — same gating:
   ```
   -    #[allow(verus_impl_method_marker)]
   +    #[cfg_attr(verus_keep_ghost, allow(verus_impl_method_marker))]
   ```

**Verify the fix with BOTH commands (both must pass):**
- `./z build -- all` → must print no `error:` lines and exit 0.
- `make verify-kernel` → must remain Exit 0, all modules, no new cheating items.

Do not report success from one command alone. A verbal "it builds" is not acceptable — paste the
exit codes of both commands. Justification is not a fix.
