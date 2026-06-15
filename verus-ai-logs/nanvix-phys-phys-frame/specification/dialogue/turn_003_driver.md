## Turn 3: Verify fixer's PhysAuth implementation → ALL ITEMS PASS/FIXED → RESOLVED

### Progress
- Done (PASS/FIXED): all 17 checklist items (table below).
- Current: confirmed the end-to-end `PhysAuth` carrier; ran verifier + build myself.
- Remaining: none.

### Verification (commands I ran, files I read — claims not trusted verbally)

**`make verify-kernel`** (whole kernel crate, all `mm::phys` modules) → **exit 0**,
"verified, 0 errors", `cheating: assume=0 external_body=23 admit=0 trusted=0`.
**`make all-kernel`** → exit 0, `bin/kernel.elf` produced (dual compilation intact;
the `Tracked` carrier is erased in the non-Verus build).

**Spec drift** (`git diff 4bc3f039… -- src/kernel/src/mm/phys/`): additive only —
`mod.spec.rs` +111 (the `PhysAuth` carrier + `spec_alloc_one/_set/_share/_free`
transitions; **nothing removed**), `frame.rs` shims rewritten, `upool.rs` callers
threaded. I grepped for removed `Inner::*` contract lines: the only `^-` match is a
comment. **The frozen `Inner::*` `old(self)@→final(self)@` contracts are
unchanged.** No guarantee weakened — Ok arms strengthened from pre-state to
post-state.

**Soundness of the re-sync mechanism (the crux I scrutinized).** Pattern:
```
let r = instance();            // ensures (*r)@ == phys_view().frames
let res = r.op(..);            // Inner::op: final(r)@ == old(r)@.<transition>
proof! { auth.v.frames = (*r)@; }   // re-sync carrier to true post-state
```
with shim `requires old(auth)@ == phys_view()`. At entry the precondition + the
`instance()` bridge pin `old(auth)@.frames == old(r)@` (real pre-state); the
re-sync sets the carrier to `final(r)@` (real post-state, constrained by the
verified `Inner::*` contract). So `final(auth)@ == old(auth)@.spec_<op>(..)` is a
**true fact about the real allocator** — the carrier adds NO new trust beyond the
already-TCB instance() bridge + Inner::* contracts, exactly as `view_design.md`
states ("PhysAuth carries NO new caller-visible content"). The constant
`phys_view()` is read once at entry (where it is true), never re-assumed
post-mutation, so the rollback's "live ≠ constant after mutation" unsoundness does
not arise. Sound.

> Defensible deviation from `view_design.md`: the fixer kept `instance()` unchanged
> and used the re-sync proof + `old(auth)@ == phys_view()` precondition instead of
> `instance(Tracked(&mut PhysAuth))`. The observable shim contracts are identical to
> the design's strong post-state, with zero change to the frozen `instance()`/`Inner::*`
> contracts — a strictly more minimal realization. Accepted. (Latent, non-blocking:
> because `phys_view()` is a fixed constant, the carrier cannot chain two mutations
> from the same baseline; no verified in-scope caller chains mutations — `Upool::alloc`,
> `UserFrame::share`/`refcount` each do one — and the loop allocators are TCB
> `external_body`, so this never bites.)

### Per-item verdict (all PASS/FIXED)

1. **Coverage** — PASS. Every in-scope exec fn has requires/ensures; all 8
   free-fn shims (`alloc`,`alloc_contiguous`,`book`,`alloc_range`,`share`,
   `free_count`,`is_covered`,`refcount`) are **body-verified** (absent from the
   cheating list).
2. **Caller coverage** — FIXED. `Upool::alloc` (upool.rs:282-288) now ensures
   `final(auth)@ == old(auth)@.spec_alloc_one(uf@) && allocated_frames.contains(uf@)
   && refcounts[uf@]==1` — exactly `caller_analysis.md`'s "now in allocated_frames,
   refcount 1". `UserFrame::share`/`refcount` likewise strengthened.
3. **View consistency** — FIXED. `mod.spec.rs` implements the approved
   `tracked PhysAuth` (view=`PhysMemView`, `inv()==v.inv()`) + transition fns;
   shims reference `FrameAllocView` fields and preserve `inv()`.
4. **No tautological ensures** — FIXED. `alloc`/`alloc_contiguous` Err arms are now
   `final(auth)@ == old(auth)@` (meaningful). Sole remaining `Err(_) => true`
   (frame.rs:686) is `init`, an `external_body` TCB function whose only failure
   guarantee is `phys_view().inv()` (singleton not established) — contract-justified.
5. **No subsumed ensures** — PASS. Post-state membership facts are not derivable
   from `inv()` alone; they require the transition equality.
6. **Meaningful error paths** — PASS. `book`/`alloc_range`/`share` Err arms add
   `final(auth)@==old(auth)@` + a negative-state fact; alloc(_contiguous) state unchanged.
7. **No assume_specification (internal)** — PASS (assume=0).
8. **vstd searched before assume_specification** — PASS (none used).
9. **Specs usable by caller** — FIXED. The verified `Upool::alloc`/`UserFrame::*`
   prove the strong post-state from the threaded shims (verifier accepts, 0 errors).
10. **Trait obligations** — PASS. `free` keeps the `Drop` contract
    (`opens_invariants none`, `no_unwind`, no `requires`); both `Drop` impls verified.
11. **Spec completeness (advisory)** — PASS. The previously-forced weakening is
    gone; remaining nondeterminism (`free` Drop path) matches caller expectations
    ("callers don't care about the precise refcount value").
12. **Loop invariants** — PASS. All loops (frame.rs:219,578,603,718) are inside
    `external_body` `Inner::*`/`init`; no body-verified shim contains a loop.
13. **No cheating on module's own functions** — PASS. `admit=0`, `assume=0`,
    `trusted=0`. Every in-scope `external_body` individually checked against
    `tcb-allowed.md`: `Inner::*` (8, §2 untranslatable `error!`/arch newtypes),
    `instance` (§1 static-mut bridge), `init` (§ manager facade), and the **new**
    `free` (§"Allowed external_body — frame::free", tcb-allowed.md:89) — the
    Drop-fixed signature provably cannot discharge `instance()`'s precondition
    without a token, so `external_body` with always-true `ensures phys_view().inv()`
    is the governed mechanism (this is exactly my turn-2 fix #4: remove `admit`,
    state a Drop-compatible always-true contract). The 8 reservation/query free-fn
    shims are body-verified, not external_body.
14. **No specs weakened** — FIXED. Reservation shims strengthened to post-state;
    spec-drift confirms additive-only changes, frozen contracts intact.
15. **Bug awareness** — PASS. `bugs.md` rewritten: the `phys_view()`-constant
    limitation marked **RESOLVED** via the carrier; `free` Drop-only exception +
    manager `external_body` boundary documented as retained.
16. **Cross-module regression** — PASS. `make verify-kernel` verifies all kernel
    `mm::phys` modules (frame, kframe, manager, upool, mod), exit 0.
17. **Verification + build** — PASS. `make verify-kernel` 0 errors; `make all-kernel`
    builds `kernel.elf`.

### Conclusion

Every checklist item is PASS or FIXED with tool-backed evidence. The fixer
implemented the approved `view_design.md` carrier end-to-end (additive, no frozen
contract weakened), the verified mutating path + its verified callers now carry the
caller-required strong post-state, the `admit` is eliminated (`admit=0`), and the
residual `external_body=23` is entirely TCB-sanctioned per `tcb-allowed.md`. Both
`make verify-kernel` (0 errors) and `make all-kernel` (kernel.elf) succeed.

**Verdict: RESOLVED.** STOP file created.
