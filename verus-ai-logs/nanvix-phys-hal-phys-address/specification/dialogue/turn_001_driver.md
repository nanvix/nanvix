## Turn 1: Full checklist walk-through (specification phase, `hal-phys-address`)

In-scope target functions (per `verus-ai.toml` / `view_design.md`):
`PhysicalAddress` (type + `inv`), `from_number`, `into_frame_number`, `from_mmio_address`.

### Progress
- **PASS:** fn-coverage, view-consistency (partial), no-subsumed (advisory), no-assume-on-internal,
  vstd-searched, specs-for-caller (mmio/into_frame_number), trait-obligations, loop-invariants (N/A),
  no-specs-weakened (spec_drift), bug-awareness, cross-module regression, verification exit code.
- **FAIL:** (1) admit on own functions, (2) tautological `Err(_) => true` / meaningless error path,
  (3) `from_number` ensures not usable by caller (missing `inv()` / alignment, not derivable).
- **Remaining:** none unverified — every item determined below.

---

### Verification (commands run)

- `make verify-kernel` → exit 0, but pipeline reports **`status: CHEATING_DETECTED`**,
  `cheating: assume=0 external_body=25 admit=2 trusted=0`. Module verified: `hal::mem::types::address::phys`.
- `cheating-detail.txt` pins the two admits to THIS module:
  - `hal/mem/types/address/phys.rs:154 from_number: admit`
  - `hal/mem/types/address/phys.rs:169 into_frame_number: admit`
- `tcb-allowed.md` → neither `from_number` nor `into_frame_number` is listed. They are **not** TCB-allowed.
- `fn_coverage.py phys.rs phys.rs` → 16/16 exec fns matched, 0 missing/extra.
- `spec_drift.py git-diff phys.rs --before <view-design end sha> --after HEAD` →
  contract drift = additions only (requires/ensures **added**, 0 removed). No prior guarantee weakened.
- Source/spec/proof read in full. `spec_page_size()` is an **uninterpreted** `spec fn`
  (`frame.rs:34`) with **no global positivity axiom**; `spec_page_size() > 0` only exists as a local
  `requires`/assumption inside `mm/phys/*.proof.rs`. `VirtualAddress`/`Address` are re-exported from the
  external `::sys` crate (not kernel-internal).

---

### Checklist results

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | Every in-scope exec fn has requires/ensures | PASS | `from_mmio_address` (req+ens), `from_number` (ens), `into_frame_number` (req+ens) all annotated. |
| 2 | Caller coverage | **FAIL** | `from_number` result must carry `inv()` + page-alignment for its sole caller `FrameAddress::from_frame_number` (→ `PageAligned::from_address` needs alignment; later `into_frame_number` needs `inv()`). Current ensures provides neither, and they are **not derivable** (see FAIL-3). |
| 3 | View consistency (specs ref View fields, maintain `inv()`) | PARTIAL | `from_mmio_address` ensures `r.inv()` ✓; `into_frame_number` requires `self.inv()` ✓. `from_number` does **not** establish `inv()` on its result → invariant not maintained by this constructor (FAIL-3). |
| 4 | No tautological ensures | **FAIL** | `from_mmio_address` ensures `Err(_) => true` — the exact tautology the checklist names. |
| 5 | No subsumed ensures | PASS (advisory) | `from_mmio_address`'s `r.inv()` is derivable from `r@==addr@` + the `requires`, but exposing the type invariant on a constructor's `Ok` result is the canonical idiom and reduces caller burden — acceptable. |
| 6 | Error paths meaningful | **FAIL** | Same as #4: `from_mmio_address` body is `Ok(Self(addr))` (infallible), yet the spec leaves `Err` unconstrained instead of guaranteeing success. |
| 7 | No assume_specification for workspace-internal code | PASS | All `assume_specification`s target external crates `::arch` (FRAME_SIZE/SHIFT, FrameNumber) and `::sys` (VirtualAddress::new, `<VirtualAddress as Address>::into_raw_value`). Mirrors existing edges in `page.spec.rs:50` / `kframe.spec.rs:33` / `frame.rs:36`. |
| 8 | vstd searched before assume_specification | PASS | All assumed items are domain types/constants (FrameNumber, VirtualAddress, FRAME_SIZE/SHIFT) with no vstd equivalent. |
| 9 | Specs written for the caller | **FAIL** | `from_number` spec is not directly usable (FAIL-3). `into_frame_number` / `from_mmio_address` are usable. |
| 10 | Trait obligations satisfied | PASS | In-scope fns are inherent; `Address` trait methods are out of scope and untouched. |
| 11 | Spec completeness (advisory) | PARTIAL | `from_number` under-specified (no inv/alignment); otherwise complete. Fold into FAIL-3. |
| 12 | Loop invariants | PASS (N/A) | No loops in any in-scope function. |
| 13 | No cheating on own functions | **FAIL** | `admit()` in `from_number` (l.155) and `into_frame_number` (l.170). Own in-scope functions, not external deps, not in `tcb-allowed.md`. The current "verify PASS" is vacuous for these two. |
| 14 | No specs weakened (spec_drift) | PASS | Only additions; 0 ensures/requires removed vs view-design end. |
| 15 | Bug awareness | PASS | Logic of `from_number` (`frame*FRAME_SIZE`) and `into_frame_number` (`>>FRAME_SHIFT` + `unwrap`) is sound under `inv()`. No fundamental bug; nothing to record in bugs file. |
| 16 | Cross-module regression | PASS | `make verify-kernel` exit 0; all modules cached-PASS (only cheating flag, no verification failure). |
| 17 | Verification + build | PASS (with cheating flag) | verify exit 0; cargo build `Finished dev profile`. The PASS is undermined by the admits (FAIL-13). |

---

### Fix Requests

#### FIX-1 (FAIL-13) — Remove `admit()` from the module's own functions
`from_number` and `into_frame_number` each contain `proof! { admit(); }`. These discharge all proof
obligations vacuously, so the module's "verify PASS" is meaningless for them, and neither is listed in
`verus-ai-logs/tcb-allowed.md`.

- **`into_frame_number` (phys.rs:170):** delete `proof! { admit(); }`. The obligation is the
  `.unwrap()` totality of `FrameNumber::from_raw_value(raw_addr >> FRAME_SHIFT)`. Discharge it from
  `requires self.inv()` (= `spec_frame_number(self@) <= spec_max_frame_number()`) plus the
  `FRAME_SHIFT`/`into_raw_value`/`from_raw_value` assume_specs — tie `raw_addr >> FRAME_SHIFT` to
  `self@ / spec_page_size()` (a bit-shift ↔ division lemma) so `from_raw_value` returns `Some`.
- **`from_number` (phys.rs:155):** delete `proof! { admit(); }`. Prove
  `result@ == spec_frame_raw_value(frame) * spec_page_size()` from `into_raw_value` + `FRAME_SIZE`
  assume_specs and `VirtualAddress::new`, handling the `usize` multiply (overflow) obligation.

Justification ("this is the proof phase's job") is **not** an acceptable resolution — the admits are in
scope for this module and the checklist forbids them. Verify with:
`make verify-kernel` → the summary line must show `admit=0` (currently `admit=2`).

#### FIX-2 (FAIL-3 / FAIL-2/coverage) — Make `from_number`'s contract usable by its caller
`from_number`'s ensures is only `result@ == spec_from_number(spec_frame_raw_value(frame))`. Its sole
caller needs (a) page-alignment for `PageAligned::from_address`, and (b) `result.inv()` so the resulting
`FrameAddress` can later call `into_frame_number`. **Neither is derivable**, because `spec_page_size()`
is uninterpreted with no positivity axiom in this module (confirmed: `spec_page_size() > 0` exists only
as local assumptions in `mm/phys/*.proof.rs`, not here). Add to `from_number`'s `ensures`:

```
result.inv(),
result@ % spec_page_size() == 0,
```

If you instead claim these are derivable, prove it: add a `spec_page_size() > 0` axiom/boundary in
`phys.spec.rs` (or import one) and show a caller-side proof. Do not leave the caller unable to discharge
`PageAligned::from_address`'s alignment `?` and `into_frame_number`'s `requires self.inv()`.
Verify with `make verify-kernel` (module still PASS) and confirm `FrameAddress::from_frame_number` /
`into_frame_number` proofs can consume the result without re-deriving the bound.

#### FIX-3 (FAIL-4 / FAIL-6) — Replace the tautological `Err(_) => true` in `from_mmio_address`
The body is `Ok(Self(addr))` — infallible — so `Err(_) => true` is dead and tautological. Strengthen the
ensures to guarantee success and drop the meaningless error arm, e.g.:

```
ensures
    result is Ok,
    result matches Ok(r) ==> (r@ == addr@ && r.inv()),
```

(or equivalent `result.is_ok()` + `result->Ok_0@ == addr@ && result->Ok_0.inv()`). Keep the existing
`requires spec_frame_number(addr@) <= spec_max_frame_number()`. Verify with `make verify-kernel`.

---

### Decision
Three FAIL items remain (admit-on-own-functions, tautological/meaningless error ensures, unusable
`from_number` contract). **STOP not created.** Awaiting fixes; will re-verify each with
`make verify-kernel` and direct file reads on the next turn.
