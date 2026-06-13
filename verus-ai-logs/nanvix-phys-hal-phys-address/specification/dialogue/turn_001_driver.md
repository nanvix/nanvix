## Turn 1: Full checklist walkthrough — FAIL on `from_mmio_address` (tautological ensures + not caller-usable)

### Progress
- **Current FAIL (this turn):** `from_mmio_address` — tautological `Err(_) => true` ensures, and missing `inv()` guarantee required by its caller (`is_covered`).
- **PASS:** fn-coverage, view-consistency, spec-drift, cross-module regression, verification/build, loop-invariants (N/A), assume_specification scoping, vstd-search, trait-obligations, subsumed-ensures, `from_number`/`into_frame_number` ensures.
- **Deferred to proving phase (tracked):** 2 in-module `admit()` lemmas.
- **Remaining:** re-verify after the `from_mmio_address` fix.

---

### Verification performed (concrete evidence)

**`make verify-kernel`** → exit 0. Module `hal::mem::types::address::phys` verified (cached). Cross-module: `kernel::all` passes.
```
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=14 admit=26 trusted=0 no_decreases=0 cfg_gate=16
coverage: 45/1023 exec functions have contracts
```

**`fn_coverage.py`** (phys.rs) → Source exec fns 16, Verus exec fns 16, Matched 16, Missing 0, Extra 0. In-scope target fns (`from_number`, `into_frame_number`, `from_mmio_address`) all present. **PASS** (note: only in-scope fns are required to carry contracts this phase; the `Address`-trait/`from_virtual_address`/`Debug` fns are explicitly out of scope per `view_design.md`/`caller_analysis.md`).

**`spec_drift.py check hal-phys-address`** → baseline `88d41fe` (specification phase start) → HEAD: Functions with changes 0, ensures removed 0, requires added 0. **✅ No contract drift.** Specs added this phase are net-new (strengthening), nothing weakened. **PASS.**

---

### Per-checklist-item verdicts

1. **Every in-scope exec fn has requires/ensures** — PASS. `from_number` (ensures view relation), `into_frame_number` (requires `inv()`, ensures frame relation), `from_mmio_address` (ensures present but defective — see FAIL).
2. **Caller coverage** (`caller_analysis.md`) — PARTIAL → drives the FAIL below. `from_number`→base-address relation ✅; `into_frame_number`→`addr/FRAME_SIZE` + totality via `requires inv()` ✅; **`from_mmio_address`→ caller `mm/phys/mod.rs:118 book_mmio_regions` wraps the result in `PageAligned` and calls `frame::is_covered`, which `requires phys_addr.inv()` (frame.rs:526). The current `from_mmio_address` spec provides no `inv()`, so this caller cannot discharge that precondition.** FAIL.
3. **View consistency** (`view_design.md`) — PASS. Specs reference `self@` (`int`), `spec_frame_number`, `spec_from_number`; `inv()` defined as `spec_frame_number(self@) <= spec_max_frame_number()`, matching the design doc.
4. **No tautological ensures** — **FAIL.** `from_mmio_address` contains the literal prohibited pattern `Err(_) => true` (phys.rs:116). The body is unconditionally `Ok(Self(addr))`, so this arm is pure tautology.
5. **No subsumed ensures** — PASS. `from_number`/`into_frame_number` deliberately omit derivable `inv()`/alignment; consistent with minimality.
6. **Error paths have meaningful ensures** — **FAIL** (same fn). The `Err` arm is unreachable yet stated as `true` instead of being captured as "never errors" (`result is Ok` / `Err(_) => false`).
7. **No assume_specification for workspace-internal code** — PASS (within temporary allowance). The 5 `assume_specification`s target cross-crate, not-yet-verified deps (`::sys::mm::VirtualAddress::{new,into_raw_value}`, `::arch::mem::FRAME_SHIFT`, `::arch::…::FrameNumber::{from_raw_value,into_raw_value}`). These are dependency contracts for unverified modules — explicitly permitted ("assume_specification on external dependencies temporarily allowed").
8. **vstd searched before assume_specification** — PASS. All assumed items are Nanvix/arch domain types (virtual address newtype, frame-number newtype, arch frame-shift constant); no vstd equivalents exist.
9. **Specs written for the caller (usable directly in caller proofs)** — **FAIL** for `from_mmio_address` (see item 2). `from_number`/`into_frame_number` PASS.
10. **Trait obligations satisfied** — PASS. The `Address`-trait methods are out of scope this phase (reached generically; no in-scope obligations). In-scope fns are inherent conversions.
11. **Spec completeness (advisory)** — `from_mmio_address`'s nondeterministic `Err` arm is *not* intentional nondeterminism matching caller expectation; the body is deterministic `Ok`. Strengthen (folds into the FAIL).
12. **Loop invariants** — PASS (N/A; no loops in module).
13. **No cheating on module's own functions** — report (counts):
    - `phys.proof.rs:6 lemma_from_number_no_overflow` → `admit()`
    - `phys.proof.rs:16 lemma_frame_index` → `admit()`
    - `assume`=0, `external_body`=0, `trusted`=0 in-module.
    These 2 `admit()`s are isolated proof lemmas, commented "Proven in the proving phase." **Phase-acceptable** (specification phase delivers specs, not discharged proofs) but **explicitly tracked**: they MUST be discharged in the proving phase and must not be used to weaken specs.
14. **No specs weakened** — PASS (`spec_drift` clean, exit 0).
15. **Bug awareness** — no fundamentally incorrect code found. One spec-usability gap recorded here (item 2/9).
16. **Cross-module regression** — PASS (`kernel::all` exit 0).
17. **Verification + build** — PASS (`make verify-kernel` exit 0, 0 errors).

---

### Fix Request (address this turn)

**File:** `src/kernel/src/hal/mem/types/address/phys.rs`, function `from_mmio_address` (currently lines 112–121).

**Problem:** The ensures uses the prohibited tautological arm `Err(_) => true`, and provides no `inv()` on the result. Its real caller `book_mmio_regions` (`mm/phys/mod.rs:118`) feeds the result through `PageAligned::from_address` into `frame::is_covered`, which `requires phys_addr.inv()` (`mm/phys/frame.rs:526`). The current spec cannot discharge that, so it is not usable in the caller proof.

**Change the annotation from:**
```rust
#[verus_spec(result =>
    ensures
        match result {
            Ok(r) => r@ == addr@,
            Err(_) => true,
        },
)]
```
**to (strengthened, non-tautological, caller-usable):**
```rust
#[verus_spec(result =>
    requires
        spec_frame_number(addr@) <= spec_max_frame_number(),
    ensures
        result is Ok,
        (result->Ok_0)@ == addr@,
        (result->Ok_0).inv(),
)]
```

Rationale / why this is provable and correct:
- Body is `Ok(Self(addr))`, so `result is Ok` and `(result->Ok_0)@ == addr@` hold directly (eliminates the tautological `Err` arm — item 4/6/11).
- `inv(r) == spec_frame_number(r@) <= spec_max_frame_number()`; since `r@ == addr@`, this reduces to the added `requires` (item 2/9). This is the caller's `unsafe` obligation — that the MMIO GPA is frame-representable — which `view_design.md` explicitly deferred to the spec phase ("whether `inv()` is attached to the Ok arm … is settled in the spec phase"). It does NOT reintroduce the RAM-range validator (MMIO addresses like `0xFEE0_0000` are frame-representable but outside tracked RAM), so the "unchecked" design intent is preserved.

Do not "explain" the `Err(_) => true` as harmless — the body never errors, so the spec must state that. Change the code.

**Verify after fixing:**
1. `make verify-kernel` → must remain exit 0 (module `phys` re-verifies; `from_mmio_address` body must prove the new ensures).
2. `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py check hal-phys-address --logs-dir verus-ai-logs` → must stay exit 0 (no drift; this is strengthening).
3. `grep -n "Err(_) => true" src/kernel/src/hal/mem/types/address/phys.rs` → must return nothing.
