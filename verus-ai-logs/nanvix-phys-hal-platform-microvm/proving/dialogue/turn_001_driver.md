## Turn 1: Full checklist walkthrough — proving phase, hal-platform-microvm

### Progress
- Done (all PASS): every checklist item below.
- Current: completed full review.
- Remaining: none.

### Module under review
- Source: `src/kernel/src/hal/platform/microvm/mod.rs`
- Spec:   `src/kernel/src/hal/platform/microvm/mod.spec.rs`
- Proof:  `src/kernel/src/hal/platform/microvm/mod.proof.rs`

### Verification commands run
- `make verify-kernel` → Exit code 0. Summary: `status: CHEATING_DETECTED` is **global**
  (`external_body=19 admit=12 cfg_gate=19`), all located in `mm/phys/*` and `mm/virt/*`
  (see `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`). **None in microvm.**
- `MODULE=hal::platform::microvm make verify-kernel` → Exit code 0,
  **`✅ No cheating detected in module hal::platform::microvm`**, `status: CLEAN`.
- Commit `194b299b8` records `verify PASS: kernel::hal::platform::microvm (1 verified, 0 errors)`.

---

### Item-by-item

**1. No specs weakened — PASS**
Compared `b899eda4e` (caller-analysis START) → `194b299b8` (verify PASS).
At START both spec and proof were empty (`verus! { } // verus!`). The proving phase added a
**net-new** spec; nothing pre-existing was weakened. New spec:
```
pub open spec fn spec_gva_to_gpa(gva: int) -> int { gva }
```
and contract on the exec fn `gva_to_gpa`:
```
#[verus_spec(result => ensures result as int == spec_gva_to_gpa(gva as int))]
```
This is exact functional correctness (identity), the strongest possible spec for this function.
`open` visibility preserves the identity as a caller-visible guarantee (frame correspondence /
injectivity are corollaries). No drift.

**2. Zero admit() — PASS**
Module scan: `admit=0`. `cheating-detail.txt` lists 12 admits, all in `mm/phys/manager.proof.rs`,
`mm/virt/identity_map.{proof.rs,rs}` — none in microvm. grep of the module for `admit`: none.

**3. Zero external_body unless in TCB-allowed — PASS**
Module scan: `external_body=0`. The 19 global external_body are all in `mm/phys/*` — none in
microvm. grep of the module: none.

**4. Zero assume / assume_specification — PASS**
Module scan: `assume=0`. grep of the module: none.

**5. No cfg-gated exec code — PASS**
`count_cfg_gates` (scripts/verify.sh:478) flags only `verus_keep_ghost`-gated items (the cheating
concern), excluding `include!`/`use`/`mod`/log macros. The module's only `verus_keep_ghost` cfgs
(mod.rs:9,11) gate `include!("mod.spec.rs")` / `include!("mod.proof.rs")` — both excluded.
Module `cfg_gate=0`. The remaining `#[cfg(feature = "whp"/"pit"/"smp"/"stdio")]` attributes are
ordinary platform conditional compilation, not Verus-hiding gates.

**6. Cheating audit (exact counts + locations) — PASS**
Module `hal::platform::microvm`: `admit=0, external_body=0, assume=0, cfg_gate=0` → `status: CLEAN`.
Global (out of scope, for the record): `external_body=19, admit=12, cfg_gate=19`, all in
`mm/phys/*` and `mm/virt/*` per `cheating-detail.txt` (31 entries, zero referencing microvm).

**7. Claimed Verus limitation has isolated reproducer — N/A (PASS)**
Proof file is empty; no limitation claimed, no admit/workaround used.

**8. Exec rewrites minimal & semantically equivalent — PASS**
No `// VERUS REWRITE` comments in the module. The only exec change is the added
`#[verus_spec(...)]` ensures attribute on `gva_to_gpa`; the body is unchanged (`gva`), which
exactly matches `spec_gva_to_gpa(gva) = gva`. Semantically equivalent.

**9. Cross-module regression — PASS**
Full `make verify-kernel` → Exit code 0; all verified modules cached/pass, 0 errors.

**10. Verification + build: 0 errors, 0 warnings — PASS**
`make verify-kernel` Exit 0; module 1 verified / 0 errors; verify log shows 0 warnings.
The kernel exec code was compiled under verification via cargo `-Z build-std` against
`x86-kernel.json` (`Finished dev profile`, 0 warnings, 0 errors) — authoritative kernel build.
(Note: `./z build -- kernel` errors with "No rule to make target 'kernel'" — a build-harness/target
name quirk, not a code defect; no standalone `kernel` make target exists. The verus build-std
compilation is the authoritative kernel compile and is clean.)

### Fix Request
None. All checklist items PASS with tool evidence. Module is CLEAN and fully verified.
