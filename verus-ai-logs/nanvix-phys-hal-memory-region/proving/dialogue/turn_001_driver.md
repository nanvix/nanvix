## Turn 1: Full checklist sweep — hal-memory-region (proving)

### Progress
- Done (all PASS):
  1. No specs weakened
  2. Zero remaining admit()
  3. Zero external_body unless TCB-listed
  4. Zero assume/assume_specification
  5. No cfg-gated exec code
  6. Cheating audit (exact counts)
  7. Claimed Verus limitation reproducers
  8. Exec rewrites minimal / `// VERUS REWRITE`
  9. Cross-module regression
  10. Verification + build (0 errors, 0 warnings)
- Current: none (sweep complete)
- Remaining: none

### Verification

Scope files:
- `src/kernel/src/hal/mem/types/region.rs`
- `src/kernel/src/hal/mem/types/region.spec.rs`
- `src/kernel/src/hal/mem/types/region.proof.rs`

**1. No specs weakened — PASS.**
`spec_drift.py check nanvix-phys-hal-memory-region` (baseline `32eecbc26bb0` → HEAD):
Functions with changes 0, Contract drift 0, Ensures removed 0, Requires added 0.
`git diff 32eecbc26bb0 HEAD --` on all three files returns **empty** — the spec
surface is byte-identical to the proving baseline. No weakening possible.
The `ensures` on `start()`/`size()` (region + truncated) and the `MemoryRegionView`
/`inv()`/`spec_set_cache_policy` contracts in `region.spec.rs` are intact.

**2. Zero admit() — PASS.**
`cheating-detail.txt` lists 12 admits, all in `mm/phys/manager.proof.rs`,
`mm/virt/identity_map.{proof.rs,rs}` — **none** in any region file.
Direct grep of the three region files for `admit` returns nothing.

**3. Zero external_body unless TCB-listed — PASS.**
`cheating-detail.txt` external_body entries are all in `mm/phys/*` — **none** in
region files. The three `#[verus_verify(external_derive)]` attributes (lines 65,
143, 298) and `#[verus_verify]` (line 43) apply to `#[derive(...)]` trait impls,
not to logic/exec function bodies; `external_derive` is the standard Nanvix derive
shim and is not counted as `external_body` (confirmed: no region entry in the
cheating detail). No TCB exception needed.

**4. Zero assume/assume_specification — PASS.**
Global `assume=0`. Grep of region files for `assume`/`assume_specification`: none.

**5. No cfg-gated exec code — PASS.**
Only `cfg` occurrences in scope are lines 9 & 11:
`#[cfg(verus_keep_ghost)] include!("region.spec.rs"/"region.proof.rs")`.
These gate ghost spec/proof module includes — not exec branches, expressions, or
match arms. This is the standard spec/proof inclusion pattern. No exec cfg gating.

**6. Cheating audit (region module exact counts) — PASS.**
- admit: 0 (locations: none in region.rs/spec.rs/proof.rs)
- external_body: 0
- assume: 0
- cfg-gated exec code: 0 (the 2 `#[cfg(verus_keep_ghost)]` at region.rs:9,11 gate
  ghost includes only)

**7. Claimed Verus limitation reproducers — PASS (N/A).**
`region.proof.rs` is empty (`verus! { }`); no limitation is claimed and no proof
workaround exists, so no isolated reproducer is owed.

**8. Exec rewrites minimal / semantically equivalent — PASS (N/A).**
Grep for `// VERUS REWRITE` in region.rs: none. No exec rewrites were introduced.

**9. Cross-module regression — PASS.**
`make verify-kernel` (Module: `<all>`, crate kernel) → Exit code 0, all modules
verified/cached, 0 errors. No regression in any verified module.

**10. Verification + build — PASS.**
- `make verify-kernel`: Exit 0, `verification: cached (no recompilation), — (exit 0)`.
- `./z build -- all`: `[OK] Build complete.`, 0 compiler errors, 0 compiler
  warnings. (The single `Warning: Sysroot directory ... not found; skipping symlink
  update` is a benign build-script notice, unrelated to compilation of this module.)

### Fix Request
None. Every checklist item passes with tool-backed evidence. The proving phase made
no changes to the scope files (identical to baseline), specs are intact and
unweakened, there are zero admit/external_body/assume/cfg-exec cheats in scope,
cross-module verification is green, and the full build is clean.

**Result: RESOLVED.**
