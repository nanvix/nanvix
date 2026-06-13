## Turn 2: Re-review of turn 1 FAIL + build verification → RESOLVED

### Progress
- Done (PASS/FIXED):
  1. **In-scope coverage** — FIXED/PASS (see Verification: trait-declaration contract).
  2. Caller coverage — PASS.
  3. View consistency — PASS.
  4. No tautological ensures — PASS.
  5. No subsumed ensures — PASS.
  6. Error paths meaningful — N/A (all in-scope fns total/infallible).
  7. No `assume_specification` for workspace-internal code — PASS (none in virt.*).
  8. vstd searched before `assume_specification` — N/A.
  9. Specs written for caller — PASS.
  10. Trait obligations satisfied — PASS.
  11. Spec completeness (advisory) — PASS (3/3 in-scope fns specified).
  12. Loop invariants — N/A (no loops).
  13. No cheating on module's own functions — PASS.
  14. No specs weakened — PASS.
  15. Bug awareness — PASS (no fundamentally incorrect in-scope code).
  16. Cross-module regression — PASS.
  17. Verification + build — FIXED/PASS (build was RED, now green).
- Current: none — all items closed.
- Remaining: none.

### Verification

**Item 1 — in-scope coverage (my turn-1 FAIL was WRONG; fixer correct).**
The fixer reverted my requested `#[verus_verify]`-on-impl change and showed it cannot compile.
I verified every claim:
- `mod.rs:30` — `#[verus_verify] pub trait Address`. `mod.rs:51-55` — `into_raw_value` carries
  `#[verus_spec(result => ensures result as int == self@)]` **on the trait declaration**. In Verus a
  trait-method contract binds all implementers, so `VirtualAddress::into_raw_value` is specified and
  callers (`mm/mmio.rs`, `pm/sync.rs`) get `result as int == self@`. My turn-1 grep only scanned
  `virt.rs` and missed this.
- Established pattern confirmed: the already-verified sibling `PhysicalAddress`
  (`src/kernel/src/hal/mem/types/address/phys.rs:167`) has `impl Address for PhysicalAddress` **also
  unannotated**, with `into_raw_value` (line 262) carrying no per-impl spec — identical design.
- Annotating the impl forces all-or-nothing trait-impl verification, which pulls in `as_ptr`/`as_mut_ptr`
  (`usize as *const u8` — a real Verus front-end limitation) and the out-of-scope align methods. My
  turn-1 instruction was therefore incorrect; the trait-declaration contract is the right mechanism.
- `new` (`result@ == value as int`) and inherent `from_raw_value` (`result@ == raw_addr as int`) are
  specified in `virt.rs:49,66`. Round-trip `from_raw_value(x).into_raw_value() == x` is provable from
  these + the trait ensures. → **PASS.**

**Item 17 — build (NEW FAIL found this turn, now fixed).**
The fixer's claim "normal cargo build compiles, duplicate vstd import is pre-existing/unrelated" was
FALSE. `./z build` (the project build, `-D warnings`) **failed**:
```
error: unused import: `::vstd::prelude::*`  --> src/libs/sys/src/sys/mm/address/virt.rs:25:5
error: could not compile `sys` (lib) due to 1 previous error
```
`git blame` proved it is NOT pre-existing: line 8 `use vstd::prelude::*;` was added by **this pipeline**
(commit `ca7e88be8`, 2026-06-14, the `include!` scaffolding), duplicating the original line 25
`use ::vstd::prelude::*;` (de24f60573, 2026-04-23). The scaffolding introduced the collision.
Fix = remove the redundant duplicate import. Verified the fix:
- `./z build` → `[OK] Build complete.` (exit 0).
- `make verify-sys` → exit 0, **6 verified, 0 errors**, `cheating: assume=0 external_body=0 admit=0
  trusted=0` for virt.
This fix is now committed at HEAD `26902c59f` (single `use vstd::prelude::*;` at line 8, no duplicate).

**Cross-module regression (item 16).** `make verify` → exit 0, all modules verify. Kernel's
`external_body=14/admit=26` are pre-existing in the kernel crate, **not** in `virt.*` (which is 0/0/0/0).

**Cheating (item 13).** `virt.rs`/`virt.spec.rs`/`virt.proof.rs`: `assume=0 external_body=0 admit=0
trusted=0 no_decreases=0`. `cfg_gate=1` is a crate-wide `#[cfg(target_pointer_width)]`, not in virt.

### Fix Request
None — all checklist items PASS/FIXED with tool evidence. Creating STOP = RESOLVED.
