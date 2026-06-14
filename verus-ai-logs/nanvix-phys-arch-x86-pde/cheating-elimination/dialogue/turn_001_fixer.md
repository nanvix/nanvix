## Response to Turn 1

### What I Changed

The reviewer flagged one undocumented exec rewrite in `frame_address` (a `let raw = …`
binding diverging from the original single-expression body). The reviewer's **preferred**
route was to collapse the body back to the original single expression and remove the
mismatch entirely. I took that route.

**`src/libs/arch/src/x86/mem/paging/pde.rs` (`PageDirectoryEntry::frame_address`, ~line 420)**

Before (the flagged rewrite):
```rust
pub fn frame_address(&self) -> usize {
    let raw: usize = self.frame.into_raw_value();
    proof! { lemma_frame_address(raw); }
    raw << crate::mem::FRAME_SHIFT
}
```
After (exec body identical to the original `dev` source):
```rust
pub fn frame_address(&self) -> usize {
    proof! { broadcast use lemma_frame_address; }
    self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
}
```
The `proof!` block is erased by the AST checker and by normal `cargo build`, so the
exec-visible body is exactly the original single expression
`self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`.

**`src/libs/arch/src/x86/mem/paging/pde.proof.rs` (`lemma_frame_address`, line 16)**

Made the lemma a `broadcast` lemma with a trigger so it applies to the inline shift
without needing the result bound to a local:
```rust
pub broadcast proof fn lemma_frame_address(raw: usize)
    requires
        0 <= raw as int <= FrameNumber::spec_max(),
    ensures
        #![trigger (raw << crate::mem::FRAME_SHIFT)]
        (raw << crate::mem::FRAME_SHIFT) as int == raw as int * (crate::mem::FRAME_SIZE as int),
        (raw << crate::mem::FRAME_SHIFT) as int % (crate::mem::FRAME_SIZE as int) == 0,
{ … unchanged proof body … }
```
The `into_raw_value()` postcondition (`result as int == self@` and
`0 <= self@ <= spec_max()`) discharges the broadcast `requires`, and the trigger
`(raw << FRAME_SHIFT)` fires on the inline shift — so no `let` binding is needed.

This removes the AST mismatch entirely (the reviewer's preferred outcome), rather than
just documenting it.

#### Why the simpler collapses don't work (evidence)

1. Drop the lemma, keep only the original expression:
   ```
   error: postcondition not satisfied
   417 |  result as int == self@.frame * (crate::mem::FRAME_SIZE as int),
   418 |  result as int % (crate::mem::FRAME_SIZE as int) == 0,
   verification results:: 46 verified, 1 errors
   ```
2. Call the lemma positionally inside a `proof!` block on the exec value:
   ```
   error: cannot call function
     `arch::x86::mem::paging::frame::number::FrameNumber::into_raw_value` with mode exec
   421 |  proof! { lemma_frame_address(self.frame.into_raw_value()); }
   ```
   (This is the limitation the reviewer's reproducer predicted; a proof fn cannot take the
   exec call. The broadcast trigger avoids needing the value as an argument at all.)

### Verification

`make verify-arch` → exit 0:
```
    Checking arch v0.16.17 (/home/ruize/nanvix-phy-specs-bottom-up/src/libs/arch)
verification results:: 47 verified, 0 errors
    Finished `dev` profile [optimized + debuginfo] target(s)
=== Results ===
  47 verified
  0 errors
  Exit code : 0
=== Summary ===
  verification: 47 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0
```
(The 3 `external_body` are the out-of-scope, TCB-allowed `invlpg`/`table::read`/`table::write`;
none are in `pde.*`. cfg_gate=0.)

AST consistency vs the original `dev` baseline (mismatch now eliminated):
```
git show dev:src/libs/arch/src/x86/mem/paging/pde.rs > /tmp/pde_orig.rs
ast_consistency.py /tmp/pde_orig.rs src/libs/arch/src/x86/mem/paging/pde.rs summary
→ Consistent: ✅ YES (matched=23 mismatched=0 missing=0 extra=0)
ast_consistency.py … count
→ ✅ Consistent: 23 functions, 2 structs match.
```

Normal build (Verus erased) — `./z build -- check-kernel`:
```
    Checking arch v0.16.17 (…/src/libs/arch)
    Checking kernel v0.16.17 (…/src/kernel)
    Finished `dev` profile [optimized + debuginfo] target(s) in 3.42s
[OK]    Build complete.
```
(`make build` is a no-op in this tree; `check-kernel` compiles the `arch` exec crate with
all Verus constructs erased, confirming the `proof!`/`broadcast` additions vanish in a
normal build.)

Full-crate regression — `make verify` → exit 0: arch `47 verified, 0 errors`,
kernel `76 verified, 0 errors`.

### Result: FIXED
