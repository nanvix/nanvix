## Turn 1: Full checklist walk-through (specification phase, `hal::platform::microvm`)

### Progress
- Done (PASS): all 16 checklist items below.
- Current: full first-pass review complete.
- Remaining: none.

### Scope
Per `verification-plan.json`, `caller_analysis.md`, and `view_design.md`, the single
in-scope exec function is the free function
`gva_to_gpa(gva: usize) -> usize` (`mod.rs:430`). Every other item in the module
(`init`, `parse_bootinfo`, MMIO/control helpers, `virt_to_phys`,
`is_valid_physical_address`, …) is explicitly out of scope and untouched.

The full module diff vs base (`verus-ai/hal-memory-region`) is purely additive:
- `mod.spec.rs`: new `pub open spec fn spec_gva_to_gpa(gva: int) -> int { gva }`.
- `mod.proof.rs`: new, empty (`verus! { }`).
- `mod.rs`: added `use vstd::prelude::*;` + `include!`s, and a `#[verus_spec]`
  `ensures result as int == spec_gva_to_gpa(gva as int)` on `gva_to_gpa`.
No exec body changed; no pre-existing spec touched.

### Verification (commands run)
- `make verify-kernel` → Exit code 0. `note: verifying module hal::platform::microvm`;
  build `Finished dev profile … in 0.23s` (compiles). Cross-module: run is
  `Module: <all>`, exit 0 → no regression in other verified modules.
- `grep -nE "admit|assume|external_body|trusted|#[verifier" mod.rs mod.spec.rs mod.proof.rs`
  → 0 cheating annotations in this module's files. (The repo-wide
  `admit=26 external_body=14` reported by the cheating check are all in OTHER
  modules — `mm/phys/*`, `hal/mem/*` — per `cheating-detail.txt`; none in
  `microvm`.)
- Read `caller_analysis.md` (sole caller `book_mmio_regions`, `mm/phys/mod.rs:114`)
  and `view_design.md` (pure stateless map, no View struct, `open` identity).

### Checklist results

1. **Every in-scope exec function has requires/ensures** — PASS. `gva_to_gpa` has
   an `ensures`; no `requires` needed (total/infallible, matches caller).
2. **Caller coverage** — PASS. Caller assumes total, infallible, deterministic,
   identity, frame-correspondence. `ensures result == gva` (via `spec_gva_to_gpa`)
   discharges all: totality (no `requires`), determinism (function of `gva`),
   frame-stepping & injectivity are immediate corollaries of identity.
3. **View consistency** — PASS. No stateful View by design (pure function); spec
   references the named map `spec_gva_to_gpa`, exactly as `view_design.md`
   prescribes. No `inv()` applies (no instance).
4. **No tautological ensures** — PASS. `result as int == spec_gva_to_gpa(gva as int)`
   constrains the result to the identity; not `true`/trivially-derivable.
5. **No subsumed ensures** — PASS. Single ensures; nothing redundant.
6. **Error paths have meaningful ensures** — PASS (N/A). Return type is `usize`,
   infallible; no `Result`/`Err` path exists.
7. **No assume_specification for workspace-internal code** — PASS. None present.
8. **vstd searched before assume_specification** — PASS (N/A). No
   `assume_specification` used.
9. **Specs written for the caller** — PASS. `result == gva` drops directly into
   `book_mmio_regions`' proof to show the booked GPA frame matches the MMIO GVA.
10. **Trait obligations satisfied** — PASS (N/A). Free function, no trait, no
    runtime dispatch.
11. **Spec completeness (advisory)** — PASS. Identity fully captures the abstract
    map; intentional totality matches caller expectation.
12. **Loop invariants** — PASS (N/A). `gva_to_gpa` body is `gva`; no loops in any
    in-scope code, spec, or proof.
13. **No cheating on module's own functions** — PASS. `admit=0 assume=0
    external_body=0 trusted=0` across `mod.rs`/`mod.spec.rs`/`mod.proof.rs`.
14. **No specs weakened (spec drift)** — PASS. Change is purely additive; the
    module had no prior specs (`mod.spec.rs` was `verus! { }`). No existing
    ensures/requires/spec-fn contract weakened anywhere in the diff.
15. **Bug awareness** — PASS. `gva_to_gpa` returning `gva` is correct for the
    MicroVM identity-mapped guest; no incorrect code found. No bugs recorded.
16. **Cross-module regression + Verification/build** — PASS. `make verify-kernel`
    exit 0 with `Module: <all>`; kernel compiles (cached dev build finished).

### Verdict
All 16 checklist items PASS with concrete evidence. No fix requests. Writing
`STOP = RESOLVED`.
