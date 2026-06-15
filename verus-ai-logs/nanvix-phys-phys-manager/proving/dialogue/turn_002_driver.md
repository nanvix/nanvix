## Turn 2: Item 10 (Verification + build, 0 errors / 0 warnings) — verifying fixer

### Progress
- Done:
  1. No specs weakened — PASS (turn 1; re-confirmed below)
  2. Zero `admit()` (manager) — PASS
  3. Zero unlisted `external_body` — PASS
  4. Zero non-boundary `assume`/`assume_specification` — PASS
  5. No cfg-gated exec code — PASS
  6. Cheating audit (counts/locations) — reported
  7. Verus-limitation reproducer — N/A (no new limitation claim)
  8. Exec rewrites minimal/equivalent — PASS
  9. Cross-module regression — PASS
  10. Verification + build, 0 errors / 0 warnings — **FIXED**
- Current: item 10 (the only turn-1 FAIL)
- Remaining: none

### Verification

I verified the fixer's change with my own tool runs, not verbal claims.

**The fix (read from source + git):** `git diff` shows working tree clean
(committed). `git log -p` confirms the entire change to `manager.rs` is a single
added line:

```
+        #[allow(unused_variables)]
         #[cfg_attr(verus_keep_ghost, verus_spec(
             invariant
                 g_old == old(self)@,
                 ...
```

placed on the `for i in 0..count` loop (manager.rs:234–246). No invariant clause
changed; `i` stays named so the invariant (`frames@.len() == i`,
`user_addr_set(frames@).len() == i`) still references it. The `alloc_many_kernel_frames`
loop was correctly left untouched (its `i` is used in exec: `base_raw + i * PAGE_SIZE`).

**1. Build (`./z build -- all`) with forced kernel recompile (`touch manager.rs`):**
```
Compiling kernel v0.16.17 (.../src/kernel)
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.67s
cp -f --preserve .../kernel.elf .../bin/kernel.elf
...
[OK]    Build complete.
```
The kernel crate reached `Finished` with **no** `unused variable: \`i\`` error and
no rustc `warning:`/`error:` line. The lone `Warning: Sysroot directory ... not
found` is a `z.py` build-system note (not a rustc diagnostic). Under `-D warnings`
any real warning aborts at the first diagnostic, so reaching `[OK] Build complete.`
confirms **0 warnings / 0 errors** for the kernel crate. The turn-1 FAIL is gone.

**2. Verification (`make verify-kernel`):**
```
verification results:: 112 verified, 0 errors
  Exit code : 0
  cheating: assume=0 external_body=18 admit=3 trusted=0 no_decreases=0 cfg_gate=12
```
0 errors. Cheating counts are byte-identical to turn 1 — adding a lint attribute
introduces no new `external_body`/`admit`/`assume`/cfg-gate. The whole-crate
`admit=3` is all `mm/virt/identity_map.rs` (out of scope); `external_body=18` all
in `tcb-allowed.md`; `cfg_gate=12` are `verus_spec` loop-invariant attrs, not exec
gates. `#[allow(unused_variables)]` is an unconditional lint attribute, **not** a
`#[cfg(...)]` gate, so item 5 stays PASS.

**Regression re-checks on the other items affected by the edit:**
- Item 1 (no specs weakened): `git diff` of `manager.spec.rs`/`manager.proof.rs`
  empty; the only `manager.rs` delta is the added attribute line — no
  `requires`/`ensures`/`invariant` clause altered. PASS holds.
- Item 5 (no cfg-gated exec): attribute is not a cfg gate; loop body identical in
  both configs. PASS holds.
- Item 9 (cross-module regression): `make verify-kernel` → 112 verified, 0 errors
  across all modules. PASS holds.

### Fix Request
None — item 10 is FIXED and all other items remain PASS. No further action.

### STOP status
All 10 checklist items are PASS or FIXED with tool-backed evidence. Creating
`STOP` with `RESOLVED`.
