## Turn 1: Exec-rewrite documentation for `align_up` (VERUS REWRITE comment + minimal reproducer)

### Progress
- Done (PASS / verified with tool output):
  - **Zero admit()** — `make verify-bump-allocator` → `admit=0`.
  - **Zero assume()** — `assume=0`.
  - **Zero trusted functions** — `trusted=0`.
  - **Zero exec_allows_no_decreases_clause** — `no_decreases=0`.
  - **Zero cfg-gated exec code** — `cfg_gate=0`. The only `cfg` attributes are
    `#[cfg(verus_keep_ghost)]` on the `include!` of `lib.spec.rs`/`lib.proof.rs`
    (ghost-only) and `#[cfg(test)]` on the unit-test module — both allowed.
  - **external_body only if in tcb-allowed** — `external_body=2`
    (`lib.rs:315 alloc`, `lib.rs:402 alloc_as`). Both are listed in
    `verus-ai-logs/tcb-allowed.md` with rationale. PASS.
  - **Each surviving external_body confirmed in tcb-allowed** — both `alloc` and
    `alloc_as` present in the allowed list. PASS.
  - **No specs weakened** — `python3 scripts/spec_drift.py check bump-allocator`
    → baseline `4d1dd731bf2e` → HEAD: *0 contract drift, 0 ensures removed,
    0 requires added*. Exit 0. PASS.
  - **Cross-module regression** — `make verify` (bitmap, sys, nanvix-slab,
    bump-allocator, kernel) all `Exit code: 0`. PASS.
  - **Verification + build** — `make verify-bump-allocator` exit 0;
    `./z build -- all` exit 0; `cargo build` on the crate: 0 warnings, 0 errors.
    PASS.
- Current: **All exec rewrites have VERUS REWRITE comment and minimal reproducer**
  (tied to the single AST mismatch on `align_up`).
- Remaining: none after the current item is fixed.

### Verification

`scripts/ast_consistency.py --base-ref exp src/libs/bump_allocator/src/lib.rs
summary` reports exactly **one MISMATCH**: `align_up`. All other functions and
structs MATCH.

`diff --name align_up` shows the exec body changed from:

```rust
value.div_ceil(alignment).checked_mul(alignment)
```

to an open-coded ceiling division (`r = value % alignment; qd = value / alignment;
q = if r == 0 { qd } else { qd + 1 }; q.checked_mul(alignment)`).

I confirmed the underlying Verus limitation is real: `grep -rn div_ceil
/mnt/toolchain/verus/vstd/` returns **no results** — vstd ships no specification
for `usize::div_ceil`. The rewrite is semantically equivalent and is backed by
`lemma_ceil_div` in `lib.proof.rs`, which is fully discharged. So the rewrite
**itself** is acceptable under the "semantically-equivalent rewrite for a verified
Verus limitation" rule.

**However, the documentation requirement is NOT met:**
1. The comment at `lib.rs:137` is tagged `// VERUS DEVIATION:`, not the required
   `// VERUS REWRITE` tag this checklist item mandates.
2. There is **no minimal reproducer** demonstrating that Verus rejects the
   original `value.div_ceil(alignment)` construct. The comment asserts the
   limitation but provides no reproducible evidence path. A claim in prose is not
   a reproducer.

This is a FAIL on "All exec rewrites have VERUS REWRITE comment and minimal
reproducer."

### Fix Request

Do **both** of the following at the `align_up` rewrite site (`src/libs/bump_allocator/src/lib.rs`, around lines 137–146):

1. **Rename the tag** from `// VERUS DEVIATION:` to `// VERUS REWRITE:` so the
   rewrite is detectable by the standard tag the checklist requires. Keep the
   existing equivalence/overflow rationale text.

2. **Add a minimal reproducer reference.** Create a minimal standalone Verus
   snippet that shows the original `value.div_ceil(alignment)` body cannot be
   verified (e.g. Verus emits "no specification" / unsupported for `div_ceil`),
   and reference it from the comment by relative path, e.g.:

   ```
   // VERUS REWRITE: original body was `value.div_ceil(alignment).checked_mul(alignment)`.
   // Verus ships no spec for `usize::div_ceil` (confirmed: `grep -r div_ceil vstd/`
   // returns nothing). Minimal reproducer:
   //   verus-ai-logs/nanvix-phys-bump-allocator/cheating-elimination/repro/div_ceil_no_spec.rs
   // ...existing equivalence + non-overflow rationale (lemma_ceil_div)...
   ```

   Place the reproducer file under
   `verus-ai-logs/nanvix-phys-bump-allocator/cheating-elimination/repro/div_ceil_no_spec.rs`
   containing the smallest `verus!{ ... }` function whose body is
   `value.div_ceil(alignment)` (returning the up-aligned value), plus a comment
   line capturing the exact Verus error message produced when it is run through
   the project's verifier.

**Do not** justify keeping `VERUS DEVIATION` in prose — change the tag and add the
reproducer file.

**Verify after fixing:**
- `grep -n "VERUS REWRITE" src/libs/bump_allocator/src/lib.rs` → shows the tag.
- The referenced reproducer file exists and contains the original `div_ceil` body
  plus the captured Verus error.
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref
  exp src/libs/bump_allocator/src/lib.rs summary` → still only the `align_up`
  MISMATCH (no new mismatches), confirming exec semantics unchanged by the edit.
- `make verify-bump-allocator` → exit 0, `admit=0 assume=0 external_body=2`.
