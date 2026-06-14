## Turn 1: Specification review — full checklist walk

Module: `arch::x86::mem::paging::table`
Files: `table.rs`, `table.spec.rs`, `table.proof.rs`
Verus command: `make verify-arch` → **exit 0** (cached, `dev` profile finished/builds).

### Progress
- Done (PASS): #7 no assume_specification, #8 vstd-before-assume (N/A), #12 loop
  invariants (no loops), #14 no specs weakened (drift=0), #15 bug awareness,
  #16 cross-module regression, #17 verification+build.
- PASS for the *verified* fns (`new`, `into_raw`, `pd_index`, `pt_index`,
  `from_address`): #4 no tautology, #5 no subsumed (minor note), #6 error paths.
- **Current FAIL: #1 every in-scope exec fn has requires/ensures** — drives the
  fix request below. Dependent FAILs: #2 (partial), #6 (read/write arm), #9, #10,
  #11, and #13 (own-fn external_body without contract).
- Remaining: re-verify after fix.

### Verification (commands run + evidence)

**`make verify-arch`** → exit 0. Tool summary:
```
cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0
coverage: 7/525 exec functions have contracts
status: CHEATING_DETECTED
cheating-detail:
  table.rs:195 read: external_body
  table.rs:215 write: external_body
```

**`fn_coverage.py table.rs table.rs`** → 7 source exec fns, 7 matched, 0 missing:
`from_address, into_raw, new, pd_index, pt_index, read, write`.

**`spec_drift.py check nanvix-phys-arch-paging-table`** → 0 contract drift. ✅

**grep cheating in module files** → only `external_body` ×2 (`read`, `write`);
no `admit`/`assume`/`trusted`/`assume_specification` in `table.{rs,spec.rs,proof.rs}`.

**Per-function spec inventory (read from `table.rs`):**

| Fn | requires/ensures present? | Verdict |
|----|---------------------------|---------|
| `TableIndex::new` | ✅ `Some => in-range + t@==index`, `None => out of range` | PASS |
| `TableIndex::into_raw` | ✅ `result as nat==self@`, `result < LEN` | PASS |
| `pd_index` | ✅ `result@==spec_pd_index`, `result@<LEN` | PASS |
| `pt_index` | ✅ `result@==spec_pt_index`, `result@<LEN` | PASS |
| `Table::from_address` | ✅ `result@.addr==base` | PASS |
| `Table::read` | ❌ **none** (external_body) | **FAIL** |
| `Table::write` | ❌ **none** (external_body) | **FAIL** |
| `TableEntry::from_raw` (trait, in-scope) | ❌ none | **FAIL** |
| `TableEntry::raw` (trait, in-scope) | ❌ none | **FAIL** |

Minor note (#5): `into_raw`'s `result < LEN` is partly derivable from
`result as nat==self@` + the `type_invariant`. It is the caller-facing bound
`gva.rs` needs for `checked_mul`, so I accept it as a deliberate convenience,
not a subsumption defect.

### The core finding (#1, #2, #9, #10, #11, #13)

`read`, `write`, and the `TableEntry` trait methods `raw`/`from_raw` carry **no
requires/ensures at all**. Per `caller_analysis.md` the callers depend on:
- `read`: `Some(e)` = valid decode, `None` = invalid encoding (→ `InvalidArgument`);
- `write` then `read`: read-after-write round-trip; only slot `index@` changes;
- `TableEntry` round-trip law `spec_from_raw(e.spec_raw()) == Some(e)`.

**None of these are expressed.** `read`/`write` are contract-free trust
boundaries: callers get *nothing* usable from them, violating "specs written for
the caller" (#9) and "trait obligations satisfied" (#10).

`view_design.md` ("As-Built Decision") defers all of this to a future
permission layer, justifying it by: *threading a `with`-clause ghost permission
parameter onto `read`/`write` would cascade into out-of-scope `admit()` callers
(`identity_map::ensure_pt/ensure_pte/identity_map_page`)*.

**That justification does not survive the precedent this module explicitly
claims to mirror.** `verus-unsupported.md`, `bugs.md`, and `tcb-allowed.md` all
say `read`/`write` mirror `bump_allocator::alloc` and `frame::instance`. I read
both:

- `frame.rs:668-675` — `instance()` is `#[verus_verify(external_body)]` yet
  carries a full `#[verus_spec]`: `ensures (*r).inv(), (*r)@ == phys_view().frames,
  phys_view().initialized`. It references a **global ghost view function**
  (`phys_view()`), and adds **no tracked parameter** to the signature.
- `bump_allocator/src/lib.rs:271-285` — `alloc()` is `external_body` with a full
  `requires bump_view(self).inv()` + structured `Ok`/`Err` `ensures`.

So the accepted pattern for an int-to-ptr trust boundary in *this very codebase*
is: **external_body body is trusted, but the function still carries a complete
abstract contract** — and the contract is expressed against a **global ghost
state function**, **without** threading any permission parameter. That means the
cascade argument in `view_design.md` is a strawman: it assumed a *parameter-
threaded* PointsTo token, but `frame::instance` shows the contract can reference
a parameter-free global ghost view, which does **not** change `read`/`write`
signatures and therefore does **not** cascade into the `admit()` callers.

`external_body` itself (#13) is an acceptable trust boundary here (genuine Verus
`usize -> *const/*mut` limitation, reproduced in `verus-unsupported.md`, logged in
`tcb-allowed.md`). The defect is that it is **contract-free**, unlike every
precedent it cites.

### Fix Request

**Goal:** give `read`, `write`, and the `TableEntry` trait methods real,
caller-usable contracts, following the `frame::instance` precedent (external_body
+ full `#[verus_spec]` referencing a *global, parameter-free* ghost state), with
**no exec signature change** so out-of-scope `admit()` callers do not cascade.

Concretely:

1. In `table.spec.rs`, model page-table memory as a global ghost view keyed by
   table base address — analogous to `phys_view()`. For example a `spec fn`
   returning the per-slot map for a given `addr`, plus restore the
   `entries`/`spec_read`/`spec_write` shapes from `view_design.md`'s target
   design (the `Map<nat, Option<E>>` model). Add the `TableEntry` round-trip law
   as a trait-level spec:
   `spec fn spec_raw(self) -> PteWord;`
   `spec fn spec_from_raw(w: PteWord) -> Option<E>;`
   with the proof obligation/assumed law `spec_from_raw(e.spec_raw()) == Some(e)`.

2. Add `#[verus_spec]` to `TableEntry::from_raw` / `TableEntry::raw` in
   `table.rs` tying exec results to `spec_from_raw` / `spec_raw`.

3. Add `#[verus_spec]` to `Table::read` (keep `external_body`):
   `requires index@ < PAGE_TABLE_LENGTH` (auto from `TableIndex::inv`),
   `ensures result == <global ghost read of (self@.addr, index@)>`.

4. Add `#[verus_spec]` to `Table::write` (keep `external_body`):
   `ensures` the global ghost state at `(self@.addr, index@)` becomes
   `spec_from_raw(entry.spec_raw())` and every other slot / address is unchanged
   (frame condition). This is what realizes the read-after-write round-trip the
   caller analysis requires.

**Do NOT** add a tracked/`with` permission *parameter* to `read`/`write`'s exec
signatures — use a parameter-free global ghost state exactly like
`frame::instance` so the out-of-scope `admit()` callers keep compiling.

**Verify with:**
- `make verify-arch` → must stay exit 0 with **all** modules verified.
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/fn_coverage.py
  src/libs/arch/src/x86/mem/paging/table.rs
  src/libs/arch/src/x86/mem/paging/table.rs` → unchanged 7/7.
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py check
  nanvix-phys-arch-paging-table` → must remain 0 drift (you are *adding*
  guarantees, never removing).
- grep `read`/`write`/`from_raw`/`raw` in `table.rs` → each must now show a
  `#[verus_spec(... ensures ...)]`.

**If you believe step 1–4 genuinely cannot be done without changing
`read`/`write` exec signatures**, that is a justification, not a fix — prove it:
attempt it and paste the concrete `make verify-arch` compiler/Verus error that
shows the cascade into `identity_map`. Only that tool output would justify
escalating the root cause to the view-design phase. A prose explanation will be
rejected.

### Checklist status after Turn 1
- PASS: #4, #5(note), #6(new), #7, #8, #12, #14, #15, #16, #17.
- **FAIL (fix requested): #1, #2(partial), #6(read/write arms), #9, #10, #11,
  #13(contract-free external_body).**
- STOP file **not** created — open FAILs remain.
