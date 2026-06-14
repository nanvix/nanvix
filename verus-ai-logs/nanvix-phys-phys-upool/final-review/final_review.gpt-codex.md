# FAIL — `phys-upool` final verification review (independent)

## 1) Spec quality (8 in-scope functions)
- `UserFrame::new`: has alignment precondition + address postcondition (`addr.inv()`, `ret@ == addr@`) but does not encode caller-expected “no alloc/refcount change” frame condition (`upool.rs:54-64`; expectation `caller_analysis.md:53-54`).
- `UserFrame::address`: good getter contract (`ret@ == self@`) (`upool.rs:78-82`), matching caller need (`caller_analysis.md:58-63`).
- `UserFrame::leak`: only `ret@ == self@`; missing explicit “no free / stays allocated” effect (`upool.rs:96-102` vs `caller_analysis.md:66-69`).
- `UserFrame::share`: success arm captures aliasing (`handle@ == self@`) but not explicit refcount +1; error arm is tautological `Err(_) => true` (`upool.rs:124-142`) versus caller expectations refcount increment + failure atomicity (`caller_analysis.md:75-80`).
- `UserFrame::refcount`: meaningful success/Err relation (`upool.rs:171-178`), including Err => not allocated, matching stated error meaning (`caller_analysis.md:85-87`).
- `UserFrame::drop`: only invariant preservation (`phys_view().inv()`), not “releases exactly one reference” (`upool.rs:188-195` vs `caller_analysis.md:29-33,94`).
- `Upool::new`: **no `#[verus_spec]` contract at all** (`upool.rs:233-236`), missing caller-facing “no mutation” guarantee (`caller_analysis.md:98-99`).
- `Upool::alloc`: success facts present (allocated/refcount=1), but Err arm is tautological `Err(_) => true` (`upool.rs:247-265`), missing caller-required failure atomicity (`caller_analysis.md:110`).

Assessment on `Err(_) => true` (`share`, `alloc`): understandable under single-state `phys_view()` style, but still a real API-spec gap for callers needing failure-atomic guarantees.

## 2) Caller coverage
**Caller coverage: 5/13 expectations covered.**
- Covered: `new` address identity, `address` getter identity, `share` aliasing-on-success, `refcount` success meaning, `alloc` success allocated/refcount=1 (`upool.rs:62,81,137,172-176,258-263`).
- **Missing list**: `new` no-state-change, `leak` no-free/stays-allocated, `share` refcount+1, `share` failure-atomic/parent-untouched, `refcount` explicit no-state-change purity, `drop` exactly-one-release, `Upool::new` no-state-change contract, `alloc` failure-atomic/no-allocation.

## 3) Proof completeness
- `upool.rs` has **no** `admit`, `assume`, or `external_body` (`evidence.gpt-codex.log:17-23,295`).

## 4) TCB compliance
- Added trust in touched code is in `frame.rs` shim functions `alloc/free/share/refcount` (`frame.rs:710,762,849,874`), and these are explicitly allow-listed (`tcb-allowed.md:54-64`).
- `upool.rs` now has zero `external_body` (`evidence.gpt-codex.log:295`).
- Docs stale: TCB file still lists `UserFrame::drop` as external_body (`tcb-allowed.md:66-79`), and bugs log repeats that (`bugs.md:11-15`), but current `drop` is non-external (`upool.rs:185-198`).

## 5) AST consistency
- AST check passes: 8/8 functions MATCH, 2/2 structs MATCH (`evidence.gpt-codex.log:265-289`).
- `// VERUS REWRITE` count is 0 (`evidence.gpt-codex.log:23`).

## 6) Verification status (central result + independent static corroboration)
- Central `make verify-kernel MODULE=mm::phys` reported PASS (provided by orchestrator).
- Independent corroboration: no local contract drift vs HEAD (`evidence.gpt-codex.log:70-83`), and no do-not-modify symbol diffs detected (`evidence.gpt-codex.log:291-293`).

## 7) Guardrails counts
**Guardrails counts (upool.rs): admit=0, assume=0, external_body=0, assume_specification=0, cfg-gated exec (`cfg(not(verus_keep_ghost))`)=0.** (`evidence.gpt-codex.log:17-23`)

## 8) Bug reconciliation
- `bugs.md` says “None” (`bugs.md:1-7`) — no concrete code bug found in these 8 functions.
- Reconciliation:
  - “drop is external_body” note is **stale/fixed** (`bugs.md:11-15` vs `upool.rs:185-198`).
  - “single-state `phys_view()` limits before/after specs” note remains valid context (`bugs.md:17-22`).
- Unrecorded issue: spec completeness gaps above (quality/caller-contract issues, not a demonstrated runtime code bug).

## Independent checks vs provided claims
- `fn_coverage`: 7/7 matched — confirmed (`evidence.gpt-codex.log:35-43`).
- `spec_drift` vs HEAD: 0 drift — confirmed (`evidence.gpt-codex.log:75-83`).
- `spec_drift` vs base: ensures removed=0, requires added=9 — confirmed (`evidence.gpt-codex.log:93-95`).
- Diff stat confirms `upool.rs` “124 changed lines”; numstat is 106 insertions / 18 deletions (`evidence.gpt-codex.log:7,14`).

## Blockers
1. `Upool::new` missing contract (`upool.rs:233-236`) for an in-scope function.
2. `UserFrame::share` lacks caller-required failure-atomic and refcount+1 guarantees (`upool.rs:124-142`; `caller_analysis.md:75-80`).
3. `Upool::alloc` lacks caller-required failure-atomic/no-allocation guarantee (`upool.rs:257-265`; `caller_analysis.md:110`).
4. `UserFrame::{leak,drop}` do not specify caller-required ownership effects (no-free / exactly-one-release) (`upool.rs:96-102,188-195`; `caller_analysis.md:66-69,29-33`).

## Issues/nits
- Stale documentation: `tcb-allowed.md` and `bugs.md` still describe `UserFrame::drop` as `external_body` though code no longer does (`tcb-allowed.md:66-79`; `bugs.md:11-15`; `upool.rs:185-198`).
