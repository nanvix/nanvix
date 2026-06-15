## Turn 1: Full checklist walkthrough — proving phase, sys-virt-address

### Progress
- Done (PASS): all 10 checklist items verified PASS with concrete evidence.
- Current: full walkthrough complete.
- Remaining: none.

### Verification summary

Baselines (from `pipeline_state.json`):
- specification END / proving START SHA: `819d903e2c7eedee38667f48ccc9ab5cb4163311`
- HEAD: `337a77248e0858c6189997e09621a5562500bd0a`
- Working tree status for the module dir: **clean** (`git status --porcelain` empty).
- `git diff 819d903..HEAD -- virt.rs virt.spec.rs virt.proof.rs`: **empty** — the
  proving phase introduced **zero source changes**. The two in-scope inherent
  functions (`new`, `from_raw_value`) were already body-verified at spec END, so
  proving had nothing to add.

---

#### 1. No specs weakened — **PASS**
- `spec_drift.py git-diff --before 819d903 --after HEAD --file-stem virt` →
  **0 contract drift** (exit 0).
- `--before 819d903 --after 819d903` → 0 drift.
- The default working-tree mode reported "2× ensures removed on
  `VirtualAddress::from_raw_value`". Investigated and **disproved**: the source is
  byte-identical to HEAD (clean status) and HEAD vs baseline shows 0 drift. The
  false positive is a tool artifact from the **duplicate function name**
  `from_raw_value` (inherent `#[verus_spec]` version at virt.rs:71–78 + the
  spec-less trait-impl version at virt.rs:188–190); the working-tree code path
  pairs them inconsistently. Authoritative committed-ref comparison = **0 drift**.

#### 2. Zero remaining admit() — **PASS**
- Fresh `make verify-sys` (forced recompile): `admit=0`. No `admit(` in `src/libs/sys/`.

#### 3. Zero external_body unless in TCB list — **PASS**
- `sys` crate: `external_body=0` (fresh verify). `grep -rn external_body src/libs/sys/`
  → only a doc-comment mention at virt.rs:266, no actual attribute.
- (Pre-existing `external_body` in `kernel` (25) and `bump-allocator` (2) are
  governed by `verus-ai-logs/tcb-allowed.md` and are not part of this proving
  target; baseline→HEAD diff touched only log files, not those crates.)

#### 4. Zero assume/assume_specification — **PASS**
- `sys` crate: `assume=0`; no `assume_specification` anywhere under `src/libs/sys/`.
- The `into_raw_value` newtype-identity fact is held as a **consumer-side**
  `assume_specification` in the *kernel* crate (`phys.spec.rs`), at the sys/kernel
  library edge, listed in `tcb-allowed.md`. It is outside this module's TCB,
  pre-existing, and not introduced by proving.

#### 5. No cfg-gated exec code — **PASS**
- Cheating detector: `cfg_gate=0`.
- cfg sites in the module: `#[cfg(verus_keep_ghost)]` at virt.rs:9,11 (standard
  ghost-include of `virt.spec.rs`/`virt.proof.rs`, not exec code);
  `#[cfg(target_pointer_width = "32")]` at virt.rs:39 (a `static_assert!`
  size check) and virt.rs:308 (`impl From<VirtualAddress> for u32`, platform
  conditional compilation). None are verus-conditional exec branches/expressions/
  match arms. No source change in proving.

#### 6. Cheating audit (exact counts) — **PASS**
- Fresh `make verify-sys`: `assume=0 external_body=0 admit=0 trusted=0
  no_decreases=0 cfg_gate=0`. Status: **CLEAN**. No locations to report.

#### 7. Claimed Verus limitation has an isolated reproducer — **PASS**
- Claim (`verus-unsupported.md`): `<VirtualAddress as Address>::into_raw_value`
  cannot be body-verified because (a) Verus requires the *entire* trait impl to be
  verified, and (b) doing so pulls sibling `as_ptr`/`as_mut_ptr` (`usize as *const
  u8`) into scope, which Verus rejects.
- I built and ran **isolated reproducers** (not the full module):
  - `fn as_ptr(x: usize) -> *const u8 { x as *const u8 }` inside `verus!` →
    `error: Verus does not support this cast: usize to *const u8`.
  - Minimal `impl Addr for VA` with one `#[verus_spec]` method + one
    pointer-cast method → `error: In order to verify any items of this trait impl,
    the entire impl must be verified.`
- Both claimed limitations reproduce minimally. Limitation is **genuine**.

#### 8. Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`) — **PASS**
- No `// VERUS REWRITE` comments in `src/libs/sys/`. Proving introduced no source
  changes, so no exec body was rewritten. Nothing to audit.

#### 9. Cross-module regression (`make verify`) — **PASS**
- `make verify` (bitmap, sys, nanvix-slab, bump-allocator, kernel): every crate
  **exit 0**. `sys` status CLEAN. Pre-existing cheating flags in kernel/bump are
  TCB-governed and unchanged by this phase.

#### 10. Verification + build, 0 errors / 0 warnings — **PASS**
- Fresh `make verify-sys`: **6 verified, 0 errors**, CLEAN.
- `./z build -- all`: **exit 0**, "[OK] Build complete.", no Rust compiler
  warnings/errors (only a benign tooling note: "Sysroot directory ... not found;
  skipping symlink update").

### Fix Request
None. All 10 checklist items PASS with verified evidence. The proving phase is a
clean no-op over already-verified specs; the one in-scope unverifiable function
(`into_raw_value`) is a genuine, independently-reproduced Verus front-end
limitation handled by a pre-existing, TCB-listed consumer-side trust boundary.

### Reviewer note (non-blocking)
`verus-unsupported.md` documents the limitation using errors emitted from the full
module compile. Consider adding the two minimal isolated reproducers above to that
entry for future auditors. This is a documentation nicety, not a proving defect.
