# Final Verification Review — `mm::phys::frame` (`phys-frame`)

## Checklist
- [ ] **Caller Analysis** — **31/34 covered** for public API expectations; 3 missing are `frame::free` transition semantics (last-release/free, decrement-on-shared, double-free failure) not encoded in shim contract (`caller_analysis.md:108-110` vs `frame.rs:881-896`).
- [x] **View Design** — `FrameAllocView`/`PhysMemView`/`PhysAuth` design is coherent and matches current shim contracts (`view_design.md:37-45`, `view_design.md:149-229`, `frame.rs:731-1073`).
- [ ] **Specification** — generally strong, but top-level `frame::free` spec is intentionally weak (only `phys_view().inv()`) and does not capture caller-observed success/failure semantics (`frame.rs:881-896`).
- [x] **Proving** — `admit()` count is 0 in `frame.rs/spec/proof`; module and full verification commands exit 0.
- [x] **Cheating Elimination** — `assume=0`, `admit=0`, `assume_specification=0`; all `external_body` in `frame.rs` are listed in TCB; no disallowed cfg-gated exec branches.
- [x] **Bug Recording** — `bugs.md` reconciled; it records limitations/trust-boundaries, no confirmed code defects.

## Spec Quality
Evidence inspected in `src/kernel/src/mm/phys/frame.rs`:
- All in-scope exec functions have `#[verus_spec]` (script check below).
- Strong contracts on mutating shims (`alloc`, `alloc_contiguous`, `book`, `alloc_range`, `share`) use `old(auth)@ -> final(auth)@` transitions (`frame.rs:731-805`, `930-1037`).
- Query shims (`free_count`, `is_covered`, `refcount`) are precise (`frame.rs:825-842`, `909-924`, `1048-1070`).
- `Inner::*` contracts are detailed and meaningful (`frame.rs:115-565`).

Findings:
1. **No tautological `Err(_) => true`** in scoped targets.
2. **Redundancy/subsumption present but harmless** (e.g., some `contains` facts derivable from `final(auth)@ == old(auth)@.spec_*`).
3. **Missing top-level error/success semantics for `frame::free`**: spec only ensures invariant preservation/no_unwind/opening discipline (`frame.rs:881-894`), not refcount transition or failure characterization expected by callers/tests (`caller_analysis.md:108-114`).

Function contract presence/requires/ensures check:
```text
python3 ... check in-scope functions for verus_spec/requires/ensures
...
free#2 fn_line=896 verus_spec=True requires=False ensures=True
(all others: verus_spec=True requires=True ensures=True)
```
(`free` being precondition-free is deliberate for `Drop`.)

## Caller Coverage
**Covered 31 / 34** public-function expectations from `caller_analysis.md`.

### Covered (examples by API)
- `alloc`: fresh allocated frame, refcount=1, unchanged-on-error (`caller_analysis.md:62-66` ↔ `frame.rs:747-755`).
- `alloc_contiguous`: `count>0`, contiguous page-stride set reserved, unchanged-on-error (`caller_analysis.md:74-80` ↔ `frame.rs:782-805`, `182-208`).
- `free_count`: finite free set and exact count (`caller_analysis.md:90-94` ↔ `frame.rs:840-842`).
- `is_covered`: iff covered set membership (`caller_analysis.md:118-123` ↔ `frame.rs:923`, `512-515`).
- `book`, `alloc_range`, `share`, `refcount`: success/failure expectations mapped to concrete ensures (`frame.rs:945-955`, `984-995`, `1024-1035`, `1063-1069`).
- Drop-safety constraints for `free` covered: no requires, `opens_invariants none`, `no_unwind`, invariant preserved (`caller_analysis.md:49-56`, `105-107` ↔ `frame.rs:881-894`).

### MISSING
1. `free` success transition “last reference returns to free pool” (`caller_analysis.md:108-110`) — **not in `frame::free` shim spec** (`frame.rs:881-896`).
2. `free` success transition “shared frame decrements refcount” (`caller_analysis.md:108-110`) — **not in shim spec**.
3. `free` failure characterization “double-free fails” (`caller_analysis.md:109-110`) — **not in shim spec**.

(These exist in `Inner::free` contract (`frame.rs:262-287`) but are not exposed at the top-level `frame::free` API because that shim is `external_body` for `Drop` compatibility.)

## Proof Completeness
- `admit()` in `frame.rs`, `frame.spec.rs`, `frame.proof.rs`: **0**.
  - Command: `rg -n "\badmit\(" ...` → no matches.
- `external_body` across same files: **11** (all in `frame.rs`):
  - `Inner::alloc` (line 137)
  - `Inner::alloc_contiguous` (210)
  - `Inner::free` (290)
  - `Inner::share` (368)
  - `Inner::refcount` (428)
  - `Inner::book` (481)
  - `Inner::is_covered` (517)
  - `Inner::alloc_range` (565)
  - `instance` (652)
  - `init` (689, excluded target)
  - `frame::free` shim (896)
- `external_body` not in TCB allowed list: **0**.

## TCB Compliance
**YES** (for `frame.rs`).

Mapping to `tcb-allowed.md`:
- `frame.rs::init` → skip/exclude entry (`tcb-allowed.md:37`).
- `frame.rs::instance` → allowed entry (`tcb-allowed.md:55-56`).
- `Inner::{alloc, alloc_contiguous, free, share, refcount, book, is_covered, alloc_range}` → allowed grouped entry (`tcb-allowed.md:68-70`).
- `frame.rs::free` shim → allowed `Drop`-path exception (`tcb-allowed.md:89-107`).

## Guardrails Compliance
Counts for frame module files (`frame.rs`, `frame.spec.rs`, `frame.proof.rs`):
- `admit`: **0**
- `assume`: **0**
- `external_body`: **11**
- `assume_specification`: **0**
- cfg-gated exec violations: **0**
  - Raw cfg markers found: 2 (`#[cfg(verus_keep_ghost)]` includes at `frame.rs:49,52`), which are explicitly allowed (include/import gating only).

## AST Consistency
**PASS**.

Evidence:
- `ast_consistency.py --base-ref HEAD src/kernel/src/mm/phys/frame.rs count` → `✅ Consistent: 19 functions, 1 structs match.`
- `summary` shows all target functions `MATCH`.

`VERUS REWRITE` inspection:
- Only rewrite comment: `frame.rs:845-857` (`free_count`).
- Original idiom `inner.bitmap.number_of_bits() - inner.bitmap.usage()` rewritten into named temporaries `nbits`, `used`, then `nbits - used` (`frame.rs:858-863`).
- Semantics preserved: same two pure queries + same subtraction, no branch/control-flow change, no side effects introduced.

## Verification
Commands run from repo root:
1. `make verify-kernel MODULE=mm::phys`
   - Exit code: **0**
   - Verifier errors: **0** (no error diagnostics; command result `Exit code : 0`).
2. `make verify`
   - Exit code: **0**
   - Verifier errors: **0** across invoked crates (all steps showed `Exit code : 0`).

(Outputs were cached/no recompilation, but both verification gates completed successfully.)

## Bug Summary
- Recorded entries in `bugs.md`: **2 thematic entries**
  1. Resolved spec-architecture limitation via `PhysAuth` carrier (`bugs.md:9-57`) — **fixed**, not a code defect.
  2. Remaining `Drop`-only `free` trust-boundary exception (`bugs.md:58-75`) — **still valid**, documented limitation/trust boundary, not a code defect.
- True code bugs confirmed in this review: **0**.
- Newly found but unrecorded **documentation drift** (not code defect): `caller_analysis.md` still says `alloc_contiguous`/`free_count` shims have no specs (`caller_analysis.md:83-84`, `96-97`), but current `frame.rs` has both specs (`frame.rs:774-806`, `825-842`).

## Issues (highest priority first)
1. **High** — Top-level `frame::free` contract does not encode caller-observed transition/failure semantics (3 missing caller expectations).
2. **Low** — `caller_analysis.md` stale notes about unspecced shims (`alloc_contiguous`, `free_count`).

## Result: FAIL
`FAIL` because checklist items are not all satisfied (Caller Analysis and Specification unchecked due missing top-level `frame::free` semantic coverage), despite clean proving/TCB/guardrail/verification results.

---

### Key command evidence (raw snippets)
```text
$ python3 .../spec_drift.py git-diff .../frame.rs --before HEAD
Functions with changes: 0
Contract drift: 0
✅ No contract drift detected.

$ python3 .../fn_coverage.py .../frame.rs .../frame.rs --markdown
Source exec fns: 11
Matched: 11
Missing: 0
```
