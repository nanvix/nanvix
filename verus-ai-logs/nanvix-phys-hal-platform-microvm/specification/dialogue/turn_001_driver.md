## Turn 1: Full checklist walkthrough — FAIL on "No cheating on module's own functions"

### Progress
- Done (PASS): items 1–12, 14, 15, 16, 17 (see Verification below).
- Current (FAIL): item 13 — `admit()` in the module's own proof fn `lemma_translation_injective`.
- Remaining: none unverified — every checklist item was checked this turn.

### Scope
In-scope target for this verification order is the single free function
`gva_to_gpa(gva: usize) -> usize` (`mod.rs:436`). All other 27 exec functions
in the module are explicitly out of scope per `caller_analysis.md` and the task
hard rules, and were not modified.

### Verification (commands run, evidence)

**`make verify-kernel`** → Exit code 0. Module `hal::platform::microvm` verifies.
```
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=25 admit=1 trusted=0 no_decreases=0 cfg_gate=7
coverage: 48/1022 exec functions have contracts
status: CHEATING_DETECTED
```

**Cheating attribution** (`verus-logs/cheating-detail.txt`) — *which* functions cheat:
- `hal/platform/microvm/mod.proof.rs:19 lemma_translation_injective: admit`  ← **module's own fn**
- The 25 `external_body` are all in OTHER modules (`mm/phys/frame.rs`,
  `mm/phys/manager.rs`, `mm/phys/mod.rs`, `mm/virt/identity_map.rs`,
  `hal/mem/types/address/...`). These are external dependencies of the in-scope
  facet → "assume_specification / external_body on external dependencies
  temporarily allowed." NOT this module's responsibility this turn.
- `grep external_body|assume|trusted` in `mod.rs`/`mod.spec.rs`/`mod.proof.rs`
  → only doc-comment "assumes" prose; zero real `assume`/`external_body`/`trusted`.

**Per-item results:**

1. **In-scope fn has requires/ensures** — PASS. `gva_to_gpa` carries a
   `#[verus_spec(result => ensures result == gva, result as nat == (MicrovmTranslationView{}).spec_gva_to_gpa(gva as nat))]`
   (`mod.rs:425–434`). No `requires` needed: total over all `usize`.
   `gva_to_gpa` is NOT in `coverage-unverified.txt` → counted as contracted.

2. **Caller coverage** — PASS. `caller_analysis.md`: sole caller
   `book_mmio_regions` (`mm/phys/mod.rs:128`). Expectations: totality (no Result,
   ensures holds ∀ input), purity/determinism (spec fn), identity
   (`result == gva`), injectivity for the frame walk (exposed via
   `injective()` + `lemma_translation_injective`), valid encoding (identity
   preserves `usize` range; caller enforces via `from_mmio_address`). All
   covered.

3. **View consistency** — PASS. Ensures references
   `MicrovmTranslationView::spec_gva_to_gpa`; `inv()==true` is trivially
   maintained (stateless facet). Matches `view_design.md`.

4. **No tautological ensures** — PASS. Both clauses are substantive; no
   `Err(_) => true` style. Lemma ensures `v.injective()` (substantive).

5. **No subsumed ensures** — PASS (with note). `result as nat ==
   spec_gva_to_gpa(gva as nat)` is logically derivable from `result == gva`
   given the `open` identity definition, but it is the intended View-vocabulary
   handle for downstream specs (documented in `view_design.md` §Query
   abstraction). Acceptable as the canonical "result tied to the View" pattern,
   not harmful redundancy. inv() contributes nothing derivable.

6. **Error paths meaningful** — PASS / N/A. Return type is `usize`; no error path.

7. **No assume_specification for workspace-internal code** — PASS. None present
   in the module.

8. **vstd searched before assume_specification** — PASS / N/A. No
   assume_specification used.

9. **Specs written for the caller** — PASS. `result == gva` is directly usable
   in `book_mmio_regions`.

10. **Trait obligations** — PASS / N/A. Free function, implements no trait.

11. **Spec completeness (advisory)** — PASS. `result == gva` fully determines the
    output (no intentional nondeterminism); matches caller expectations.

12. **Loop invariants** — PASS / N/A. No loops in `gva_to_gpa`, `mod.spec.rs`, or
    `mod.proof.rs`.

13. **No cheating on module's own functions** — **FAIL** (see Fix Request).

14. **No specs weakened** — PASS. `spec_drift.py git-diff … --before HEAD`:
    "✅ No contract drift detected" (0 ensures removed, 0 requires added). This
    is the initial spec for `gva_to_gpa` (previously empty `verus!{}` block), so
    there is no prior guarantee to weaken.

15. **Bug awareness** — PASS. `gva_to_gpa` returning `gva` (identity) is correct
    for the MicroVM identity-mapped guest. No fundamental bug.

16. **Cross-module regression** — PASS. `make verify-kernel` verifies all modules,
    exit 0.

17. **Verification** — PASS. `make verify-kernel` exit 0; cargo build-std
    completed ("Finished dev profile"). No verification errors.

### Fix Request (item 13)

`lemma_translation_injective` (`mod.proof.rs:19`) uses `admit()`. This is the
module's OWN proof function, and its `ensures v.injective()` is part of the
caller-facing spec surface (`book_mmio_regions` relies on frame-distinctness).
An `admit()` body asserts that ensures unsoundly. The proof is trivial: with the
`open` identity definition `spec_gva_to_gpa(x) == x`, `injective()` reduces to
`forall|a,b| a == b ==> a == b`, which Verus discharges automatically.

**Change:** In `src/kernel/src/hal/platform/microvm/mod.proof.rs`, remove the
`admit();` from `lemma_translation_injective` and replace it with a real proof.
Try the empty body first (Verus should prove it from the open definition):

```rust
pub proof fn lemma_translation_injective(v: MicrovmTranslationView)
    ensures
        v.injective(),
{
}
```

If the quantifier does not auto-prove, make the witness explicit:

```rust
{
    assert forall|a: nat, b: nat|
        v.spec_gva_to_gpa(a) == v.spec_gva_to_gpa(b) implies a == b
    by {
        // spec_gva_to_gpa is identity, so a == b follows directly.
    }
}
```

**Verify:** run `make verify-kernel`. The summary line must show `admit=0` and
exit code 0 with no new errors. Confirm via
`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` that the
`mod.proof.rs:19 lemma_translation_injective: admit` entry is gone.

Justification ("admit is fine during the specification phase") is NOT accepted —
the proof is one line; discharge it or show tool output proving `admit=0`.

### Remaining for next turn
Re-verify item 13 after the fix (admit=0, exit 0). All other items already PASS.
