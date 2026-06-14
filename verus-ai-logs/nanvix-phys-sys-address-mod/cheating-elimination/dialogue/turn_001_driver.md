## Turn 1: Full checklist walkthrough — sys-address-mod cheating elimination

### Progress
- Done (PASS/FIXED with evidence): all 12 checklist items (see Verification).
- Current: full-checklist verification.
- Remaining: none.

### Scope
- Target: `src/libs/sys/src/sys/mm/address/mod.rs` (+ companion `mod.spec.rs`, `mod.proof.rs`).
- In-scope functions: `is_aligned`, `into_raw_value`, `from_raw_value`.
- Phase change under review (cheating-elimination start `aa3991bc` → HEAD):
  the ONLY source change is a single line in `src/libs/sys/src/sys/mm/alignment.rs`:
  removal of a redundant `#[cfg(verus_keep_ghost)]` attribute from a spec-only
  `verus! { pub open spec fn spec_align_value(..) }` block (the sole cfg-gated-exec
  item flagged by the cheating scanner in the whole `sys` crate).

### Verification

**1. Zero admit() — PASS.**
`make verify-sys` summary: `admit=0`. Whole-crate scan, status CLEAN.

**2. Zero assume() — PASS.**
`make verify-sys` summary: `assume=0`.

**3. Zero trusted functions — PASS.**
`make verify-sys` summary: `trusted=0`.

**4. Zero exec_allows_no_decreases_clause — PASS.**
`make verify-sys` summary: `no_decreases=0`.

**5. Zero cfg-gated exec — PASS (FIXED).**
Was `cfg_gate=1` (the redundant gate on the `spec_align_value` block in
`alignment.rs:151`). Fix removed the attribute; bare `verus! { … }` now matches the
established same-crate convention (`address/virt.rs` declares `impl View for
VirtualAddress` in a bare `verus!` block). `make verify-sys` summary now `cfg_gate=0`,
status CLEAN. The `verus!` macro strips `spec fn` items in non-verus builds, and
`spec_align_value` has zero exec references:
`grep -rn spec_align_value src/ | grep -v '\.spec\.rs' | grep -v 'spec fn'` → empty.

**6. Zero external_body unless listed in tcb-allowed.md — PASS.**
`make verify-sys` summary: `external_body=0`. No external_body anywhere in the `sys`
crate, so nothing to reconcile against `verus-ai-logs/tcb-allowed.md`.

**7. AST consistency: zero mismatches — PASS.**
- `address/mod.rs` (target): `ast_consistency.py count` → `✅ Consistent: 0 functions,
  0 structs match` (trait-only file, no exec bodies).
- `alignment.rs` against the canonical module baseline (branch base
  `verus-ai-prove-bottom-up`): `✅ Consistent: 12 functions, 0 structs match` (EXIT 0).
- A stricter intra-phase run (base `aa3991bc`) reports a single `MISMATCH` on
  `Alignment::try_from`. **This is a tool name-collision artifact, not a real change:**
  the file defines FOUR `impl TryFrom<…> for Alignment` blocks (`usize`, `u8`, `u16`,
  `u32`), all yielding the qualified name `Alignment::try_from`; the checker de-dups by
  name and ends up comparing the `u32` impl against the `u16` impl. Ground truth from
  `git diff aa3991bc..HEAD -- alignment.rs` is a **single-line** cfg-attribute deletion
  on a spec-only block — all four `try_from` exec bodies are byte-identical. Exec AST is
  provably unchanged.

**8. All exec rewrites have VERUS REWRITE comment + minimal reproducer — PASS (N/A).**
No exec code was rewritten. The only change is deletion of a cfg attribute on a
spec-only block; no exec statement/signature was touched (confirmed by the one-line
git diff and by item 7).

**9. Each surviving external_body listed in tcb-allowed.md — PASS (N/A).**
Zero external_body in the `sys` crate (item 6).

**10. No specs weakened — PASS.**
`spec_drift.py check sys-address-mod` (baseline `aa3991bc` → HEAD):
`Contract drift: 0`, `Ensures removed: 0`, `Requires added: 0` → `✅ No contract drift
detected.` (EXIT 0). The `Address` trait's `#[verus_spec]` ensures on
`from_raw_value`/`into_raw_value`/`clone_address`/`is_aligned`, and
`spec_addr_is_aligned`, are intact.

**11. Cross-module regression — PASS.**
`make verify` (bitmap, sys, nanvix-slab, bump-allocator, arch, kernel): **every crate
Exit code 0** — verification succeeds workspace-wide. `sys` is CLEAN
(`cfg_gate=0`). The `CHEATING_DETECTED` statuses on the other crates (bitmap
`cfg_gate=5`, nanvix-slab `cfg_gate=1`, bump-allocator `external_body=2`, arch
`external_body=3`, kernel `admit=31/external_body=11/cfg_gate=15`) are **pre-existing**
items owned by those modules' own pipeline stages — they are not regressions: the
single-line `alignment.rs` change is confined to the `sys` crate and introduced no new
cheating anywhere.

**12. Verification + build — PASS.**
- `make verify-sys`: Exit code 0, status CLEAN,
  `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- `make check-guest-rlib-sys` (non-verus normal build): Exit 0, **0 warnings, 0
  errors** (scanned compiler-message JSON: no `warning`/`error` level messages). This
  confirms removing the cfg gate does not break the normal (`verus_keep_ghost`-off)
  build.

### Verdict
All 12 checklist items PASS / FIXED with tool evidence. No fix requests. Module
sys-address-mod cheating-elimination is RESOLVED.
