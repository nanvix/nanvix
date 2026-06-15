## Turn 1: Full checklist walk-through (proving phase, phys-frame)

### Progress
- Done (PASS): #2 admit, #3 external_body, #5 cfg-gated exec, #6 cheating audit, #8 exec rewrites
- Current (FAIL): #10 Verification (`make verify-kernel` fails to build) — also the root of concerns under #1 and #7
- Remaining (could not fully validate while build is broken): #1 spec-drift (partial), #7 limitation reproducer, #9 cross-module regression (blocked by #10)

### Verification (commands run + findings)

`make verify-kernel` output:
```
verification results:: 112 verified, 0 errors
error[E0080]: evaluation panicked: does not have the expected size
    --> src/kernel/src/mm/phys/frame.proof.rs:1:1
error: could not compile `kernel` (bin "kernel") due to 1 previous error
  status: VERIFY_FAILED  (exit 101)
  cheating: assume=0 external_body=17 admit=13 trusted=0 cfg_gate=13   (whole-kernel totals)
```

Per-item findings (phys-frame scope = `frame.rs` / `frame.spec.rs` / `frame.proof.rs`):

- **#2 Zero admit — PASS.** `grep -c admit()` over all three frame files = 0/0/0. (The 13 kernel admits are in `manager.proof.rs` and `mod.proof.rs`, out of phys-frame scope.)
- **#3 external_body only if in `verus-ai-logs/tcb-allowed.md` — PASS.** 8 `external_body` in `frame.rs`: `instance` (1245), `init` (1272), `alloc` (1328), `alloc_contiguous` (1358), `free_count` (1379), `free` (1397), `book` (1439), `alloc_range` (1460). All 8 are individually listed in `tcb-allowed.md` (instance §Allowed; init §Skip + §Cross-module; the other 6 under "Cross-module dependencies marked external_body"). `share`/`refcount`/`is_covered` are NOT external_body. None in spec/proof files.
- **#4 assume/assume_specification — DOCUMENTED CONCERN (not blocking).** `assume=0`. `frame.spec.rs` has 2 `assume_specification` (`PageAligned::<T> as Address::into_raw_value`, `PageAligned::<T> as Deref::deref`). Both are *intra-crate* (`crate::hal::mem`), not std/external — the checklist's strict wording allows only external-bottom std/external boundaries. They ARE recorded in `tcb-allowed.md` (lines 159-160) as trusted-until-HAL-verified placeholders, matching the established bottom-up convention. Accepted as-is for this phase, but noting it does not meet the literal "std/external only" bar.
- **#5 No cfg-gated exec code (branches/expressions/match arms) — PASS.** Every `#[cfg(not(verus_keep_ghost))]` in `frame.rs` gates ONLY `error!(...)` logging or `debug_assert_eq!(...)` — verified by enumerating all gated lines. No branch, match-arm, or value-producing expression is cfg-gated; control flow (`return Err(...)`) is outside the gate. `#[cfg(verus_keep_ghost)]` on lines 49/52 only gates the `include!` of the spec/proof files (required idiom).
- **#6 Cheating audit (frame scope) — counts: admit=0, external_body=8 (all allow-listed), assume=0, assume_specification=2 (allow-listed), cfg-gated-exec=0 (logging-only).**
- **#8 Exec rewrites minimal/semantically-equivalent — PASS (spot-checked).** `bugs.md` documents the `into_frame_number()` → `addr.into_raw_value() / FRAME_SIZE` rewrite; both equal `addr@ / spec_page_size()`, division needs no precondition — behaviour-preserving. No `// VERUS REWRITE` markers found; rewrites are described in bugs.md.

### Fix Request — BLOCKING FAIL: #10 (`make verify-kernel` does not build)

**Problem.** `frame.proof.rs:13` declares:
```rust
global size_of usize == 8;
```
The default verify target is 32-bit: `Makefile:11 export TARGET ?= x86`, and `verify-kernel` builds with `--target build/targets/x86-kernel.json` (`"target-pointer-width": 32`). After Verus verification, `cargo verus verify` runs rustc **codegen** for that target with `verus_keep_ghost` set, so the directive is present and its const-eval assertion panics:
`error[E0080]: evaluation panicked: does not have the expected size` at `frame.proof.rs:1:1` (the anonymous const `mm::phys::frame::_` the directive generates). Exit code 101 → `make verify-kernel` FAILS.

This is **not** a cosmetic codegen artifact. The proof's `alloc`/`alloc_contiguous` representability bound is discharged *only* under the false assumption `usize == 8`. On the actual default build target (`usize == 4`) the top managed frame (`idx = 0xFFFFF`) exceeds `FrameNumber::spec_max() = 0xFFFFE`, so the conversion can legitimately `Err` while a free frame exists — exactly the postcondition gap you recorded in `bugs.md` ("genuine representability gap on a 32-bit build"). So "112 verified" was obtained under a word-size assumption that does not hold for the compiled target.

**`bugs.md` marks this "ACCEPTED limitation". That is a justification, not a fix.** A non-zero `make verify-kernel` exit is a hard checklist failure regardless of rationale. Do not close this by editing `bugs.md`.

**Required change (make it target-agnostic; do NOT rely on `usize == 8`):**
1. Delete `global size_of usize == 8;` from `frame.proof.rs`.
2. Strengthen the allocator invariant `Inner::internal_inv` to carry the *correct* representability fact:
   `self.bitmap@.num_bits <= FrameNumber::spec_max() + 1`
   (equivalently every managed index `i < num_bits` satisfies `i <= FrameNumber::spec_max()`). This is the real fact established by `init` (`NFRAMES = MEMORY_SIZE / FRAME_SIZE`, small) and is the property `bugs.md` itself names as missing ("internal_inv is too weak; it does not capture num_bits <= spec_max() + 1"). Because `instance()` already `ensures (*r).inv()` and `init` is the allow-listed TCB boundary that constructs the singleton, this invariant is sound to assume from the trust boundary — and it holds on **both** 32- and 64-bit targets.
3. Discharge the `alloc`/`alloc_contiguous` `FrameNumber::from_raw_value`/`from_frame_number` success bound from that invariant instead of from the word size. Remove the now-unreachable `Err`-while-free reasoning that depended on `usize == 8`.

**Verify:** run `make verify-kernel` (default `TARGET=x86`) — must print `... verified, 0 errors` AND compile to exit 0 with no E0080 and no warnings. Then run `make verify-kernel` again with `TARGET=x86_64` to confirm both targets pass (cross-target robustness). Paste both exit codes.

If you genuinely believe step 2/3 is unprovable, do not assert it — produce an **isolated** Verus reproducer (checklist #7) showing the specific bound that fails from `num_bits <= spec_max()+1` alone, and we will reassess. A whole-function failure is not a reproducer.

### Items deferred until the build is fixed
- **#1 spec-drift:** the `#[verus_spec]` contracts and `frame.spec.rs` ensures must be re-diffed against the speccing-phase baseline once #10 is resolved. Preliminary concern: verifying under a false `usize==8` global means the current "no spec weakened" claim is not trustworthy for the real target — re-confirm after the invariant-based fix.
- **#7 limitation reproducer:** the only claimed limitation (the `global size_of` directive) is being challenged above; no isolated reproducer was provided, and the directive is not actually required (see fix). Provide a reproducer only if step 2/3 truly fails.
- **#9 cross-module regression (`make verify-kernel` over all modules):** blocked — cannot pass while the build errors out. Re-run after #10.

STOP not created: items #1, #7, #9, #10 are not yet PASS/FIXED.
