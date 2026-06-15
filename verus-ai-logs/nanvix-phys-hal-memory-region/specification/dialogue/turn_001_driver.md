## Turn 1: No cheating on module's own functions (admit in `MemoryRegion::start`)

### Progress
- Done (PASS this turn):
  - **Every in-scope exec function has requires/ensures** — the 4 verification-order
    targets all carry `ensures`:
    - `MemoryRegion::start` → `spec_addr(&result) == self@.start` (region.rs:210-217)
    - `MemoryRegion::size` → `result as int == self@.size` (region.rs:228-231)
    - `TruncatedMemoryRegion::start` → `spec_addr(&result) == self@.start` (region.rs:379-386)
    - `TruncatedMemoryRegion::size` → `result as int == self@.size` (region.rs:392-395)
  - **Caller coverage** — checked against `caller_analysis.md`. Callers need
    `start() == self@.start` and `size() == self@.size` as abstract ints for frame/overlap
    math; the ensures supply exactly that. Page-alignment / no-overflow expectations are
    captured by `inv()` (`wf_geometry` + `start/size % spec_page_size == 0`), derivable by
    callers that carry `inv()`. Adequate.
  - **View consistency** — checked against `view_design.md`. Specs reference `self@.start`
    / `self@.size`; `View` impls + `inv()` match the design (closed forwarding view on the
    truncated variant, `spec_addr` projection, `wf_geometry`). Consistent.
  - **No tautological ensures** — accessors are infallible (no `Result`); no `Err(_) => true`.
  - **No subsumed ensures** — `result as int == self@.size` pins the exact value, not
    derivable from `inv()` alone.
  - **Error paths** — N/A: the 4 targets are infallible reads.
  - **No assume_specification for workspace-internal code** — region.{rs,spec,proof}.rs
    contain zero `assume_specification` (grep confirmed).
  - **Loop invariants** — no loops in the module.
  - **No specs weakened (spec drift)** — `region.spec.rs` was previously an empty
    `verus! { }` stub (per `caller_analysis.md`); the phase only *adds* View/inv/helpers.
    Nothing weakened.
  - **Cross-module regression / Verification** — `make verify-kernel` exits 0 (all modules
    pass, cached).
- Current: **No cheating on module's own functions** → **FAIL** (see below).
- Remaining (re-check after fix): final cheating-pattern recount + build_cmd.

### Verification
Ran `make verify-kernel`:
```
Exit code : 0
cheating: assume=0 external_body=25 admit=1 trusted=0 no_decreases=0 cfg_gate=7
status: CHEATING_DETECTED
```
`verus-logs/cheating-detail.txt`, filtered to this module:
```
- hal/mem/types/region.rs:218 start: admit
```
All 25 `external_body` entries are in *other* modules (page.rs, frame.rs, manager.rs,
mod.rs, etc.) — external dependencies / TCB-allowed, out of scope here.

Source confirmation (region.rs:218-225):
```rust
pub fn start(&self) -> T {
    proof! { admit(); }   // region.rs:223
    self.start.clone()
}
```
`admit()` discharges the obligation `spec_addr(&self.start.clone()) == self@.start` by
fiat. The ensures only *appears* verified because of the admit. This is a cheat on the
module's **own** function — `admit` is NOT in the "assume_specification on external
dependencies temporarily allowed" exception.

The accompanying comment ("admit() is the specification-phase placeholder ... discharged
in the proving phase") is a **justification, not a fix**. Per review rules, a justification
does not clear the item — the code must change.

Root of the obligation: `spec_addr` is `uninterp` (page.spec.rs:31), and `start()` returns
`self.start.clone()`. With no clone spec in scope, Verus cannot relate
`spec_addr(&clone) == spec_addr(&orig)`. This is fixable **in this phase** without `admit`,
because `Address: Clone` (src/libs/sys/src/sys/mm/address/mod.rs:32) and a value-preserving
clone spec can be introduced via `assume_specification` on the external `Clone` impl — the
same trust-boundary pattern already used in `page.spec.rs` for
`<PageAligned<T> as Address>::into_raw_value`.

### Fix Request
**File:** `src/kernel/src/hal/mem/types/region.rs` and `region.spec.rs`.

1. In `region.spec.rs`, inside the `verus! { ... }` block, add a value-preserving clone
   specification for the `Address` family (external dependency — temporarily allowed),
   e.g.:
   ```rust
   // Address clone is value-preserving (newtype identity): the clone has the
   // same abstract address as the original.
   pub assume_specification<T: Address>[ <T as Clone>::clone ](addr: &T) -> (result: T)
       ensures
           spec_addr(&result) == spec_addr(addr),
   ;
   ```
   (Use whatever `Clone` path Verus accepts for the blanket `T: Address`; mirror the
   existing `assume_specification<T: Address> [ <PageAligned<T> as Address>::into_raw_value ]`
   form in `page.spec.rs`.)

2. In `region.rs`, **delete** the `proof! { admit(); }` line (and its placeholder comment)
   from `MemoryRegion::start`. The body becomes just `self.start.clone()`, and the ensures
   `spec_addr(&result) == self@.start` must now discharge from the new clone spec plus the
   `View` definition (`self@.start == spec_addr(&self.start)`).

**Do NOT** keep `admit()` and explain why it is acceptable. Remove it.

**Verify:**
```bash
make verify-kernel
grep -nE "region\.(rs|spec\.rs|proof\.rs)" verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt
```
Required outcome: `make verify-kernel` exits 0 **and** the `cheating-detail.txt` grep prints
**no** `region.rs ... admit` line (own-module `admit` count for this module = 0). If a
different/weaker spec is substituted to dodge the obligation, that will be rejected — the
ensures `spec_addr(&result) == self@.start` must remain exactly as strong.
