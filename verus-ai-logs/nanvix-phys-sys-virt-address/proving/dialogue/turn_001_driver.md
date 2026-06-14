## Turn 1: Full Proving Checklist Walkthrough

### Progress
- Done (all PASS): spec-drift, zero-admit, zero-external_body, zero-assume,
  no-cfg-gated-exec, cheating-audit, verus-limitation-reproducer,
  exec-rewrites, cross-module-regression, verification+build.
- Current: completed all items.
- Remaining: none.

### Verification

Commands run from repo root `/home/ruize/nanvix-phy-specs-bottom-up`.

Note on layout: `virt.spec.rs` and `virt.proof.rs` are empty placeholders
(`verus! { }`). All contracts for this module are inline in `virt.rs` via
`#[verus_spec]` / `#[verus_verify]`, plus the trait-declaration contract for
`into_raw_value` in `mod.rs`.

**1. No specs weakened (spec-drift)** — PASS
`python3 .../scripts/spec_drift.py check nanvix-phys-sys-virt-address`
(baseline = proving start `fe53d18963cf`, after = HEAD):
```
Functions with changes: 0
Contract drift (⚠ review required): 0   (ensures removed: 0, requires added: 0)
Proof drift: 0 | added: 0 | removed: 0
✅ No contract drift detected.
```

**2. Zero remaining admit()** — PASS
`make verify-sys` summary: `admit=0`. Grep over virt.{rs,spec.rs,proof.rs}: none.

**3. Zero external_body unless in tcb-allowed.md** — PASS
`make verify-sys` summary: `external_body=0`. Grep: none. (No tcb entry needed.)

**4. Zero assume / assume_specification** — PASS
`make verify-sys` summary: `assume=0`. Grep for `assume`/`assume_specification`
over all three files: none.

**5. No cfg-gated exec code** — PASS
`make verify-sys` summary: `cfg_gate=0`. The only `#[cfg(...)]` in virt.rs are:
lines 9/11 `cfg(verus_keep_ghost)` (standard spec/proof include guards) and
lines 39/296 `cfg(target_pointer_width="32")` (platform-conditional
`static_assert` + `From<VirtualAddress> for u32` impl). The cheating checker
does not count these as exec-evasion cfg gates (`cfg_gate=0`).

**6. Cheating audit (exact counts/locations)** — PASS
Module `sys` (this target), `make verify-sys`:
`assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0` →
status: CLEAN. No violating functions exist; nothing to challenge individually.

**7. Claimed Verus limitation has isolated reproducer** — PASS
`verus-unsupported.md` documents the genuine front-end limits blocking an
impl-level spec on `<VirtualAddress as Address>::into_raw_value`, each isolated
to the specific construct:
- `usize as *const u8` / `usize as *mut u8` cast (as_ptr/as_mut_ptr, exact
  per-cast errors quoted).
- all-or-nothing trait-impl rule cascading to non-verus inherent calls (exact
  per-call errors quoted).
Resolution: contract is carried at the trait declaration
(`mod.rs:51-55`, `ensures result as int == self@`), so no admit/external_body/
assume was introduced — crate stays clean.

**8. Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`)** — PASS
Grep for `VERUS REWRITE` / `VERUS` in virt.rs: none. No exec rewrites were
performed (consistent with verus-unsupported.md "no exec rewrite").

**9. Cross-module regression (`make verify`, all crates)** — PASS
All crates exit 0 (verification passes):
```
bitmap        exit 0   (cfg_gate=5)        pre-existing, out of scope
sys           exit 0   CLEAN               <-- this module
nanvix-slab   exit 0   (cfg_gate=1)        pre-existing, out of scope
bump-allocator exit 0  (external_body=2)   pre-existing, in tcb-allowed
arch          exit 0   (external_body=3)   pre-existing, in tcb-allowed
kernel        exit 0   (external_body=11 admit=27 cfg_gate=14) pre-existing
```
Requirement is "all verified modules must pass" = exit 0; all pass. Residual
cheating in other crates is pre-existing and outside this module's scope.

**10. Verification + build (0 errors, 0 warnings)** — PASS
- `make verify-sys`: Exit code 0, status CLEAN.
- `./z build -- all`: `sys v0.16.17` compiles, all crates `Finished`, no rustc
  errors/warnings. The single `Warning: Sysroot directory ... not found` is a
  `z`-utility symlink note (environment), not a compiler diagnostic on this code.

### Fix Request
None — every checklist item is PASS with tool evidence. No fixes required.
