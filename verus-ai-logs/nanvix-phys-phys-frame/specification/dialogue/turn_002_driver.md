## Turn 2: Verify Turn-1 fix (tautological/error-path ensures) → RESOLVED

### Progress
- Done (PASS/FIXED): all 17 checklist items (see verdict table).
- Current: verified the fixer's Turn-1 changes to the 4 flagged Err arms.
- Remaining: none.

### Verification

**Fixer's claim audited against actual code (not trusted verbally).**
`grep -n "Err(_) =>" frame.rs` confirms the four edits landed and only the two
permitted `Err(_) => true` remain:
- `alloc` (748): `Err(_) => phys_view().frames.free_frames.is_empty()` ✅ (OOM witness)
- `book` (890): `Err(_) => !phys_view().frames.free_frames.contains(phys_addr@)` ✅
- `alloc_range` (914): `Err(_) => !PhysMemView::region_frames(region@.start, region@.size).subset_of(phys_view().frames.free_frames)` ✅
- `share` (942): `Err(_) => !allocated_frames.contains(frame@) || (refcounts.contains_key(frame@) && refcounts[frame@] >= 255)` ✅
Each matches the corresponding `Inner::*` Err arm and the caller-relied facts in
`caller_analysis.md`. Read in context (lines 910–945, 204–207) — correct.

**Two retained `Err(_) => true` justified by QUOTED contracts, not prose:**
- `alloc_contiguous` (792): verified `Inner::alloc_contiguous` Err arm (frame.rs
  205–207) is `final(self)@ == old(self)@` and nothing else. With no
  `old(phys_view())` to diff, the strongest expressible shim arm is `true`
  (`phys_view().inv()` already stated above the match). Accepted.
- `init` (686): its own contract Err arm; on failure the singleton was not
  established, `phys_view().inv()` (stated unconditionally) is the only guarantee.
  TCB Skip/exclude. Accepted.

**`make verify-kernel`** → exit 0, cached (edited source already verifies), all 5
`mm::phys` modules green, `assume=0 trusted=0`. **`make all-kernel`** → exit 0,
`kernel.elf` produced. Both run by me.

**Spec drift re-checked** (`git-diff --before 7c792c75`): reports 3 "ensures
removed" for `alloc`/`book`/`alloc_range`. Inspected each — they are the
text-diff chunks where `Err(_) => true` + comment were replaced by a **stronger**
predicate; the Ok arms are intact. These are strengthenings, NOT weakenings.
No original guarantee removed.

### Full checklist verdict
1. Function coverage — PASS (11/11, `fn_coverage.py`)
2. Caller coverage — PASS (Err facts now present for alloc/book/alloc_range/share)
3. View consistency — PASS (only View fields; `inv()` on every path)
4. No tautological ensures — **FIXED** (only 792/686 remain, contract-justified)
5. No subsumed ensures — PASS
6. Error paths meaningful — **FIXED**
7. No internal `assume_specification` — PASS (only one, on external `Address`)
8. vstd before assume_spec — N/A (none internal)
9. Specs for caller — PASS
10. Trait obligations — PASS (`free`: `opens_invariants none` + `no_unwind`, no requires)
11. Spec completeness (advisory) — acceptable; shim nondeterminism matches caller needs
12. Loop invariants — PASS (all loops inside `external_body`)
13. No cheating on own functions — counts reported: `frame.rs` external_body=10,
    admit=7; spec.rs/proof.rs have zero admit/assume/external_body. The
    external_body on exec fns + `admit()` in shims are the **expected
    specification-phase scaffold** (the whole `mm::phys` subsystem's green
    baseline carries them); they are discharged in the proof phase. No cheating in
    the spec definitions themselves. PASS for this phase.
14. No specs weakened — PASS (drift "removed" are strengthenings; verified)
15. Bug awareness — PASS (no fundamentally incorrect code; refcount/partition
    semantics in `Inner::*` are sound and match `wf`)
16. Cross-module regression — PASS (`make verify-kernel`: 31 verified, 0 errors)
17. Verification + build — PASS (`make verify-kernel` exit 0; `make all-kernel` exit 0)

### Fix Request
None — all items PASS or FIXED with tool-backed evidence.

### Result: RESOLVED
