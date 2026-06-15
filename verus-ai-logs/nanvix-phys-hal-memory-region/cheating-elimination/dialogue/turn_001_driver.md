## Turn 1: AST consistency / undocumented exec rewrite of `MemoryRegion::start`

### Progress
- Done (PASS):
  - Zero `admit()` — global `admit=0`; grep of `region.rs`/`region.spec.rs`/`region.proof.rs` finds none.
  - Zero `assume()` — global `assume=0`; none in the region files.
  - Zero trusted functions — global `trusted=0`.
  - Zero `exec_allows_no_decreases_clause` — global `no_decreases=0`.
  - Zero cfg-gated exec code — `region.rs` has exactly two `#[cfg(verus_keep_ghost)]` lines, both gating `include!("region.spec.rs")` / `include!("region.proof.rs")` (ghost spec/proof). The detector (`scripts/verify.sh::count_cfg_gates`) explicitly excludes `include!` targets, and the rule allows imports of ghost files. No cfg-gated exec.
  - Zero `external_body` in region — grep finds none in any of the three region files. The 25 global `external_body` (cheating-detail.txt) are all in `mm/phys/*`, `hal/mem/types/address/*`, `mm/virt/*`, and every one is enumerated in `verus-ai-logs/tcb-allowed.md`. None belongs to the region module.
  - Per-`external_body` listing check — N/A for region (none surviving); global set fully governed by tcb-allowed.md.
  - No specs weakened — `spec_drift.py git-diff src/.../region.rs --before 3b7e25cc23 --after HEAD`: **0 contract drift** (ensures removed: 0, requires added: 0; 9 functions added, 0 removed).
  - Cross-module regression — `make verify-kernel`: **41 verified, 0 errors, exit 0**.
  - Verification + build — `make verify-kernel`: 41 verified, 0 errors; `cargo`/verus build finished with no warnings.
- Current: **AST consistency (FAIL)** and **VERUS REWRITE comment + minimal reproducer (FAIL)** — same root cause.
- Remaining: none unchecked; the two coupled FAIL items above must be fixed.

### Verification
Tooling run (base ref auto-detected = `exp` merge-base `3b7e25cc23`):

`ast_consistency.py src/kernel/src/hal/mem/types/region.rs summary` → all elements MATCH **except one**:

```
MemoryRegion::start                      MISMATCH
```

Diff (`... diff --name "MemoryRegion::start"`):

```
     pub fn start(&self) -> T {
-        self.start.clone()      # original / base (also still in HEAD commit 3ce6a72)
+        self.start              # current working tree
     }
```

I verified the rewrite is *forced* by a genuine Verus limitation, not a cosmetic edit. Restoring `self.start.clone()` and running `make verify-kernel`:

```
error: postcondition not satisfied
   --> src/kernel/src/hal/mem/types/region.rs:218:13
    |
218 |             crate::hal::mem::spec_addr(&result) == self@.start,
    |             ^^^ failed this postcondition
221 |         self.start.clone()
    |         ------------------ at the end of the function body
verification results:: 40 verified, 1 errors
```

Reverting to `self.start` → **41 verified, 0 errors**. Root cause: `Clone::clone` on a generic `T: Address` carries no spec relating `spec_addr(&result)` to `spec_addr(&self.start)`, so the postcondition is unprovable through `.clone()`. `Address: Copy` (confirmed at `src/libs/sys/src/sys/mm/address/mod.rs:33`, `Self: ... + Copy + ...`) makes the direct field read `self.start` identity (`result == self.start`), discharging the postcondition by congruence of `spec_addr`. The rewrite is therefore **semantically equivalent** and qualifies as a "verified Verus limitation" rewrite.

**However**, the checklist requires every exec rewrite to carry a `VERUS REWRITE` comment and a minimal reproducer. The body of `MemoryRegion::start` (region.rs:220–222) has **no** such comment. The block comment on the `ensures` (lines 210–219) explains the *postcondition* but is not a tagged `VERUS REWRITE` note and cites no reproducer. So the mismatch is currently **undocumented** → FAIL on both item 7 (AST consistency) and item 8 (VERUS REWRITE comment + reproducer).

Note: HEAD commit `3ce6a72` actually committed the broken `self.start.clone()`; the verifying `self.start` existed only as an uncommitted working-tree edit. I restored `self.start` (working tree now verifies green) but the rewrite remains undocumented and uncommitted.

### Fix Request
Document the `MemoryRegion::start` exec rewrite so the single AST mismatch is a justified, semantically-equivalent rewrite for a verified Verus limitation.

1. In `src/kernel/src/hal/mem/types/region.rs`, inside the body of `MemoryRegion::start` (currently lines 220–222), keep `self.start` and add an inline `VERUS REWRITE` comment, e.g.:

```rust
pub fn start(&self) -> T {
    // VERUS REWRITE: original exec body was `self.start.clone()`.
    // Verus limitation: `Clone::clone` on a generic `T: Address` has no spec
    // relating `spec_addr(&result)` to `spec_addr(&self.start)`, so the
    // postcondition `spec_addr(&result) == self@.start` cannot be discharged
    // through `.clone()`. Minimal reproducer — restoring `self.start.clone()`:
    //   error: postcondition not satisfied
    //     --> region.rs:218:13  (spec_addr(&result) == self@.start)
    //     221 | self.start.clone()  -- at the end of the function body
    //   verification results:: 40 verified, 1 errors
    // `Address: Copy` (sys/mm/address/mod.rs:33) makes the direct field read
    // identity (`result == self.start`), discharging the postcondition.
    // Semantically equivalent: clone of a `Copy` type is a bitwise copy.
    self.start
}
```

(The wording may be adjusted, but it MUST: (a) be tagged `VERUS REWRITE`, (b) state the original `self.start.clone()` body, (c) name the Verus limitation, and (d) include the minimal reproducer output above.)

2. Verify the fix:
   - `make verify-kernel` → must remain **41 verified, 0 errors, exit 0**.
   - `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/kernel/src/hal/mem/types/region.rs summary` → `MemoryRegion::start` will still report MISMATCH (the tool compares exec bodies); that single mismatch is now acceptable because it is a documented, semantically-equivalent rewrite for a verified Verus limitation. No other element may become MISMATCH.

Do not attempt to make the mismatch disappear by reverting to `.clone()` — that reintroduces the verification failure proven above. The only acceptable resolution is the `VERUS REWRITE` documentation.
