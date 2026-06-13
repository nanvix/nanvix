## Turn 1: Full checklist walk — primary FAIL = tautological ensures (`UserFrame::share` `Err(_) => true`)

### Progress
- Done (PASS / accepted): items 1, 3, 5, 7, 8, 9, 11, 12, 13, 14, 15, 16, 17 (see Verification).
- Current FAIL (fix requested this turn): **#4 No tautological ensures** — `UserFrame::share` has `Err(_) => true`. Also drives **#6 Error paths meaningful** for `share`.
- Remaining open (documented as upstream/inexpressible, see notes): #2 caller coverage (refcount-transition), #6 for `drop`, #10 trait `release` semantics.

### Verification (commands run + evidence)

`make verify-kernel` → exit 0. `note: verifying module mm::phys::upool`, `cargo Finished dev profile`, cached, **0 errors**. Cheating summary (kernel-wide): assume=0, external_body=26, admit=10, trusted=0. Per-module `cheating-detail.txt` for upool:
```
mm/phys/upool.rs:218 Upool (struct): external_body
mm/phys/upool.rs:243 new: external_body
mm/phys/upool.rs:276 alloc: external_body
```
→ upool itself has **admit=0, assume=0, trusted=0**; only 3 `external_body` (the `Upool` facade + `new` + `alloc`), all on `tcb-allowed.md`. `UserFrame` methods are NOT external_body (genuinely verified).

`fn_coverage.py upool.rs upool.rs` → 7 source exec fns, 7 verus exec fns, 7 matched, 0 missing/extra (`new, address, leak, share, refcount, drop, alloc`).

`spec_drift.py git-diff --before 975741f6 (view-design end) --after HEAD` → **Ensures removed: 0**; requires added: 5; functions added: 1; all changes are additions. No mechanical spec weakening vs. the git baseline.

`grep admit|assume|assume_specification|trusted upool*.rs/.spec.rs/.proof.rs` → none. No loops in module. `upool.proof.rs` is empty (`verus! { }`).

Source semantics confirmed by reading `Inner::share` (frame.rs:368): it returns `Err` **iff** frame number out of bounds, `refcount==0` (unallocated), or `checked_add` overflow (`refcount==255`). On `Ok` the frame is allocated and was `< 255`. So an honest, snapshot-only (no `old()`) Err arm is *soundly* expressible.

Per-item dispositions:

1. **Every exec fn has requires/ensures** — PASS (7/7 carry `#[verus_spec]`). Note: `drop` carries only `opens_invariants none / no_unwind` (its partition effect is inexpressible, see #6/#10).
2. **Caller coverage** — PARTIAL/accepted. Round-trip (`new`/`address`/`leak`: `result@ == addr@/self@`) and `share`/`refcount` snapshot facts match caller expectations. The refcount-**transition** expectations (CoW "survives until both drop": `share`=`add_ref`, `drop`=`release`, `leak`=suppress-release) are NOT captured because they require `old(phys_view())`, which is not valid Verus (`phys_view()` is a 0-arg `uninterp spec fn`; `grep` finds zero `old(phys_view(...))` usages anywhere in the tree). This is an upstream modeling limitation (the global accessor lives in `mod.spec.rs`, do-not-modify), already documented in `view_design.md §8` and `bugs.md`. Not fixable in the upool spec phase and consistent with the already-verified frame layer.
3. **View consistency** — PASS. Specs reference `self@`, `phys_view().frames`, and `inv()`; `inv()` (`self@ % spec_page_size() == 0`) is used as pre/postcondition.
4. **No tautological ensures** — **FAIL** → `UserFrame::share` `Err(_) => true`. Actionable (see Fix Request).
5. **No subsumed ensures** — PASS (minor): `result.inv()` on `new/address/leak/share` is derivable from `result@ == src@` + caller `inv()`. Harmless for caller ergonomics; not blocking.
6. **Error paths meaningful** — **FAIL for `share`** (`Err(_) => true`). `refcount` Err arm is meaningful (`!allocated_frames.contains(self@)`); `alloc` Err arm is meaningful. `drop` has no ensures, but its partition effect is inexpressible (see #2/#10) — not counted as a fixable FAIL.
7. **No assume_specification for workspace-internal code** — PASS (assume=0; none present).
8. **vstd searched before assume_specification** — PASS (N/A; no assume_specification).
9. **Specs written for the caller** — PASS. Snapshot facts (`allocated_frames.contains`, `refcounts[self@]`, `alloc_one`, `free_count()==0`) are directly usable in caller proofs.
10. **Trait obligations** — PARTIAL/accepted. `Drop`: `drop` delegates to `frame::free` and carries `no_unwind`/`opens_invariants none`. The semantic `release` contract is inexpressible (same `old(phys_view())` limitation, and `frame::free` itself ensures `true`). `View for UserFrame` = `int` (address) matches caller reasoning.
11. **Spec completeness (advisory)** — nondeterminism (unmodeled refcount transition) matches the deferral noted in caller expectations; advisory only.
12. **Loop invariants** — PASS (no loops).
13. **No cheating on module's own functions** — PASS. upool: admit=0, assume=0, trusted=0; `external_body`×3 = `Upool` struct + `Upool::new` + `Upool::alloc`, all TCB-allowed (opaque pool facade / allocation primitive). `UserFrame` methods are verified.
14. **No specs weakened** — PASS (mechanical): spec_drift shows 0 ensures removed, additions only.
15. **Bug awareness** — PASS. `bugs.md` records no code bugs and documents the deferred refcount-transition modeling.
16. **Cross-module regression** — PASS. `make verify-kernel` exit 0; upool 6 verified / 0 errors; other modules cached (no regressions).
17. **Verification + build** — PASS. verify exit 0, 0 errors; the verify run compiled the kernel (`cargo Finished dev profile`).

### Fix Request (item #4, drives #6 for `share`)

`UserFrame::share`'s `Err(_) => true` is the textbook tautological ensures the checklist forbids. It is the *honest ceiling only because* `frame::share` (the delegate, `external_body`/trusted) currently also ensures `Err(_) => true`. Reading `Inner::share` (frame.rs:368) proves a stronger Err fact is **true** and snapshot-expressible (no `old()` needed). Make these two changes:

1. In `src/kernel/src/mm/phys/frame.rs`, strengthen the trusted `external_body` wrapper `frame::share` (line ~806) Err arm:
   ```rust
   ensures
       match result {
           Ok(())  => crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@),
           Err(_)  => !crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@)
                       || crate::mm::phys::phys_view().frames.refcounts[frame@] >= 255,
       },
   ```
   This is sound: `Inner::share` returns `Err` exactly on out-of-bounds / `refcount==0` (⟹ not in `allocated_frames`) or overflow (⟹ `refcount==255`).

2. In `src/kernel/src/mm/phys/upool.rs`, mirror it in `UserFrame::share` (line ~147), replacing `Err(_) => true`:
   ```rust
   Err(_) => {
       !crate::mm::phys::phys_view().frames.allocated_frames.contains(self@)
       || crate::mm::phys::phys_view().frames.refcounts[self@] >= 255
   },
   ```
   (`self.addr@ == self@`, so the delegate's `frame@`-fact transfers directly through `frame::share(self.addr)?`.)

Do NOT attempt to add `F' == F.add_ref(self@)` or `phys_view() == old(phys_view())` — `old()` cannot be applied to the 0-arg `phys_view()` accessor; that transition layer is the upstream limitation documented in §2 above and is out of scope for this phase.

Verify after the change:
- `make verify-kernel` → must remain exit 0 with `mm::phys::upool` 0 errors (the new upool Err arm must discharge from the new `frame::share` Err arm).
- Re-run `spec_drift.py git-diff --before 975741f6 --after HEAD` → still 0 ensures removed (this is a strengthening).
- Confirm `UserFrame::share` no longer contains `Err(_) => true`.

Paste the `make verify-kernel` tail and the `share` spec block as evidence. A justification ("frame layer only gives `true`") is NOT acceptable — the frame wrapper is trusted/`external_body` and the stronger fact is provably true from `Inner::share`, so change the code.
