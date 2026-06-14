# Final Independent Review — `hal::mem::types::address::phys` (`PhysicalAddress`)

## 1) Per-check findings (PASS/FAIL/CONCERN with evidence)

### Check 1 — Spec quality for target items
**Status: CONCERN (not blocker by itself)**

- **`PhysicalAddress` View + `inv()`**: clear and minimal.
  - `View` is scalar `int` (`phys.rs:303-310`).
  - `inv()` is defined as frame representability (`phys.spec.rs:43-45`):
    - `spec_frame_number(self@) <= spec_max_frame_number()`.
- **`from_number`** (`phys.rs:138-141`):
  - Ensures `result@ == spec_from_number(spec_frame_raw_value(frame))`.
  - This captures frame-base address semantics; alignment and `inv()` are derivable, but not explicit.
- **`into_frame_number`** (`phys.rs:159-164`):
  - Requires `self.inv()` and ensures `spec_frame_raw_value(result) == spec_frame_number(self@)`.
  - This is the right semantic contract for frame projection and supports unwrap totality rationale.
- **`from_mmio_address`** (`phys.rs:112-119`):
  - Requires `spec_frame_number(addr@) <= spec_max_frame_number()`.
  - Ensures unconditional `result is Ok`, identity (`(Ok_0)@ == addr@`), and `(Ok_0).inv()`.
  - **Concern:** as an unsafe “unchecked MMIO wrapper,” this precondition still excludes top-of-address-space values and makes invalid calls UB rather than `Err`; this is defensible but narrower than a fully unchecked identity constructor.
- **No tautological `Err(_) => true`** in target function contracts.

### Check 2 — Caller coverage mapping
**Status: CONCERN**

- Covered: **13/14** caller expectations (explicit + derivable).
- One expectation is outside the target-function contract surface (trait-generic usability).
- Missing explicit clauses (round-trip/injectivity) are mostly derivable consequences, not fundamental gaps.

### Check 3 — Proof completeness (`admit`, `external_body` in three phys files)
**Status: PASS**

Exact declaration counts across:
- `src/kernel/src/hal/mem/types/address/phys.rs`
- `src/kernel/src/hal/mem/types/address/phys.spec.rs`
- `src/kernel/src/hal/mem/types/address/phys.proof.rs`

Results:
- `admit(` declarations: **0**
- `assume(` calls: **0**
- `#[verifier::external_body]` attributes: **0**
- `#[verifier::trusted]` attributes: **0**
- `assume_specification` declarations: **1** (at `phys.spec.rs:74`)

### Check 4 — TCB compliance
**Status: PASS**

- The single retained assumption is:
  - `phys.spec.rs:74` — `assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ]`
- It is explicitly allow-listed in:
  - `verus-ai-logs/tcb-allowed.md:170-175` (entry for this exact symbol/path).
- No additional trust boundary declaration in the three phys files.

### Check 5 — AST consistency / exec fidelity
**Status: FAIL — BLOCKER**

Using `ast_consistency.py ... summary`:
- `matched=14`, `mismatched=2`, `extra=1`.
- Mismatches:
  1. `PhysicalAddress::from_number` (expected documented deviation).
  2. `PhysicalAddress::into_frame_number` (**additional exec edit**).
- Extra in verus vs source:
  3. `PhysicalAddress::clone_address` (**out-of-scope function change**).

Details:
- `from_number` deviation (`phys.rs:143-153`) is semantically equivalent to original inline multiply:
  - Old: `frame.into_raw_value() * mem::FRAME_SIZE`
  - New: bind `raw_value`, then multiply.
  - Same evaluation effects and overflow behavior for pure operand/constant.
- **Additional in-scope exec edit** (`into_frame_number`, `phys.rs:167-169`):
  - Old: `raw_addr >> mem::FRAME_SHIFT`
  - New: bind `let shift = mem::FRAME_SHIFT;` then shift.
  - Semantically equivalent, but this is another exec edit beyond the declared single deviation.
- **Out-of-scope edit**:
  - `clone_address` exists at `phys.rs:278-280`; AST tool marks `EXTRA_IN_VERUS`.
  - Violates the hard scope rule (“unlisted functions must NOT be modified”).

### Check 6 — Verification result usage
**Status: PASS (as instructed)**

- Relied on provided authoritative statement: `make verify-kernel` exit 0 and module verified.
- Did **not** re-run `make verify-kernel` or `make build` (per instruction).

### Check 7 — Guardrails exact counts
**Status: FAIL due exec-fidelity blocker; token counts themselves PASS**

Token/declaration counts in the three phys files:
- `admit`: **0**
- `assume(`: **0**
- `external_body`: **0** (attribute declarations)
- `assume_specification`: **1**
- `trusted`: **0**
- `cfg`-gated exec functions: **0**
- `#[cfg(verus_keep_ghost)]` uses: **2** (`phys.rs:9,11`, include-gating only)

### Check 8 — Bug reconciliation (`bugs.md` B1)
**Status: PASS**

- B1 claims `#[verus_verify]` on `impl Address for VirtualAddress` regressed `verify-sys`; fix was revert.
- Independent confirmation in `src/libs/sys/src/sys/mm/address/virt.rs`:
  - Comment explicitly states trait impl intentionally not `#[verus_verify]` (`virt.rs:167-175`).
  - Impl exists unannotated (`virt.rs:176`).
  - Unsupported pointer casts remain in same impl (`virt.rs:270-275`).
- Therefore B1 classification as a genuine Verus limitation + reverted regression is consistent.
- Retained `assume_specification` in `phys.spec.rs:74` appears justified rather than defect-masking.

---

## 2) Spec Quality (focused analysis)

- **Strengths**
  - Contracts are attached directly to target exec functions (`phys.rs:112-119`, `138-141`, `159-164`).
  - `View` + `inv()` abstraction is coherent and caller-oriented.
  - `into_frame_number` contract directly states the semantic frame mapping used by callers.
- **Weak points / skeptic notes**
  - `from_mmio_address` is specified as always-`Ok`, with a representability precondition. This is internally consistent with unsafe UB semantics, but still narrows “unchecked MMIO” behavior.
  - `from_number` does not explicitly ensure `result.inv()`; this is derivable but less caller-friendly.
  - Cross-function properties (round-trip/injectivity) are not explicit postconditions; mostly derivable.

---

## 3) Caller Coverage (Covered N/Total + missing list)

### Covered: **13 / 14**

| Caller expectation (caller_analysis.md) | Coverage status | Clause / evidence |
|---|---|---|
| Type is opaque integer value model | Covered | `View` as `int` (`phys.rs:303-310`) |
| Type supports generic wrappers via `Address` trait | **Not directly covered in target specs** | Out-of-scope trait-level behavior |
| Validly-constructed address has frame mapping | Covered | `inv()` (`phys.spec.rs:43-45`) + `into_frame_number` requires |
| `from_number` total | Covered | Signature returns `Self` (`phys.rs:142`) |
| `from_number` returns frame base/aligned address | Covered | Ensures (`phys.rs:138-141`) |
| `from_number` round-trip intent | Derivable | From `from_number` + `into_frame_number` contracts with arithmetic |
| `into_frame_number` total/no panic for well-formed addr | Covered | `requires self.inv()` + unwrap justification (`phys.rs:159-175`) |
| `into_frame_number == addr >> FRAME_SHIFT` | Covered | Ensures (`phys.rs:163`) |
| Same-frame/different-frame mapping behavior | Derivable | Consequence of exact frame-number equality |
| Inverse of `from_number` on aligned inputs | Derivable | Consequence of both contracts |
| `from_mmio_address` identity on success | Covered | Ensures `result is Ok`, `(Ok_0)@ == addr@` (`phys.rs:116-117`) |
| `from_mmio_address` bypasses RAM validator | Partially covered | No RAM check in contract/body; not directly contrasted in clause |
| Unsafe caller-responsibility semantics | Covered (coarsely) | Unsafe fn + representability precondition |
| `from_mmio_address` Err arm currently unreachable | Covered | `ensures result is Ok` (`phys.rs:116`) |

### Missing list

- **Genuinely absent from target contracts:**
  1. Trait-generic usability expectation (wrap/use via `Address`) is not specified in in-scope target contracts (mostly out-of-scope by phase boundary).
- **Not explicit but derivable:**
  1. Round-trip (`from_number(n).into_frame_number() == n`)
  2. Per-frame injectivity/consistency properties
  3. `from_number` result invariant (`result.inv()`)

---

## 4) Guardrails — exact counts

From explicit grep/count across the three phys files:

- `admit`: **0**
- `assume(`: **0**
- `external_body` attrs: **0**
- `assume_specification` declarations: **1**
  - `src/kernel/src/hal/mem/types/address/phys.spec.rs:74`
- `trusted` attrs: **0**
- `cfg`-gated exec functions: **0**
- `#[cfg(verus_keep_ghost)]` uses: **2** (`phys.rs:9,11`, include-only)

Also:
- `spec_drift.py git-diff ... --before HEAD`: **No contract drift**.
- `fn_coverage.py phys.rs phys.rs`: **Matched 17/17 exec fns**, missing 0.

---

## 5) Bug reconciliation (`bugs.md`)

- **B1 status:** fixed and still valid as historical regression.
- **Independent verification of fix condition:** `virt.rs` trait impl intentionally not `#[verus_verify]`; pointer casts (`usize -> *const/*mut`) still present; this supports the stated limitation and rationale for one retained `assume_specification`.
- **Undiscovered/unrecorded bugs found by this review:**
  - Not a runtime bug, but **process/compliance bug**: additional exec edits beyond the declared single deviation, including out-of-scope function change.

---

## 6) Issues list (highest priority first)

1. **BLOCKER — Additional exec edits beyond declared single deviation**
   - Evidence: AST summary reports `into_frame_number` mismatch; diff shows added `let shift` (`phys.rs:167-169`).
   - Even if semantically equivalent, this violates “verify no other exec edits” requirement.

2. **BLOCKER — Out-of-scope function modified (`clone_address`)**
   - Evidence: AST summary reports `PhysicalAddress::clone_address` as `EXTRA_IN_VERUS`; function present at `phys.rs:278-280`.
   - Violates hard scope rule: unlisted functions must not be modified.

3. **CONCERN — `from_mmio_address` precondition narrows unsafe unchecked identity**
   - Evidence: `requires spec_frame_number(addr@) <= spec_max_frame_number()` (`phys.rs:114`).
   - May be acceptable as UB boundary, but is a stricter interface than “unchecked wrapper” wording suggests.

---

## 7) Final verdict

**FAIL** — verification/trust counters are clean for the three phys files, but AST consistency reveals additional exec changes (including an out-of-scope function), violating the stated scope/fidelity requirements.
