# Final Independent Review — `mm::phys` (`phys-mod`)

## Summary Verdict: **FAIL**

Strict review failed on contract completeness and TCB compliance.

- ✅ Verification command exit code: 0
- ✅ Build command exit code: 0
- ✅ AST consistency: no mismatch
- ✅ Spec drift: none
- ✅ No `admit` / `assume`
- ❌ Caller-required booking effects are not exposed in `init` / `book_*` contracts
- ❌ `ExLinkedList` trust boundary is not listed in `tcb-allowed.md`

---

## 1) Spec Quality (strict)

### a) `Err(_) => true` arms
- `book_physical_memory_regions` uses `Err(_) => true` (`src/kernel/src/mm/phys/mod.rs:77-80`).
- `book_mmio_regions` uses `Err(_) => true` (`src/kernel/src/mm/phys/mod.rs:112-115`).
- `init` uses `Err(_) => true` (`src/kernel/src/mm/phys/mod.rs:177-180`).

Assessment:
- This is a tautological error-arm pattern (anti-pattern), but partially justified for this boot path because caller treats failure as terminal (`caller_analysis.md:79-82, 99-101, 112-114, 140-141`).
- Still weak: neither `book_*` error arm provides explicit frame condition / state-preservation guarantee.

### b) `init` Ok-arm disjointness redundancy
- `init` ensures both `phys_view().inv()` and on `Ok` disjointness (`src/kernel/src/mm/phys/mod.rs:168-176`).
- `phys_view().inv()` means `initialized ==> frames.wf()` (`mod.spec.rs:105-107`).
- `FrameAllocView::wf()` already includes `allocated_frames.disjoint(free_frames)` (`mod.spec.rs:37`).

Classification: **Subsumed** (redundant, not wrong).

### c) Do `book_*` contracts capture booking effect?
- Both `book_*` contracts only ensure `phys_view().inv()` and (on `Ok`) `phys_view().initialized` (`mod.rs:71-81`, `106-116`).
- They do **not** specify which frames became booked.
- Proof lemmas model booking effects abstractly (`mod.proof.rs:159-197`, `199-229`) but those effects are not connected to `book_*` exec postconditions.

Assessment: gap exists. LinkedList-orphan-rule limitation explains body externalization, but does **not** fully excuse missing caller-visible booking effect at boundary. This is a review failure for completeness.

---

## 2) Caller Coverage

Source expectations: `verus-ai-logs/nanvix-phys-phys-mod/caller_analysis.md`.

### Coverage summary: **Covered 4 / 10**, Missing 6

Covered:
1. `init` establishes initialized/invariant on `Ok` (`mod.rs:168-176`).
2. Failure treated terminal/no recovery semantics compatible with weak `Err` arm (`mod.rs:177-180`, caller analysis `79-82, 140-141`).
3. `book_*` can return `Err` without contradictory guarantees (compatible with fatal propagation from `init`).
4. Invariant preservation (`phys_view().inv()`) is present across all three contracts.

Missing / insufficiently bound:
1. Physical-region booking effect (“all region frames booked”) not in `book_physical_memory_regions` contract (`mod.rs:71-81`).
2. MMIO tracked booking + untracked-skip effect not in `book_mmio_regions` contract (`mod.rs:106-116`).
3. `init` does not expose reserved physical frames guarantee expected by caller (`caller_analysis.md:70-72`).
4. `init` does not expose tracked-MMIO booking/skip guarantee expected by caller (`caller_analysis.md:73-74`).
5. `init` contract does not state `PhysMemoryManager`/`Upool` ready condition (`caller_analysis.md:75-76`).
6. One-shot behavior not captured as spec-level requirement/guarantee (`caller_analysis.md:77-79, 138-139`).

Note: abstract lemmas exist (`mod.proof.rs`) but are not wired into the target exec contracts.

---

## 3) Proof Completeness

Module-scope (`mod.rs`, `mod.spec.rs`, `mod.proof.rs`) grep/count:
- `admit(...)`: **0**
- `assume(...)` / `assume!(...)`: **0**
- `assume_specification`: **0**
- `trusted`: **0**
- `exec_allows_no_decreases_clause`: **0**

`admit > 0` blocker check: **PASS**.

---

## 4) TCB Compliance

Module trust boundaries found:
- `book_physical_memory_regions` external body (`mod.rs:70`)
- `book_mmio_regions` external body (`mod.rs:105`)
- `ExLinkedList` external type specification + external body (`mod.spec.rs:69-73`)

`tcb-allowed.md` explicitly lists:
- `mod.rs::book_physical_memory_regions` (`tcb-allowed.md:7-17`) ✅
- `mod.rs::book_mmio_regions` (`tcb-allowed.md:18-22`) ✅

But no entry for `ExLinkedList` / `mod.spec.rs` external type specification ❌ (no match found).

Strict result: **FAIL (TCB list mismatch / missing approval entry).**

---

## 5) Guardrails (exact module-scope counts)

Across:
- `src/kernel/src/mm/phys/mod.rs`
- `src/kernel/src/mm/phys/mod.spec.rs`
- `src/kernel/src/mm/phys/mod.proof.rs`

Counts:
- `admit`: **0**
- `assume`/`assume!`: **0**
- `external_body` attrs: **3** (2 functions + 1 `ExLinkedList`)
- `assume_specification`: **0**
- cfg-gated exec (`cfg(not(verus_keep_ghost))`): **0**

Additional observation:
- `cfg(verus_keep_ghost)` occurrences: **3** (imports/includes in `mod.rs`), not cfg-gated exec bodies.

Guardrail blocker condition (`admit>0` or `assume>0`): **PASS**.

---

## 6) AST Consistency

- `ast_consistency.py ... count` => `✅ Consistent: 4 functions, 0 structs match.`
- `... summary` => all target functions `MATCH`; no mismatch.
- `VERUS REWRITE` comments in three module files: none found.

Result: **PASS**.

---

## 7) Verification Results

### `make verify-kernel MODULE=mm::phys`
- Exit code: **0**
- Raw cheating summary line:
  - `cheating: assume=0 external_body=17 admit=0 trusted=0 no_decreases=0 cfg_gate=5`
- Module verification status from tool: `status: CHEATING_DETECTED` (due allowed external bodies in broader module set).

### `make build`
- Exit code: **0**
- Output: `make: Nothing to be done for 'build'.`

Result: command-level verification/build **PASS**, but final review remains FAIL due issues above.

---

## 8) Bug Reconciliation

Read: `verus-ai-logs/nanvix-phys-phys-mod/bugs.md`.

- Existing record says no code bugs found and LinkedList limitation is tooling-only.
- Final code is consistent with that statement for runtime logic.
- New review findings are **spec/assurance defects**, not newly found runtime logic bugs:
  1. Missing caller-visible booking effects in contracts.
  2. Missing TCB approval entry for `ExLinkedList` trust boundary.

No evidence `external_body` directly masks a newly discovered runtime bug in `init/book_*`; however, it does reduce assurance where booking effects are not contracted.

---

## 9) Issues (highest priority first)

1. **BLOCKER — TCB allow-list mismatch**
   - `ExLinkedList` external trust boundary present (`mod.spec.rs:69-73`) but absent from `tcb-allowed.md`.
2. **BLOCKER — Caller contract gap on booking effects**
   - `book_*` and therefore `init` do not expose caller-required reservation effects (physical + tracked MMIO booking).
3. **MAJOR — Tautological Err-arm style**
   - `Err(_) => true` in all three contracts is weak; partially justified by terminal boot semantics, but still under-specifies failure-state behavior.
4. **MINOR — Redundant `disjoint` clause in `init` Ok-arm**
   - Subsumed by `inv` + `initialized` via `FrameAllocView::wf`.

---

## Raw Command Outputs

### Command
`make verify-kernel MODULE=mm::phys`

```text
Using Verus installation at /home/ruize/toolchain/verus.
RUSTFLAGS="-C relocation-model=static -C prefer-dynamic=no" \
PATH="/home/ruize/toolchain/verus:$PATH" \
VERUS_AI_DIR="/home/ruize/verus-ai-exp/verus-ai" \
VERUS_EXTRA_CARGO_ARGS="--locked --features microvm,trace -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target /home/ruize/nanvix-phy/build/targets/x86-kernel.json" \
/home/ruize/nanvix-phy/scripts/verify.sh --crate kernel --module mm::phys --log-dir verus-ai-logs/verify-kernel
=== Verus Verification (cargo mode) ===
  Crate     : kernel
  Module    : mm::phys
  Source dir : /home/ruize/nanvix-phy/src/kernel/src
  Channel   : <default>
  Extra args: --locked --features microvm,trace -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target /home/ruize/nanvix-phy/build/targets/x86-kernel.json
  Timestamp : 2026-06-15_03-03-07

note: verifying module mm::phys

note: verifying module mm::phys::frame

note: verifying module mm::phys::manager

note: verifying module mm::phys::upool

    Finished `dev` profile [optimized + debuginfo] target(s) in 0.23s

=== Results ===
  cached (no recompilation)
  —
  Exit code : 0

=== Cheating Pattern Check ===
  Module mm::phys:
    ⚠️  external_body: 17
  Affected functions:
    - frame.rs:137 alloc: external_body
    - frame.rs:210 alloc_contiguous: external_body
    - frame.rs:290 free: external_body
    - frame.rs:368 share: external_body
    - frame.rs:428 refcount: external_body
    - frame.rs:481 book: external_body
    - frame.rs:517 is_covered: external_body
    - frame.rs:565 alloc_range: external_body
    - frame.rs:668 init: external_body
    - frame.rs:765 is_covered: external_body
    - frame.rs:785 book: external_body
    - frame.rs:807 alloc_range: external_body
    - manager.rs:86 init: external_body
    - mod.rs:82 book_physical_memory_regions: external_body
    - mod.rs:117 book_mmio_regions: external_body
    - mod.spec.rs:73 ExLinkedList (struct): external_type_spec
    - upool.rs:148 new: external_body
  Global: assume=0 external_body=17 admit=0 trusted=0 cfg_gate=5
  Detail: verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt

=== Function Coverage ===
  15/44 exec functions have contracts.
  Unverified functions:
    - instance
    - alloc
    - alloc_contiguous
    - free_count
    - free
    - share
    - refcount
    - new
    - base
    - clear
    - deref
    - deref_mut
    - drop
    - init
    - get_mut
    - alloc_many_user_frames
    - alloc_user_frame
    - check_user_watermark
    - alloc_kernel_frame
    - alloc_many_kernel_frames
    - test
    - new
    - address
    - leak
    - share
    - refcount
    - drop
    - new
    - alloc

=== Summary ===
  verification: cached (no recompilation), — (exit 0)
  cheating: assume=0 external_body=17 admit=0 trusted=0 no_decreases=0 cfg_gate=5
  coverage: 15/44 exec functions have contracts
  status: CHEATING_DETECTED

Log written to: verus-ai-logs/verify-kernel/verus-logs/verus_2026-06-15_03-03-07.log
<shellId: 20 completed with exit code 0>
```

### Command
`make build`

```text
make: Nothing to be done for 'build'.
<shellId: 21 completed with exit code 0>
```

### Command
`python3 /home/ruize/verus-ai-exp/verus-ai/scripts/fn_coverage.py src/kernel/src/mm/phys/mod.rs src/kernel/src/mm/phys/mod.rs`

```text
# Function Coverage Report

- **Source file:** `src/kernel/src/mm/phys/mod.rs`
- **Verus file:** `src/kernel/src/mm/phys/mod.rs`
- **Verus dir:** `/home/ruize/nanvix-phy/src/kernel/src`
- **Parser:** tree-sitter

## Summary

| Metric | Count |
|--------|------:|
| Source exec fns | 4 |
| Verus exec fns | 4 |
| Verus spec fns | 0 |
| Verus proof fns | 0 |
| Matched | 4 |
| Missing | 0 |
| Extra | 0 |

## MISSING (exec fns in source but not in verus)

_None — all source exec fns are present in verus._

## EXTRA (exec fns in verus but not in source)

_None._

## MATCHED (exec fns present in both)

| Function |
|----------|
| `book_mmio_regions` |
| `book_physical_memory_regions` |
| `init` |
| `test` |

## SPEC/PROOF ONLY (informational)

_None._
<shellId: 23 completed with exit code 0>
```

### Command
`python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref verus-ai/bump-allocator src/kernel/src/mm/phys/mod.rs count`

```text
✅ Consistent: 4 functions, 0 structs match.
<shellId: 24 completed with exit code 0>
```

### Command
`python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref verus-ai/bump-allocator src/kernel/src/mm/phys/mod.rs summary`

```text
## Functions

Function                                 Status               Verification    
──────────────────────────────────────── ──────────────────── ────────────────
book_mmio_regions                        MATCH                                
book_physical_memory_regions             MATCH                                
init                                     MATCH                                
test                                     MATCH                                

Consistent: ✅ YES (matched=4 mismatched=0 missing=0 extra=0)
<shellId: 25 completed with exit code 0>
```

### Command
`python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff /home/ruize/nanvix-phy/src/kernel/src/mm/phys/mod.rs --before HEAD`

```text
# Spec Drift Report

## Summary

- Functions with changes: 0
- **Contract drift (⚠ review required): 0**
  - Ensures removed: 0
  - Requires added: 0
- Proof drift (informational): 0
- Functions added: 0
- Functions removed: 0

**✅ No contract drift detected.**
<shellId: 26 completed with exit code 0>
```

### Command
Cheating grep scan in module files

```text
## GREP external_body
src/kernel/src/mm/phys/mod.rs:63:// The function is therefore `external_body`; its `#[verus_spec]` contract is honored by the
src/kernel/src/mm/phys/mod.rs:70:#[verus_verify(external_body)]
src/kernel/src/mm/phys/mod.rs:96:// `external_body` for the same std-`LinkedList` reason as
src/kernel/src/mm/phys/mod.rs:105:#[verus_verify(external_body)]
src/kernel/src/mm/phys/mod.spec.rs:63:// functions are marked `external_body` (see `verus-unsupported.md`).
src/kernel/src/mm/phys/mod.spec.rs:70:#[verifier::external_body]
## GREP admit
## GREP assume/assume!
## GREP assume_specification
## GREP trusted
## GREP exec_allows_no_decreases_clause
## GREP cfg not verus_keep_ghost
## GREP cfg verus_keep_ghost
src/kernel/src/mm/phys/mod.rs:36:#[cfg(verus_keep_ghost)]
src/kernel/src/mm/phys/mod.rs:40:#[cfg(verus_keep_ghost)]
src/kernel/src/mm/phys/mod.rs:42:#[cfg(verus_keep_ghost)]
<shellId: 27 completed with exit code 0>
```

### Command
Exact module-scope counts

```text
admit_calls=0
assume_calls=0
external_body_attrs=3
assume_specification=0
trusted_attrs=0
no_decreases_attrs=0
cfg_not_keep_ghost=0
cfg_keep_ghost=3
<shellId: 28 completed with exit code 0>
```

### Command
`rg -n 'VERUS REWRITE' ...`

```text
<shellId: 29 completed with exit code 0>
```

### Command
Check TCB entry for ExLinkedList

```text
<shellId: 33 completed with exit code 0>
```
