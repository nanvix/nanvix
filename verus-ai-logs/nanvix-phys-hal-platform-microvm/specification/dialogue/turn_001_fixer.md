## Response to Turn 1

### What I Changed

**File:** `src/kernel/src/hal/platform/microvm/mod.proof.rs`

Removed the `admit();` body from the module's own proof function
`lemma_translation_injective` and replaced it with an empty body. Verus
discharges `v.injective()` automatically from the `open` identity definition of
`spec_gva_to_gpa` (`spec_gva_to_gpa(x) == x`), where injectivity reduces to
`forall|a,b| a == b ==> a == b`.

Before:
```rust
pub proof fn lemma_translation_injective(v: MicrovmTranslationView)
    ensures
        v.injective(),
{
    admit();
}
```

After:
```rust
pub proof fn lemma_translation_injective(v: MicrovmTranslationView)
    ensures
        v.injective(),
{
}
```

Also updated the accompanying comment (formerly "proving phase discharges it")
to state that the lemma is now discharged directly from the identity definition.

No other files changed. The in-scope exec contract on `gva_to_gpa` (`mod.rs`)
and the View in `mod.spec.rs` are unchanged.

### Verification

`make verify-kernel MODULE=hal::platform::microvm` → exit 0:
```
=== Summary ===
  verification: 2 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=25 admit=0 trusted=0 no_decreases=0 cfg_gate=7
  coverage: 1/31 exec functions have contracts
  status: CLEAN
```

- `admit=0` (was `admit=1`) — the `mod.proof.rs:19 lemma_translation_injective: admit`
  entry is gone.
- `status: CLEAN` (was `CHEATING_DETECTED`).
- The remaining `external_body=25` / `cfg_gate=7` are all in OTHER modules
  (external dependencies), not this module's responsibility this turn.

`./z build -- all-kernel` → exit 0:
```
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.19s
cp -f --preserve .../kernel.elf .../bin/kernel.elf
[OK]    Build complete.
```
Dual compilation confirmed — Verus constructs erase cleanly in the normal build.

### Result: FIXED

Item 13 ("No cheating on module's own functions") now passes: the empty proof
body auto-proved from the identity definition, `admit=0`, status CLEAN, exit 0.
The explicit `assert ... by` fallback was not needed.
