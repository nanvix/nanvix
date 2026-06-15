## Turn 1: AST consistency / exec-rewrite documentation (MemoryRegion::start)

### Progress
- Done (PASS):
  - **Zero admit()** — `region.rs/.spec.rs/.proof.rs` contain no `admit()`. The 12 admits
    reported by `make verify-kernel` are all in `mm/phys/manager.proof.rs`,
    `mm/virt/identity_map.*` (out of scope; see `cheating-detail.txt`).
  - **Zero assume()** — `assume=0` globally; none in the region module.
  - **Zero trusted functions** — `trusted=0`; grep of the three region files finds none.
  - **Zero exec_allows_no_decreases_clause** — `no_decreases=0`.
  - **Zero cfg-gated exec code** — only `#[cfg(verus_keep_ghost)] include!("region.spec.rs")`
    and `include!("region.proof.rs")` (lines 9–12): spec/proof imports, which are allowed.
    No cfg-gated exec logic.
  - **Zero external_body in region** — `cheating-detail.txt` lists all 19 `external_body`
    in `mm/phys/*` only; none in `hal/mem/types/region.*`. The TCB-allowed list need not
    be consulted for this module (nothing to allow).
  - **No specs weakened** — `spec_drift.py check hal-memory-region` (baseline
    `c4b739de01c1` → HEAD) → **exit 0**, 0 contract drift.
  - **Cross-module regression** — `make verify-kernel` → **exit 0** (module
    `hal::mem::types::region` verified; whole-crate verify cached, exit 0).
  - **Verification + build** — `make verify-kernel` exit 0; `./z build` recompiled
    `kernel v0.16.17` → Finished, **0 errors, 0 warnings**.
- Current: **AST consistency** + **exec-rewrite documentation** — FAIL (one site).
- Remaining: none — this is the only outstanding blocker.

### Verification
Ran `ast_consistency.py src/kernel/src/hal/mem/types/region.rs summary`:
`Consistent: ❌ NO (matched=27 mismatched=1 missing=0 extra=0)`.

The single mismatch is `MemoryRegion::start` (`diff --name "MemoryRegion::start"`):

```diff
     pub fn start(&self) -> T {
-        self.start.clone()
+        self.start.clone_address()
     }
```

Investigation:
- `clone_address` is a required method of the `sys::mm::Address` trait, added during the
  address-layer pipeline; it carries the verified contract `result@ == self@`
  (`PhysicalAddress` impl: `phys.rs:286`, documented at `phys.rs:277`). `Clone::clone`
  has no Verus spec, so the `#[verus_spec] ensures result@ == self@.start` on
  `MemoryRegion::start` (region.rs:210–216) cannot be discharged with `.clone()`.
- The rewrite is therefore a **semantically-equivalent, view-preserving** substitution for
  a genuine Verus limitation — i.e. an *acceptable* deviation under the ast-consistency
  rules. **But** the call site at `region.rs:215` carries **no** `VERUS REWRITE` comment
  and no minimal reproducer. The checklist requires "All exec rewrites have VERUS REWRITE
  comment and minimal reproducer". The comment at `phys.rs:277` documents the *trait method
  definition*, not this *call-site* rewrite.

This fails two checklist items simultaneously:
- "AST consistency: zero mismatches (only semantically-equivalent rewrites for verified
  Verus limitations)" — the rewrite is semantically equivalent, but undocumented, so it is
  not yet an approved deviation.
- "All exec rewrites have VERUS REWRITE comment and minimal reproducer."

### Fix Request
Document the `.clone()` → `.clone_address()` exec rewrite at its call site so it qualifies
as an approved semantically-equivalent deviation.

1. In `src/kernel/src/hal/mem/types/region.rs`, immediately above `pub fn start(&self) -> T`
   (the one at line ~214, inside `impl<T: Address> MemoryRegion<T>`), add a `VERUS REWRITE`
   comment, e.g.:

   ```rust
   // VERUS REWRITE (view-preserving clone): `self.start.clone()` -> `self.start.clone_address()`.
   // `Clone::clone` has no Verus spec, so the postcondition `result@ == self@.start` cannot be
   // discharged through the bare `Clone` supertrait. `Address::clone_address` is the required
   // trait method that carries the verified contract `result@ == self@` (see
   // src/kernel/src/hal/mem/types/address/phys.rs:277-288). Same value, same complexity as a
   // `Copy`; purely a view-preserving substitution for a Verus limitation.
   // Minimal reproducer: a generic `fn f<T: Clone + View>(x: &T) -> (r: T) ensures r@ == x@ { x.clone() }`
   // fails to verify ("postcondition not satisfied") because `Clone::clone` is unspecified;
   // replacing the body with `x.clone_address()` (the spec'd trait method) verifies.
   ```

   Keep the wording accurate to the actual reproducer you confirm.

2. Do **not** change `region.rs:179` (`start.clone().into_raw_value()`) — that `.clone()` is
   unchanged from the original (AST MATCH) and its result feeds `into_raw_value()` with no
   `@`-preservation obligation, so no rewrite is needed there.

3. Verify:
   - `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py \
      src/kernel/src/hal/mem/types/region.rs summary` — `MemoryRegion::start` will remain a
      MISMATCH (the exec hash differs), so additionally confirm the deviation is now
      **documented**: the comment must name it a VERUS REWRITE and include the reproducer.
      (A comment does not change the AST hash; the reviewer accepts the MISMATCH only once a
      compliant VERUS REWRITE comment + reproducer is present at the site.)
   - `make verify-kernel` → still 0 errors.
   - `./z build` → still 0 warnings.

Justification alone is not a fix: add the comment to the source and show the updated
`ast_consistency.py diff --name "MemoryRegion::start"` plus the surrounding source lines
proving the `VERUS REWRITE` comment and reproducer are in place.
