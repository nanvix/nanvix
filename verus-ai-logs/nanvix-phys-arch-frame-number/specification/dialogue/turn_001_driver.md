## Turn 1: Full checklist sweep — primary FAIL is "every in-scope exec function has requires/ensures"

### Progress
- Done (PASS): items 7, 8, 10, 12, 13, 15 (see status table).
- Current FAIL (blocking, fix first): item 1 — in-scope exec functions have **no** contracts, and the crate does **not compile**.
- Remaining (blocked on item 1): items 2, 3, 4, 5, 6, 9, 11, 14, 16, 17.

### Verification

**Command run:** `make verify-arch`

```
error[E0252]: the name `mem` is defined multiple times
  --> src/libs/arch/src/x86/mem/paging/frame/number.spec.rs:3:5
 3 | use crate::mem;
   |     ^^^^^^^^^^ `mem` reimported here
  ::: src/libs/arch/src/x86/mem/paging/frame/number.rs:14:5
14 | use crate::mem;
   |     ---------- previous import of the module `mem` here
error: could not compile `arch` (lib) due to 1 previous error
=== Results ===
  0 verified
  compilation/setup error (verus did not run)
  Exit code : 101
  coverage: 0/525 exec functions have contracts
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
```

**Files read:** `number.rs`, `number.spec.rs`, `number.proof.rs`, `caller_analysis.md`, `view_design.md`, and the sibling reference implementation `src/kernel/src/hal/mem/types/address/frame.rs` (the codebase's `#[verus_spec]` attachment pattern).

**Findings:**
1. `number.spec.rs:3` reimports `use crate::mem;` which `number.rs:14` already imports (the spec file is `include!`d into the exec file). This is a hard **compile error** — Verus never even runs. `number.rs` must keep its `use crate::mem;` (the exec `const MAX = mem::MAX_ADDRESS / mem::FRAME_SIZE - 1` needs it), so the duplicate must be removed from `number.spec.rs`.
2. `number.rs` exec functions `from_raw_value` and `into_raw_value` carry **no** `#[verus_spec(...)]` attribute. `number.spec.rs` defines only `spec_frame_size()`, `spec_max_frame_number()`, `View for FrameNumber`, and `inv()`. `number.proof.rs` is an empty `verus! { }`. Therefore **0 in-scope contracts exist** — the specification phase deliverable is essentially not started.
3. `from_raw_value` is a self-less associated function; per the sibling pattern (`frame.rs`), self-less verification targets must live in a `#[verus_verify] impl` block.

### Per-checklist-item status

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Every in-scope exec fn has requires/ensures | **FAIL** | `from_raw_value`, `into_raw_value` have no `#[verus_spec]`; coverage 0; crate does not compile |
| 2 | Caller coverage vs `caller_analysis.md` | **FAIL** | None of the required guarantees (Some-iff-`value<=MAX`, index preservation, `result as int == self@`, range) are expressed |
| 3 | View consistency vs `view_design.md` | **PARTIAL** | `View`/`inv` definitions match view_design and reference `self@`/`spec_max_frame_number()` ✅, but **no exec spec references the View** ❌ |
| 4 | No tautological ensures | N/A now | No ensures exist; must avoid `None => true` when added |
| 5 | No subsumed ensures | WATCH | view_design itself flags `into_raw_value`'s `0 <= result <= max` as "(implied by inv())" — keep it only if needed by callers, else it's subsumed |
| 6 | Error paths meaningful | **FAIL** | `from_raw_value` None branch needs `value > spec_max_frame_number() <==> result is None`; absent |
| 7 | No assume_specification for workspace-internal code | **PASS** | none present (only a comment mentions "assumed") |
| 8 | vstd searched before assume_specification | **PASS (N/A)** | no assume_specification |
| 9 | Specs written for the caller | **FAIL** | no specs exist |
| 10 | Trait obligations | **PASS** | caller_analysis: only derived Debug/Clone/Copy; no contract traits |
| 11 | Spec completeness (advisory) | PENDING | cannot assess until specs exist |
| 12 | Loop invariants | **PASS (N/A)** | no loops in module |
| 13 | No cheating (admit/assume/external_body/trusted) | **PASS** | make: `assume=0 external_body=0 admit=0 trusted=0`; grep confirms none |
| 14 | No specs weakened (spec drift) | **FAIL** | `tcb-allowed.md` records upstream-trusted contracts for `from_raw_value`/`into_raw_value`; with 0 in-module specs those assumed contracts are undischarged (= weaker/absent). Added specs must be ≥ tcb-allowed strength |
| 15 | Bug awareness | **PASS** | exec bodies are trivially correct; no bugs_file needed |
| 16 | Cross-module regression (`make verify-arch`) | **FAIL** | compile error, exit 101, 0 verified |
| 17 | Verification + build | **FAIL** | `make verify-arch` exit 101; build cannot succeed while crate doesn't compile |

### Fix Request (address item 1 first; it unblocks 2,3,6,9,14,16,17)

**File: `src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`**
1. Delete line 3 `use crate::mem;`. The exec file `number.rs:14` already imports `crate::mem`, and the spec file is `include!`d into it, so the spec fns `spec_frame_size`/`spec_max_frame_number` resolve `mem::FRAME_SIZE`/`mem::MAX_ADDRESS` through the exec import. Re-run `make verify-arch` and confirm the `E0252` error is gone.

**File: `src/libs/arch/src/x86/mem/paging/frame/number.rs`** — add contracts to both in-scope exec functions, matching the `view_design.md` sketch and `caller_analysis.md`, and following the sibling pattern in `src/kernel/src/hal/mem/types/address/frame.rs`:

- `into_raw_value(self) -> usize`: add
  ```
  #[verus_spec(result =>
      requires self.inv(),
      ensures result as int == self@,
  )]
  ```
  (`result as int == self@` is the load-bearing identity callers need. The `0 <= result <= spec_max_frame_number()` range follows from `self.inv()`; include it only if a caller proof cannot derive it from `inv()` — otherwise it is a subsumed ensures, item 5.)

- `from_raw_value(value: usize) -> Option<Self>`: place it in a `#[verus_verify] impl FrameNumber { ... }` block (self-less associated fn) and add
  ```
  #[verus_spec(result =>
      ensures
          (result is Some) <==> (value as int <= spec_max_frame_number()),
          result matches Some(f) ==> f@ == value as int && f.inv(),
  )]
  ```
  This is the **bidirectional** Some/None condition (item 6, not tautological) plus exact index preservation and well-formedness — exactly the `tcb-allowed.md` contract (item 14) and every caller's assumption (item 2).

- Ensure `spec_max_frame_number()` / `spec_frame_size()` are provably tied to the exec `FrameNumber::MAX` and `mem::FRAME_SIZE` so the `from_raw_value` body (`value > Self::MAX`) discharges against `value as int <= spec_max_frame_number()`. If a constant-bridge lemma or `broadcast`/`assert` is required, add it in `number.proof.rs`.

**Verify after changes:**
- `make verify-arch` → must report the two functions verified, `0 errors`, exit 0, and coverage increasing (no longer 0 for `from_raw_value`/`into_raw_value`).
- Confirm no new `admit`/`assume`/`external_body`/`trusted` were introduced (the cheating check must stay all-zero).

Do not justify the missing contracts — add the `#[verus_spec]` attributes and show a passing `make verify-arch`.
