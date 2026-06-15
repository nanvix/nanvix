## Turn 1: Full checklist walk — FAIL on `is_aligned` tautological / non-meaningful `Err` arm

### Progress
- **Done (PASS / verified):** items 1, 3, 5, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17
- **Current (FAIL):** items **4 (No tautological ensures)** and **6 (Error paths have meaningful ensures)** — both rooted in `is_aligned`'s `Err(_) => true`
- **Advisory:** item 2 (caller coverage — one gap, same root), item 11 (spec completeness — same root)
- **Remaining:** none unverified; the only blocking defect is the `is_aligned` Err arm

### Environment fix applied (precondition for any verification)
`make verify-sys` initially **failed to compile** (`vstd` `atomic.rs`: "expected generics to match … found bool", `could not compile vstd`). Root cause: the Makefile default `VERUS_EXECUTABLE_DIR=/home/ruize/toolchain/verus` is Verus `0.2026.06.14`, incompatible with the pinned `vstd =0.0.0-2026-05-31-0205` (`Cargo.toml:231`, `build/verus-version` = `0.2026.05.31.5dd6d83`). Installed the pinned release from cache:
```
./scripts/setup/verus.sh /home/ruize/toolchain/verus-pinned-0531
VERUS_EXECUTABLE_DIR=/home/ruize/toolchain/verus-pinned-0531 make verify-sys
```
All results below use the pinned Verus.

---

### Verification — per checklist item

**1. Every in-scope exec function has requires/ensures — PASS.**
In-scope = the three `Address` trait method declarations. Read `mod.rs`:
- `from_raw_value` (`mod.rs:59-66`) — `#[verus_spec]` ensures present (Ok/Err match).
- `into_raw_value` (`mod.rs:72-76`) — ensures `result as int == spec_addr(&self)`.
- `is_aligned` (`mod.rs:126-133`) — ensures present.
Out-of-scope (`align_up`, `align_down`, `max_addr`, `as_ptr`, `as_mut_ptr`) correctly carry no specs (per `caller_analysis.md`). The whole-crate `fn_coverage` metric (2/254) is not the in-scope granularity — trait declarations have no bodies; their ensures are confirmed by direct read.

**2. Caller coverage — PASS with one gap (see item 4/6).** Cross-checked `caller_analysis.md`:
- `from_raw_value`: Ok ⟹ `a@ == raw` + valid; Err ⟹ `BadAddress`. ✓ (`spec_addr(&a) == raw_addr as int && addr_inv(&a)`; `e.code == BadAddress`).
- `into_raw_value`: `result as int == self@`. ✓
- `is_aligned`: `Ok(b) ⟺ b == addr_is_aligned(...)`. ✓ **but** callers treat it as **total** (`caller_analysis.md:106` "the `Err` arm (no concrete implementor returns it)"; `mprotect.rs:74`, `munmap.rs:66`, `heap.rs:194` `debug_assert!`). The current spec does **not** capture totality — gap addressed in item 4/6.

**3. View consistency — PASS (documented deviation).** `view_design.md` prescribes `self@`/`inv()`. The spec instead uses the universal projection `spec_addr` + `addr_inv` (`mod.spec.rs:41,48`). This is a **justified, documented** deviation: `Address` cannot carry a `View<V=int>` supertrait (per-impl `View` impls are `cfg(verus_keep_ghost)`-gated → unsatisfiable in plain `cargo build`; would also form a definition cycle — `bugs.md` §1, `mod.spec.rs:13-40`). `addr_inv` preserves the exact `view_design` bound `0 <= spec_addr <= usize::MAX`. Acceptable.

**5. No subsumed ensures — PASS (note).** `from_raw_value` Ok arm states `addr_inv(&a)` which is technically derivable from `spec_addr(&a) == raw_addr as int` + `raw_addr: usize`. Retained intentionally: it is the caller-facing invariant, idiomatic to assert on construction. Not blocking.

**7/8. No assume_specification for workspace-internal code; vstd searched first — PASS / N/A.** No `assume_specification` in this module (grep over `address/` finds the term only in comments).

**9. Specs written for the caller — PASS (note).** Generic `T: Address` callers (`PageAligned<T>`, `MemoryRegion<T>`) consume the `spec_addr`-based ensures directly. `VirtualAddress` concrete callers still need the kernel's existing `assume_specification` (`phys.spec.rs`) until `spec_addr` is bridged to `VirtualAddress@` downstream — acceptable for this phase (`bugs.md` §2).

**10. Trait obligations satisfied — PASS** (except the `is_aligned` Err looseness, item 4/6).

**12. Loop invariants — N/A.** No loops in trait declarations.

**13. No cheating — PASS.** Tool: `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0` for `sys`. grep of `address/` finds `admit/assume/external_body/trusted` only inside comments — zero in code.

**14. No specs weakened — PASS.** `git diff ac8020f1a~1..HEAD -- virt.spec.rs virt.rs` is **empty** (existing `VirtualAddress::new`/inherent `from_raw_value` specs unchanged). `mod.spec.rs`/`mod.proof.rs` went from empty `verus!{}` stubs to **additions** only. Kernel `assume_specification` for `into_raw_value` preserved; kernel re-verified 47/0.

**15. Bug awareness — PASS.** `bugs.md` present; documents the cycle resolution and kernel coexistence; no code bugs (declarations only).

**16. Cross-module regression — PASS.** `VERUS_EXECUTABLE_DIR=…verus-pinned-0531 make verify` → bitmap **70/0**, sys **6/0 (CLEAN)**, nanvix-slab **35/0**, bump-allocator **0 errors**, kernel **47/0**. The `CHEATING_DETECTED` status on non-sys crates is **pre-existing** (`external_body=24 cfg_gate=6` in kernel; present at baseline commit `d791be2cd`) and unrelated to this module. **sys is CLEAN.**

**17. Verification + build — PASS.** `make verify-sys` → **6 verified, 0 errors, status CLEAN**. The sys crate compiled successfully under the verus cargo driver (a superset of `cargo build`); the design deliberately avoids a `View` supertrait so a normal `cargo build` is unaffected (`mod.spec.rs:13-20`).

---

### FAIL — items 4 & 6: `is_aligned` has `Err(_) => true`

`mod.rs:126-133`:
```rust
#[verus_spec(result =>
    ensures
        match result {
            Ok(b) => b == addr_is_aligned(spec_addr(self), align),
            Err(_) => true,        // <-- tautological, non-meaningful
        },
)]
fn is_aligned(&self, align: Alignment) -> Result<bool, Error>;
```

`Err(_) => true` is the **exact pattern the checklist prohibits** (item 4) and is a **non-meaningful error path** (item 6). It is also self-contradictory with the author's own comment two lines above (`mod.rs:125`: *"Concrete implementors never take the `Err` arm"*) and with `caller_analysis.md:106` and `view_design.md:175-177`: the contract **permits** an `Err` that the design asserts **never occurs**, and as a result callers who guard alignment (`if !addr.is_aligned(k)? { … }`, `debug_assert!(addr.is_aligned(k))` in `mprotect`/`munmap`/`heap`/`PageAligned`) cannot prove the call succeeds.

A strictly stronger, non-tautological, caller-faithful spec exists. This is **not** the rejected "model the Err arm with state" alternative — it is the opposite: assert the Err arm is unreachable.

### Fix Request

In `src/libs/sys/src/sys/mm/address/mod.rs`, replace the `is_aligned` ensures (lines 126–132) with a form that **forbids** the `Err` arm:

```rust
#[verus_spec(result =>
    ensures
        result is Ok,
        result->Ok_0 == addr_is_aligned(spec_addr(self), align),
)]
fn is_aligned(&self, align: Alignment) -> Result<bool, Error>;
```

(Equivalently, `result matches Ok(b) ==> b == addr_is_aligned(spec_addr(self), align)` **plus** `result is Ok` — the point is the `Err(_) => true` clause must be removed and replaced by an unreachability guarantee, not a vacuous one.)

Rationale this is correct and safe:
- `impl Address for VirtualAddress::is_aligned` returns `Ok(self.is_aligned(align))` unconditionally (`virt.rs:243-245`) — `result is Ok` holds.
- `caller_analysis.md` confirms every concrete implementor (`PhysicalAddress`, `PageAligned`, `PageTableAligned`) forwards and never errors, so the strengthened obligation is dischargeable when those crates are verified.
- `Alignment` is a closed enum of valid powers of two, so there is no genuine error condition to describe — making "never errors" the *only* honest, non-tautological contract.

If you believe the trait must keep an `Err` arm for an actual failure mode, then `justification is not a fix` — you must instead specify a **meaningful** `Err` ensures (e.g. an error code, as `from_raw_value` does) and point to the implementor path that produces it. Otherwise apply the `result is Ok` strengthening above.

Verify after the change:
```
VERUS_EXECUTABLE_DIR=/home/ruize/toolchain/verus-pinned-0531 make verify-sys
```
Expect: `6 verified, 0 errors, status CLEAN`, and confirm `grep -n "Err(_) => true" src/libs/sys/src/sys/mm/address/mod.rs` returns nothing.

### Decision
Checklist **NOT** fully PASS — items 4 and 6 FAIL. **No STOP file written.** Fixer must apply the `is_aligned` change above and re-run `make verify-sys`; Turn 2 will verify the diff and tool output.

---

## Turn 2: Verify fix for items 4 & 6 — FIXED

### Change applied
`src/libs/sys/src/sys/mm/address/mod.rs` `is_aligned` ensures changed from the
tautological `match { Ok(b) => …, Err(_) => true }` to:
```rust
#[verus_spec(result =>
    ensures
        result is Ok,
        result->Ok_0 == addr_is_aligned(spec_addr(self), align),
)]
fn is_aligned(&self, align: Alignment) -> Result<bool, Error>;
```

### Verification (tool output, not claims)
- `grep -n "Err(_) => true" src/libs/sys/src/sys/mm/address/mod.rs` → **NONE** (tautology removed).
- `make verify-sys` (pinned Verus) → **6 verified, 0 errors, status CLEAN**, `cheating: assume=0 external_body=0 admit=0 trusted=0`.
- Cross-module re-check `make verify-kernel` → **47 verified, 0 errors, exit 0** (the `external_body=24 cfg_gate=6` is pre-existing baseline, identical to commit `d791be2cd`; not introduced by this change). The strengthened `result is Ok` does not break any downstream consumer.

Item 4 (No tautological ensures) → **FIXED.**
Item 6 (Error paths have meaningful ensures) → **FIXED** (`is_aligned` now guarantees `Ok`; `from_raw_value` Err already meaningful `BadAddress`).
Item 2 / 11 advisory gaps (is_aligned totality) → **closed** by the same change.

### Final decision
All checklist items are **PASS or FIXED** with tool-backed evidence. Writing `STOP = RESOLVED`.
