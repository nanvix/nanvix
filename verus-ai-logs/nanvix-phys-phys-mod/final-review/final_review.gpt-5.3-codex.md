# Final Verification Review — `mm::phys` phys-mod (`init`, `book_mmio_regions`, `book_physical_memory_regions`)

## Spec Quality

### 1) API-level quality (strict)
- `init` is externally observed; helpers are private.
- Contracts are declarative in style, but **underspecified on failure**.

Evidence (`src/kernel/src/mm/phys/mod.rs`):
- `book_physical_memory_regions`: `Err(_) => true` (line 70), with only unconditional `phys_view().inv()` / `initialized` ensures.
- `book_mmio_regions`: `Err(_) => true` (line 100), same pattern.
- `init`: `Err(_) => true` (line 164), only unconditional `phys_view().inv()`.

Verdict on `Err(_) => true` + unconditional `inv()`:
- This is **not fully tautological** (because `inv()` still constrains post-state),
- but it is still a **one-sided error spec** for failure causality/liveness: no non-spurious-failure guarantee, no abstract conflict predicate (e.g., `!all_free(...)`), no fail-fast criterion.
- Under `spec-design` anti-patterns, this remains a material quality issue.

### 2) `uninterp spec fn` review (firm verdict per function)
In `src/kernel/src/mm/phys/mod.spec.rs`:
- `byte_at_address` (line 13): **Does NOT qualify as mechanical consequence** of an external-bottom boundary in this module; appears unused in phys-mod. **Verdict: disguised/dead assumption surface (non-compliant with strict no-`uninterp` rule).**
- `phys_view` (line 98): parameter-free global state accessor tied to singleton trust boundary (`frame::instance` external_body pins `(*r)@ == phys_view().frames`, `phys_view().initialized`). **Verdict: qualifies as mechanical consequence.**
- `phys_regions_frame_set` (line 177): `LinkedList` has no Verus model and no fold semantics available; helper loops are externalized. **Verdict: qualifies as mechanical consequence (given current `LinkedList` limitation).**
- `mmio_regions_frame_set` (line 183): same rationale as above. **Verdict: qualifies as mechanical consequence.**

## Caller Coverage

Source: `verus-ai-logs/nanvix-phys-phys-mod/caller_analysis.md`.

### Coverage result: **7 / 12 covered**

| # | Caller expectation | Covered by current requires/ensures? | Evidence |
|---|---|---|---|
| 1 | `init` establishes allocator initialized on success | ✅ | `Ok(_) => phys_view().live()` |
| 2 | One-shot init (`init` only when not initialized) | ❌ | no `requires !phys_view().initialized` |
| 3 | Seed relation to `physical_memory_layout` | ❌ | no postcondition relating state to layout/seed |
| 4 | Physical regions booked => reserved | ✅ | `all_reserved(phys_regions_frame_set(...))` |
| 5 | Coverage-gated MMIO booking | ✅ (partial) | `contains(a) && covers(a) ==> reserved(a)` |
| 6 | Manager/upool layer live after success | ✅ | `phys_view().live()` (includes `manager_ready`) |
| 7 | `wf`/invariant preserved across `init` | ✅ | unconditional `phys_view().inv()` and `live()` |
| 8 | Fail-fast conflict surfaces as `Err` (non-spurious failure) | ❌ | no bidirectional error condition |
| 9 | `book_physical_memory_regions` success books all region frames | ✅ | helper `Ok(_) => all_reserved(...)` |
|10 | `book_physical_memory_regions` failure condition captured (conflict) | ❌ | helper `Err(_) => true` |
|11 | `book_mmio_regions` success books covered MMIO frames | ✅ | helper `Ok(_) => covers ==> reserved` |
|12 | `book_mmio_regions` failure condition captured (conversion/book conflict) | ❌ | helper `Err(_) => true` |

Missing items: **#2, #3, #8, #10, #12**.

### Drift vs `view_design.md` (proposed stronger clauses)
Dropped from design:
- `init requires !v.initialized` (one-shot precondition) — dropped.
- Composed transition `v'.frames == seed(...).book_all(P).book_covered(M)` — dropped.
- Helper `Err` arms with conflict/wf clauses (`!all_free(R)`, etc.) — dropped to `Err(_) => true`.

Assessment:
- This is **not an acceptable simplification** for a strict final API review because it weakens caller-relied guarantees (especially one-shot and fail-fast/error-causality).

## Proof Completeness

### Phys-mod files only (`mod.rs`, `mod.spec.rs`, `mod.proof.rs`)
- `admit()`: **0** (PASS)
- `external_body` attributes: **3**
  - `mod.rs:59` `book_physical_memory_regions`
  - `mod.rs:87` `book_mmio_regions`
  - `mod.spec.rs:66` `ExLinkedList` type registration

### Kernel-wide (from actual verify run)
From `make verify-kernel MODULE=mm::phys` output:
- Global cheating totals: **assume=0, external_body=18, admit=27, cfg_gate=15**
- These are kernel-wide (siblings included), not phys-mod-only.

## TCB Compliance

- `book_physical_memory_regions` is pre-listed in `verus-ai-logs/tcb-allowed.md` (line 74).
- `book_mmio_regions` is pre-listed (line 79).
- So both phys-mod helper trust boundaries are approved.

`ExLinkedList` assessment:
- Registered with `external_type_specification` + `external_body` (mod.spec.rs:65-69).
- Not explicitly enumerated in TCB allowed list’s mm::phys external_body bullets.
- Since it is a type-registration shim (not a callable function contract), I treat this as **non-blocking but should be explicitly documented in TCB for bookkeeping clarity**.

## Guardrails Compliance (phys-mod only)

Grep scope: `src/kernel/src/mm/phys/{mod.rs,mod.spec.rs,mod.proof.rs}`.

- `admit`: **0**
- `assume(...)`: **0**
- `assume_specification`: **0**
- `external_body` attrs: **3**
  - `mod.rs:59`, `mod.rs:87`, `mod.spec.rs:66`
- cfg-gated exec cheating pattern (`cfg(not(verus_keep_ghost))`): **0**

Notes on cfg usage (present but non-cheating in this module):
- `mod.rs:15`, `195` (`feature = "test"`)
- `mod.rs:36`, `40`, `42` (`verus_keep_ghost` around ghost imports/includes)

## AST Consistency

Commands run:
- `python3 .../ast_consistency.py src/kernel/src/mm/phys/mod.rs count`
  - Output: `✅ Consistent: 4 functions, 0 structs match.`
- `python3 .../ast_consistency.py src/kernel/src/mm/phys/mod.rs summary`
  - `book_mmio_regions`, `book_physical_memory_regions`, `init`, `test` all MATCH.

`// VERUS REWRITE` in `mod.rs`: **none found**.
No AST mismatch blockers.

## Verification

Command run:
- `make verify-kernel MODULE=mm::phys`

Result:
- Exit code **0**
- Verification errors: **0**
- Status still reports `CHEATING_DETECTED` at module aggregate level due sibling modules (`frame`, `manager`, `upool`) being in-progress; this is consistent with scope notes.

## Bug Summary (reconciling `bugs.md`)

1. **"Code bugs: None found"**
   - Recheck found no concrete exec bug in the three in-scope functions.

2. **LinkedList verifier limitation entry**
   - Still valid.
   - Confirmed helpers are externalized (`mod.rs:59`, `87`) and TCB-listed.
   - `ExLinkedList` registration present (`mod.spec.rs:65-69`).

3. **Any bug masked by external_body?**
   - No new concrete runtime bug evidence found.
   - However, helper bodies are trusted due Verus limitation; semantic issues inside those loops would not be mechanically proven here.

4. **Unrecorded verification failures**
   - None for phys-mod target functions (module verify exit 0).

## Issues (priority-ordered)

### BLOCKER 1 — API contract under-specifies failure semantics (one-sided error spec)
- `Err(_) => true` on all three target functions lacks fail-fast/non-spurious-error characterization required by caller expectations.

### BLOCKER 2 — Caller-required one-shot and seeding guarantees are missing in `init` contract
- No `requires !phys_view().initialized`.
- No postcondition tying final frame state to `physical_memory_layout` seeding/composed transition.

### BLOCKER 3 — `byte_at_address` `uninterp` lacks clear mechanical external-boundary justification
- Present in phys-mod spec surface; appears unused and not boundary-pinned in this module.

### Advisory
- `ExLinkedList` external type shim uses `external_body`; explicitly list it in TCB doc for transparency even if treated as non-callable registration.

## Final Verdict

**FAIL**.

Reason: despite clean AST consistency and successful `verify-kernel` run for the module, the **public-contract quality is insufficient for strict final sign-off**: missing one-shot/seeding guarantees, one-sided error-path specification against caller fail-fast expectations, and one unqualified `uninterp` surface (`byte_at_address`).
