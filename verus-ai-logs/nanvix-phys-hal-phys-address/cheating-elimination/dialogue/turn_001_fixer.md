## Response to Turn 1

### What I Changed

All three AST-consistency divergences were addressed in
`src/kernel/src/hal/mem/types/address/phys.rs`. Two were eliminated by restoring the original
exec / proving it required; one is a mandatory out-of-scope trait method, now documented.

1. **`into_frame_number` (phys.rs ~165-175) — MISMATCH eliminated (exec restored to `dev`).**
   Removed the `let shift: usize = mem::FRAME_SHIFT;` binding and restored the `dev` original
   `let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;`. The proof lemma is now called with
   `mem::FRAME_SHIFT` directly. Verus accepts this — the `shift` binding was unnecessary.
   → tool now reports `PhysicalAddress::into_frame_number  MATCH`.

2. **`from_number` (phys.rs ~142-156) — MISMATCH, genuine Verus requirement, now documented.**
   I first tried the reviewer's default (restore the single line `frame.into_raw_value() *
   mem::FRAME_SIZE`, keep the `proof!`). It **fails** — see error below. I then tried
   `use_type_invariant(frame)` to keep the single line; that also **fails** (the `FrameNumber`
   type invariant is private to the `arch` crate). The split is therefore mandatory: the bound
   `frame@ <= spec_max()` is exposed only via `into_raw_value()`'s postcondition, which must land
   in context *before* the overflow-bearing multiply, with the lemma invoked between. I kept the
   minimal `addr_raw` split and added a `VERUS REWRITE:` comment plus a committed, executable
   minimal reproducer:
   `verus-ai-logs/nanvix-phys-hal-phys-address/cheating-elimination/repro/from_number.rs`.

3. **`clone_address` (phys.rs ~277-289) — EXTRA_IN_VERUS, mandatory trait method, documented.**
   It is a **required** method of the `sys::mm::Address` trait, which gained it during the verus
   pipeline (`git show dev:.../mod.rs | grep -c clone_address` → 0;
   `git show verus-ai-prove:.../mod.rs | grep -c clone_address` → 1). Since `PhysicalAddress:
   Address`, the impl method cannot be removed without editing the out-of-scope `sys` crate. Added
   a `VERUS REWRITE (interface addition):` comment and recorded it in `verification_todo.md`.

### Verification

`make verify-kernel MODULE=hal::mem::types::address::phys` — **exit 0, CLEAN**:
```
verification results:: 6 verified, 0 errors (partial verification with `--verify-*`)
  ✅ No cheating detected in module hal::mem::types::address::phys.
=== Summary ===
  verification: 6 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0  (in-module)
  status: CLEAN
```

`make verify` (full crate) — exit 0. `make build` — exit 0.

AST consistency (after fixes):
```
$ python3 .../ast_consistency.py src/kernel/.../phys.rs count
⚠️  1 mismatched, 1 extra (15 functions match)
```
- `into_frame_number`: was MISMATCH → now **MATCH** (one mismatch eliminated; from 2→1).
- `from_number`: MISMATCH — required Verus rewrite, carries `VERUS REWRITE` + committed reproducer.
- `clone_address`: EXTRA_IN_VERUS — mandatory out-of-scope trait method, carries `VERUS REWRITE` +
  git evidence.

Per the reviewer's exit criterion ("zero mismatches **OR** every surviving divergence carries a
`VERUS REWRITE` comment + committed reproducer"), both surviving divergences are now compliant:
```
$ grep -n "VERUS REWRITE" src/kernel/.../phys.rs
142:    // VERUS REWRITE: the original `frame.into_raw_value() * mem::FRAME_SIZE` ...
277:    // VERUS REWRITE (interface addition): `clone_address` is a *required* method ...
```

Reproducer evidence (executed):
```
$ verus from_number.rs
error: possible arithmetic underflow/overflow
  --> from_number.rs:69:23
   |
69 |     let addr: usize = frame.into_raw_value() * SIZE;
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
verification results:: 2 verified, 1 errors
```
The single-line `bad` form fails exactly as `from_number` would; the split `good` form is among
the 2 verified.

### Result: FIXED

- `into_frame_number` MISMATCH **eliminated** (exec restored to `dev` baseline; now MATCH).
- `from_number` MISMATCH is a **proven-required** Verus rewrite — both reviewer-suggested
  single-line forms fail; documented with `VERUS REWRITE` + committed, executable reproducer.
- `clone_address` EXTRA is a **mandatory** out-of-scope `sys::mm::Address` trait method (git
  evidence) — cannot be deleted here; documented with `VERUS REWRITE` + recorded in
  `verification_todo.md`.

#### Exact errors from the two attempts to make `from_number` MATCH (single-line)

Attempt A — restore `dev` single line, keep `proof! { lemma_from_number_no_overflow(frame); }`:
```
error: precondition not satisfied
   |
144 |             lemma_from_number_no_overflow(frame);
    |
 19 |         spec_frame_raw_value(frame) <= spec_max_frame_number(),
    |         ------------------------------------------------------ failed precondition
verification results:: 5 verified, 1 errors
```

Attempt B — single line + `proof! { use_type_invariant(frame); lemma_from_number_no_overflow(frame); }`:
```
error: Verus Internal Error: missing type invariant function
   |
144 |             use_type_invariant(frame);
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^
error: could not compile `kernel` (bin "kernel") due to 1 previous error
```
(`FrameNumber`'s `#[verifier::type_invariant]` is defined in the `arch` crate and is not
accessible across the crate boundary, so the bound can only enter via `into_raw_value()`'s
postcondition — which forces the split.)
