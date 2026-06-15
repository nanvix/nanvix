## Response to Turn 1

### What I Changed

Item 8 (exec rewrite marker + minimal reproducer for `free_count`). Two changes,
both in scope; no `Inner::*` method, contract, or `frame.spec.rs` touched.

1. **`src/kernel/src/mm/phys/frame.rs`, `free_count` body (lines ~845–859).**
   Replaced the non-standard `// VERUS DEVIATION (pre-approved: ...)` comment with
   the project-standard `// VERUS REWRITE:` marker. The new comment states all
   three required points:
   - (a) **original idiom**: the single expression
     `inner.bitmap.number_of_bits() - inner.bitmap.usage()`.
   - (b) **semantic/cost equivalence**: split into `let nbits` / `let used`;
     `nbits - used` yields the identical `usize` (two already-present calls, one
     subtraction — no extra work, no extra storage).
   - (c) **Verus limitation**: `lemma_free_count` requires `bitmap@.num_bits >= 0`,
     but `Bitmap::inv()` hides the backing-slice length so the bound is opaque
     inside the lemma; binding `let nbits = number_of_bits()` materializes the
     `usize` postcondition (`nbits as int == num_bits`, `nbits >= 0`) at the call
     site, discharging the precondition. Inlining fails with
     `precondition not satisfied: inner.bitmap@.num_bits >= 0`.
   The comment cites the reproducer path. The exec statements (`let nbits`,
   `let used`, `proof! { lemma_free_count(inner); }`, `nbits - used`) are
   unchanged — only the comment was rewritten.

2. **New file
   `verus-ai-logs/nanvix-phys-phys-frame/reproducers/04_free_count_inline_fails.rs`.**
   Minimal, self-contained reproducer modeling the exact architecture: a `Bitmap`
   whose `inv()` hides the `num_bits` lower bound, a `number_of_bits()` whose
   `usize` postcondition is the only materialization point of `num_bits >= 0`, and
   a `lemma_free_count` requiring `num_bits >= 0`. It contains the PASS form
   (split bindings — verifies) and the FAIL form (inlined expression, commented
   out, with the precondition error documented inline).

### Verification

I confirmed the FAIL form against the real tree first by temporarily inlining the
expression — `make verify-kernel MODULE=mm::phys`:

```
error: precondition not satisfied
   --> src/kernel/src/mm/phys/frame.rs:851:9
851 |         lemma_free_count(inner);
 95 |         inner.bitmap@.num_bits >= 0,
    |         --------------------------- failed precondition
verification results:: 30 verified, 1 errors (partial verification with `--verify-*`)
```

Then restored the split form with the `VERUS REWRITE` marker.

`make verify-kernel MODULE=mm::phys`:
```
verification results:: 31 verified, 0 errors (partial verification with `--verify-*`)
  31 verified
  0 errors
  Exit code : 0
```

Marker present:
```
$ grep -n "VERUS REWRITE" src/kernel/src/mm/phys/frame.rs
845:    // VERUS REWRITE: original idiom is the single expression
```

Build (`./z build -- all-kernel`):
```
   Compiling kernel v0.16.17 (/home/ruize/nanvix-phy/src/kernel)
    Finished `dev` profile [optimized + debuginfo] target(s) in 11.35s
[OK]    Build complete.
```
0 warnings, 0 errors.

### Result: FIXED
