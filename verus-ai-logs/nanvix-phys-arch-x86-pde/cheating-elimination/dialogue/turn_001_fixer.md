## Response to Turn 1

### What I Changed

**1. Added `// VERUS REWRITE:` comment** — `src/libs/arch/src/x86/mem/paging/pde.rs:420-431`
(immediately above `pub fn frame_address`, after its `#[verus_spec]`, mirroring
`phys.rs:142-148`). The comment states:
- original exec was the single expression `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`;
- the `let raw` binding is mandatory so `into_raw_value()`'s bound postcondition
  (`0 <= self@ <= FrameNumber::spec_max()`) is in context before the overflow-bearing shift,
  letting `lemma_frame_address(raw)` discharge the no-overflow + `FRAME_SIZE`-alignment `ensures`
  between the call and the shift (an exec call cannot live inside `proof!`, so the operand must be
  named);
- same value, same operations, same time/space complexity — semantically equivalent;
- a `Reproducer:` path line.

The original single-expression form was confirmed at the true pre-verus commit
`1b6d97508` (`[arch] E: Fix asymmetries between PDE and PTE`):
```rust
pub fn frame_address(&self) -> usize {
    self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
}
```

**2. Created the minimal reproducer** —
`verus-ai-logs/nanvix-phys-arch-x86-pde/cheating-elimination/repro/frame_address.rs`
(format mirrors `nanvix-phys-hal-phys-address/.../repro/from_number.rs`): opaque `Copy`
`FrameNumber` whose bound is exposed only via the `into_raw_value()` postcondition, a
`Pde::bad` (single-expression `raw << FRAME_SHIFT`, no binding — FAILS) and a `Pde::good`
(the `let raw` + `proof { lemma_frame_address(raw); }` rewrite — PASSES). `lemma_frame_address`
is `external_body` to isolate the *ordering* limitation (in `pde.proof.rs` it is fully proven
via `lemma_usize_shl_is_mul` + the div/mod lemmas).

**No exec semantics of `frame_address` were changed** — only the comment and the repro file
were added.

### Verification

**Reproducer FAILS without the rewrite (`bad`):**
```
$ verus frame_address.rs
error: postcondition not satisfied
  --> frame_address.rs:89:13
   |
89 |             result as int == self.frame@ * (FRAME_SIZE as int),
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ failed this postcondition
92 |         self.frame.into_raw_value() << FRAME_SHIFT
   |         ------------------------------------------ at the end of the function body
error: postcondition not satisfied
  --> frame_address.rs:90:13
   |
90 |             result as int % (FRAME_SIZE as int) == 0,
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ failed this postcondition
verification results:: 4 verified, 1 errors
```

**Reproducer PASSES with the rewrite (`good`):**
```
$ verus --verify-root --verify-function good frame_address.rs
note: verifying root module (selected functions)
verification results:: 1 verified, 0 errors (partial verification with `--verify-*`)
```

**`grep -rn "VERUS REWRITE" pde.rs`:**
```
420:    // VERUS REWRITE: the original `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`
```

**`make verify-arch` — exit 0, cheating counts unchanged:**
```
verification results:: 48 verified, 0 errors
  Exit code : 0
  cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```

**`./z build -- all`:**
```
[OK]    Build complete.   (exit 0)
```

**AST consistency** vs the true pre-verus original (`1b6d97508`): the only MISMATCH is
`PageDirectoryEntry::frame_address` — now the documented, reproducer-backed VERUS REWRITE
(item 7/8 satisfied). (`from_raw`/`raw` show as `EXTRA_IN_VERUS`: the pre-existing
`impl TableEntry` delegation present since the spec base, not introduced this turn. Against the
task base branch `verus-ai-prove` the AST is fully consistent — 23 functions, 2 structs match.)

### Result: FIXED
