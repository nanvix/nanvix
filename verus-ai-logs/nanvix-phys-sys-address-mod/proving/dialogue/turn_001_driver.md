## Turn 1: Full checklist walk-through — sys-address-mod (proving)

### Progress
- Done (PASS): all 10 checklist items verified PASS with evidence below.
- Current: full first-pass review.
- Remaining: none.

### Scope
Module under review: `sys::sys::mm::address` (sys-address-mod).
Files in scope:
- `src/libs/sys/src/sys/mm/address/mod.rs`
- `src/libs/sys/src/sys/mm/address/mod.spec.rs`
- `src/libs/sys/src/sys/mm/address/mod.proof.rs`
- `src/libs/sys/src/sys/mm/address/virt.rs`
- `src/libs/sys/src/sys/mm/address/virt.spec.rs`
- `src/libs/sys/src/sys/mm/address/virt.proof.rs`

### Verification commands run
- `make verify-sys` (cached): exit 0.
- `make verify-sys` (fresh, after `touch mod.rs`): **6 verified, 0 errors, exit 0, 0 warnings.**
- Crate-wide cheating scan: `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=1`.
- Module-scoped pattern audit:
  `grep -rnE "admit\(|external_body|assume\b|assume_specification|no_decreases|#\[verifier::external" src/libs/sys/src/sys/mm/address/` → **NONE FOUND**.

---

### Item-by-item results

**1. No specs weakened — PASS**
`git status`/`git diff HEAD` on the module: clean (committed). `git log -p --follow` on `mod.rs`
shows specs were only **added/strengthened** during proving, never removed or weakened:
- `from_raw_value`: `Ok(a) => a@ == raw_addr as int`, `Err(e) => e.code == ErrorCode::BadAddress` (added).
- `into_raw_value`: `result as int == self@` (added).
- `clone_address`: `result@ == self@` (added).
- `is_aligned`: `Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)` (added).
- `spec_addr_is_aligned` (mod.spec.rs): `v % spec_align_value(align) == 0` — exact declarative meaning.
No ensures clause was deleted or relaxed. Functions without contracts (`align_up`, `align_down`,
`max_addr`, `as_ptr`, `as_mut_ptr`) never had contracts — absence is not weakening.

**2. Zero remaining admit() — PASS**
Module audit: 0 `admit`. Fresh crate cheating scan: `admit=0`.

**3. Zero external_body unless in TCB file — PASS**
Module audit: 0 `external_body`. The only external marker in scope is
`#[verus_verify(external_derive)]` on `VirtualAddress` (virt.rs:35), which governs the
`#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]` and is **not** `external_body`.
Crate scan: `external_body=0`.

**4. Zero assume/assume_specification — PASS**
Module audit: 0 `assume`, 0 `assume_specification`. Crate scan: `assume=0`.

**5. No cfg-gated exec code (branches, expressions, match arms) — PASS (for this module)**
The module contains two item-level `#[cfg(target_pointer_width = "32")]` gates:
- virt.rs:39 — `static_assert::assert_eq_size!` (compile-time assertion).
- virt.rs:296 — `impl From<VirtualAddress> for u32`.
These are **item-level platform conditionals**, not exec branches/expressions/match arms.
The verification target is 32-bit (`build/targets/x86-kernel.json` → `target-pointer-width: 32`),
so **both gated items are part of the verified build** — no exec code is hidden from Verus.
The `#[cfg(verus_keep_ghost)]` gates in `mod.rs`/`virt.rs` precede `include!` only.

**6. Cheating audit — exact counts/locations — PASS**
For sys-address-mod:
- `admit` = 0
- `external_body` = 0
- `assume` / `assume_specification` = 0
- `no_decreases` = 0
- cfg-gated **exec** code (`verus_keep_ghost` kind) = 0
- Item-level `target_pointer_width` gates = 2 (virt.rs:39, virt.rs:296) — verified at 32-bit, benign.
Note: the crate-wide `cfg_gate=1` reported by `make verify-sys` is located at
`src/libs/sys/src/sys/mm/alignment.rs:151` — `#[cfg(verus_keep_ghost)] verus! { ... }` gating
**pure spec functions** (`spec_align_value`), i.e. ghost code, in the **alignment** module.
That is out of scope for sys-address-mod and is a checker heuristic flag on spec-gating, not exec-gating.

**7. Claimed Verus limitation isolated reproducer — PASS (N/A)**
No Verus limitation is claimed. `mod.proof.rs` and `virt.proof.rs` are empty stubs
(`verus! { } // verus!`). Nothing to reproduce.

**8. Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`) — PASS (N/A)**
`grep "VERUS REWRITE"` → none. Exec code is unchanged (git clean vs HEAD); no rewrites performed.

**9. Cross-module regression (`make verify-sys`, all modules) — PASS**
Fresh `make verify-sys`: **6 verified, 0 errors, exit 0**. All modules pass.
(The `status: CHEATING_DETECTED` line is driven solely by the alignment.rs spec-gating
`cfg_gate=1` above; it does not represent a verification failure and is not in scope here.)

**10. Verification + build, 0 errors / 0 warnings — PASS**
`make verify-sys`: 6 verified, 0 errors, exit 0. Verus log warning count = 0.
Cargo build step (`Finished dev profile`) completed with no errors/warnings.

### Fix Request
None. Every checklist item is PASS with concrete tool evidence. No code changes required
for sys-address-mod.

### Result
RESOLVED.
