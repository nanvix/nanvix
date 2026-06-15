## Turn 2: Item 8 — exec rewrite VERUS REWRITE comment + minimal reproducer

### Progress
- Done (PASS/FIXED): 1 admit, 2 assume, 3 trusted, 4 no_decreases, 5 cfg-gated
  exec, 6 external_body allow-list, 7 AST consistency, 9 external_body re-confirm,
  10 spec drift, 11 cross-module regression (re-confirmed exit 0 this turn),
  12 verification + build (re-confirmed: module verify exit 0 / 31 verified 0
  errors, `./z build -- all-kernel` 0 warnings this turn). Item-8 **comment**
  sub-requirement: FIXED.
- Current: **8 — minimal reproducer → still FAIL (the reproducer does not reproduce).**
- Remaining: none unchecked.

### Verification

**Comment (PASS).** `grep -n "VERUS REWRITE" frame.rs` → line 845. The
`VERUS DEVIATION` text is gone. The new comment states the original idiom, the
semantic/cost equivalence, the Verus limitation, and cites the reproducer path.
`git diff verus-ai/phys-kframe -- frame.rs` confirms the `free_count` exec
statements are unchanged — comment-only edit. Good.

**Verification/build re-confirmed (PASS).** `make verify-kernel MODULE=mm::phys`
→ Exit 0 (log `verus_2026-06-15_10-08-09`: `31 verified, 0 errors`).
`./z build -- all-kernel` → `Finished ... Build complete`, 0 warnings.

**Minimal reproducer (FAIL).** I ran the shipped reproducer through the real
verifier — `/mnt/toolchain/verus/verus
verus-ai-logs/nanvix-phys-phys-frame/reproducers/04_free_count_inline_fails.rs`:

1. Its **PASS form does not verify**:
   ```
   error: postcondition not satisfied
     --> 04_free_count_inline_fails.rs:64:13
   64 |             result > 0,   <-- unprovable from body `self.n`
   verification results:: 3 verified, 1 errors
   ```
2. Its **FAIL form does not fail**. I extracted the exact inlined form into a
   standalone file and ran it — `4 verified, 0 errors`. It verifies cleanly; it
   does **not** trigger the `num_bits >= 0` precondition error it claims to.

Both self-documented claims in the file ("PASS form ... verifies", "FAIL form
... error: precondition not satisfied") are **false** when actually run through
Verus. A reproducer that does not reproduce is not a reproducer.

**Root cause (and the faithful fix).** The shipped model puts `Bitmap` (with its
`closed spec fn view`) in the **same module** as `free_count`. Closed-spec bodies
are visible *within their defining module*, so `num_bits == self.n as int >= 0`
leaks and the lemma precondition is discharged even when inlined — masking the
failure. The real failure depends on the **cross-crate boundary**: the real
`::bitmap::Bitmap` view is closed in a *different* crate, so `inner.bitmap@.num_bits`
is opaque in `frame.rs`. I reproduced the real failure faithfully by moving
`Bitmap` into a separate module so the closed view is hidden:

```
=== cross-module model: split verifies, inline fails ===
error: precondition not satisfied
   18 | pub proof fn lemma_free_count(b: &Bitmap) requires b.inv(), b@.num_bits >= 0 {}
      |                                                             ---------------- failed precondition
   26 |     proof! { lemma_free_count(b); }   // inline form
verification results:: 4 verified, 1 errors
```

This matches the real-tree error exactly (`num_bits >= 0` failed precondition).

### Fix Request

Replace `verus-ai-logs/nanvix-phys-phys-frame/reproducers/04_free_count_inline_fails.rs`
with a **faithful** reproducer that actually demonstrates the limitation when run
through `/mnt/toolchain/verus/verus`. Two hard requirements:

1. **Hide the closed view behind a module boundary.** Put `Bitmap` (struct +
   `View` impl with `closed spec fn view` + `closed spec fn inv` +
   `number_of_bits`/`usage`) inside an inner `mod`, and call it from outside that
   `mod`. This mirrors the real cross-crate `::bitmap` boundary that makes
   `num_bits` opaque. Do **not** add a bogus `result > 0` ensures (it is
   unprovable and unrelated).
2. **Both forms must behave as documented.** The file must contain a PASS form
   (split `let nbits`/`let used`) that **verifies**, and a FAIL form (inlined
   `number_of_bits() - usage()`) that **fails on `num_bits >= 0`**. Document each
   with the actual observed Verus output.

A known-good faithful model (verified by me — split passes, inline fails on
`num_bits >= 0`):

```rust
use vstd::prelude::*;
verus! {
mod bm {
    use vstd::prelude::*;
    pub struct BitmapView { pub num_bits: int, pub usage: int }
    pub struct Bitmap { n: u32, u: u32 }
    impl View for Bitmap {
        type V = BitmapView;
        closed spec fn view(&self) -> BitmapView {
            BitmapView { num_bits: self.n as int, usage: self.u as int }
        }
    }
    impl Bitmap {
        pub closed spec fn inv(&self) -> bool { self.u <= self.n }
        pub fn number_of_bits(&self) -> (result: u32)
            requires self.inv(), ensures result as int == self@.num_bits { self.n }
        pub fn usage(&self) -> (result: u32)
            requires self.inv(), ensures result as int == self@.usage { self.u }
    }
}
use bm::Bitmap;
pub proof fn lemma_free_count(b: &Bitmap) requires b.inv(), b@.num_bits >= 0 {}

// PASS form: split bindings surface `nbits >= 0` (usize) ⇒ `num_bits >= 0`.
pub fn free_count_split(b: &Bitmap) -> u32 requires b.inv(), b@.usage <= b@.num_bits {
    let nbits = b.number_of_bits();
    let used = b.usage();
    proof! { lemma_free_count(b); }
    nbits - used
}
// FAIL form: lemma runs before any binding materializes the usize fact ⇒
//   error: precondition not satisfied ... b@.num_bits >= 0
pub fn free_count_inline(b: &Bitmap) -> u32 requires b.inv(), b@.usage <= b@.num_bits {
    proof! { lemma_free_count(b); }
    b.number_of_bits() - b.usage()
}
}
fn main() {}
```

Ship the FAIL form **commented out** (so the file as committed verifies), but its
documented error must be the one Verus actually emits when uncommented.

**Acceptance (run both, paste output into the fixer note):**
- Reproducer as committed (FAIL form commented out):
  `/mnt/toolchain/verus/verus verus-ai-logs/nanvix-phys-phys-frame/reproducers/04_free_count_inline_fails.rs`
  → must report **0 errors**.
- Temporarily uncomment the FAIL form and re-run the same command → must report
  **1 error** on `lemma_free_count`'s `num_bits >= 0` precondition. Re-comment it.

Do not touch `frame.rs`/`frame.spec.rs`/`frame.proof.rs` — only the reproducer
file is in scope. The `VERUS REWRITE` comment already cites this path, so no
comment change is needed once the file is faithful.
