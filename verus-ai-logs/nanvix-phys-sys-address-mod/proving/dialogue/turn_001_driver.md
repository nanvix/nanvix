## Turn 1: Full checklist walkthrough — proving phase, `sys::mm::address` (`mod.rs`)

### Progress
- Done (PASS): 1 (no spec drift), 2 (no admit), 3 (no external_body), 4 (no
  assume), 5 (no cfg-gated exec), 6 (cheating audit), 7 (Verus-limitation
  reproducer N/A), 8 (exec rewrites N/A), 10 (verify-sys + build)
- Item 9 (cross-module `make verify`): FAILS, but the failure is a pre-existing
  **environmental** `vstd`/toolchain incompatibility in the unlisted `bitmap`
  crate — **no regression is introduced by this phase** (zero source changes).
- Current: documenting item 9.
- Remaining: none — every item verified.

### Verification

Scope reminder: the in-scope module is a **pure trait-declaration** file. The
three target methods (`is_aligned`, `into_raw_value`, `from_raw_value`) are
trait method *declarations* with `#[verus_spec]` contracts and **no bodies**, so
they carry no proof obligations here; `mod.proof.rs` is an empty `verus! { }`.

Baseline / current state:
- `pipeline_state.json`: phase = `proving`, proving baseline SHA = `d82e2042327f`.
- `git diff --stat d82e2042327f HEAD -- Cargo.toml Cargo.lock src/` → **empty**
  (zero source/dependency changes in the proving phase; the contracts were
  finalized in the specification phase and proving confirmed they verify).

Item-by-item evidence:

1. **No specs weakened.**
   `python3 scripts/spec_drift.py check nanvix-phys-sys-address-mod`
   → Functions with changes: 0; Contract drift: 0 (ensures removed 0, requires
   added 0). **✅ No contract drift.** PASS.

2. **Zero admit().** `grep -rn 'admit' address/` → only a prose mention inside a
   comment in `mod.spec.rs:33`. Cheating check: `admit=0`. PASS.

3. **Zero external_body (unless in `verus-ai-logs/tcb-allowed.md`).**
   `grep -rn 'external_body' address/` → only a comment in `mod.spec.rs:33`.
   Cheating check: `external_body=0`. No function in this module uses it. PASS.

4. **Zero assume / assume_specification.** `grep -rn 'assume' address/` → only
   comments (`mod.rs:71`, `virt.rs:266`). Cheating check: `assume=0`. PASS.

5. **No cfg-gated exec code.** The only `#[cfg(...)]` in `mod.rs` are lines 9 &
   11: `#[cfg(verus_keep_ghost)] include!("mod.spec.rs" / "mod.proof.rs")` —
   these gate **ghost** (spec/proof) code, the standard pattern, not exec
   branches/expressions/match-arms. Cheating check: `cfg_gate=0`. PASS.

6. **Cheating audit (exact counts + locations).** From
   `verus-ai-logs/verify-sys/verus-logs/verus_2026-06-15_18-43-22.log`:
   `assume=0  external_body=0  admit=0  trusted=0  no_decreases=0  cfg_gate=0`.
   Locations: none. Status: CLEAN. PASS.

7. **Claimed Verus limitation has isolated reproducer.** `bugs.md` documents one
   limitation — the definition cycle when `spec_addr` is bounded by `Address`
   (`error: found a cyclic self-reference`). It was **resolved by spec design**
   (unbounded generic `spec_addr<T>` / `addr_inv<T>`), not papered over with
   `admit`/`assume`/`external_body`. The construct is isolated to the
   `uninterp spec fn spec_addr<T>` declaration. No outstanding limitation relies
   on a cheat. PASS (no unresolved limitation).

8. **Exec rewrites minimal / `// VERUS REWRITE`.** `grep -rn 'VERUS REWRITE'`
   → none. The proving phase made **zero source changes** (logs-only diff) and
   the module has no exec bodies to rewrite. PASS (N/A).

9. **Cross-module regression: `make verify`.** Result: **FAILS at the first
   crate, `bitmap`**, with exit 101:
   ```
   error: expected generics to match: expected / found i16 ...
     --> ~/.cargo/registry/.../vstd-0.0.0-2026-05-31-0205/std_specs/atomic.rs:16
   error: could not compile `vstd` (lib) due to 9 previous errors
   verification: 0 verified, compilation/setup error (verus did not run)
   ```
   This is an **environmental** failure, not a proving-phase defect:
   - The error is entirely inside the third-party registry crate
     `vstd-0.0.0-2026-05-31-0205` (pinned at workspace `Cargo.toml:231`,
     `=0.0.0-2026-05-31-0205`). The installed toolchain is verus
     `0.2026.06.14` (Rust nightly `1.96.0`). The pinned vstd snapshot fails to
     **recompile** under the newer nightly (`atomic_specs_*` macro mismatch).
   - It only manifests when vstd must be rebuilt (cache miss). Git history shows
     this exact `bitmap` "compilation/setup error" **alternating FAIL/PASS**
     (`dcce27364` FAIL → `c7501fc18` PASS (70 verified) → ... → `803890e4f`
     FAIL) — classic cache/toolchain flakiness, independent of source.
   - **This phase introduced it neither directly nor indirectly**:
     `git diff --stat d82e2042327f HEAD -- Cargo.toml Cargo.lock src/` is empty.
     `bitmap` is an unlisted crate the prover was instructed not to touch.
   - The in-scope crate **`sys` verifies cleanly**: fresh (non-cached) runs at
     18:32/18:38/18:40 each report `6 verified, 0 errors`.
   Conclusion: **no regression is attributable to the sys-address-mod proving
   work.** The cross-module verify cannot run to completion in this environment
   because of a toolchain/`vstd`-pin drift in `bitmap`, which is out of scope
   for this phase and is **not** a specification defect (so ROLLBACK does not
   apply). Verified deterministically (two runs, 18:44 and 18:46 — identical).

10. **`make verify-sys` + build, 0 errors / 0 warnings.**
    `make verify-sys` → exit 0, status CLEAN, `6 verified, 0 errors`, no
    warnings emitted. The module is pure trait declarations, compiled by verus
    as part of the `sys` crate during verify-sys (which exercises the exec
    surface); no separate exec build defects exist. PASS.

### Fix Request
None for items 1–8 and 10 — all PASS with tool evidence.

Item 9 is a **pre-existing environmental blocker** (registry `vstd` pin vs
toolchain nightly) in the unlisted `bitmap` crate, with zero source changes in
this phase and a clean-verifying `sys` crate. It is not fixable within the
sys-address-mod proving scope and is not a specification defect, so it is
recorded as a no-regression environmental exception rather than a proving fix
request. Remediation (bumping the workspace `vstd` pin to match the toolchain
and re-locking) belongs to a separate, workspace-wide task — not this phase.

### Verdict
All proving-scope checklist items PASS. The sole failing item (cross-module
`make verify`) fails for a documented, pre-existing, environmental reason
external to and not caused by this phase; the `sys` crate (this module's crate)
verifies with 0 errors and 0 cheating. Proving phase work is complete and sound.
