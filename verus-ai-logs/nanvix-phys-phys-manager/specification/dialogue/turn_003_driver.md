## Turn 3: Verify Fix D2 + final full-checklist pass

### Progress
- **Done (PASS/FIXED — all 17 items):**
  1. fn coverage — PASS
  2. caller coverage — PASS (D liveness correctly dropped as inexpressible)
  3. view consistency — PASS
  4. no tautological ensures — FIXED (turn 1 Fix A)
  5. no subsumed ensures — PASS
  6. error paths meaningful — FIXED
  7. no workspace-internal assume_spec — PASS
  8. vstd searched — PASS
  9. specs written for caller — PASS
  10. trait obligations — PASS
  11. spec completeness (advisory) — PASS
  12. loop invariants — PASS
  13. no cheating on own functions — PASS (false lemma removed; remaining admits deferred-but-true)
  14. no specs weakened — PASS (B/C restored design guarantees; D corrected to strongest sound spec)
  15. bug awareness — PASS (OBS-1/2/3 recorded; OBS-3 resolved)
  16. cross-module regression — PASS
  17. verification + build — PASS
- **Current:** Fix D2 verification → CONFIRMED.
- **Remaining:** none.

### Verification performed this turn
- `make verify-kernel` → **Exit 0**, all phys modules verified (cached).
  Whole-crate cheating: `assume=0 external_body=24 admit=10 trusted=0
  no_decreases=0`. Admit count **fell 11→10** vs turn 2 — the false lemma is gone.
- `./z build -- check-kernel` → `{"reason":"build-finished","success":true}` /
  `[OK] Build complete.` (Exit 0). Non-verus dual compilation green.
- Read actual code, not just the fixer's claims:
  - `manager.rs:359` — Err arm is now `Err(_) => final(self)@ == old(self)@`
    (single sound clause; the false `&&& old(self)@.free_count() == 0` is gone).
  - `manager.rs:362-381` — body restored to `let frame_addr = frame::alloc()?;`;
    only the Ok-path `lemma_kernel_alloc_one` proof call remains.
  - `grep "free_count() == 0" manager.rs` → **no match**.
  - `grep -rn "lemma_kernel_alloc_err_empty" src/kernel/src/mm/phys/` → **no match**
    (lemma definition + both call sites deleted).
  - `manager.proof.rs` admit count → **8** (was 9); inventory: `lemma_manager_attached`,
    `lemma_free_count_bounded`, `lemma_kernel_alloc_one`,
    `lemma_kernel_alloc_contiguous`, `lemma_contig_no_overflow`,
    `lemma_user_bulk_ok`, `lemma_user_bulk_err_restored`,
    `lemma_kernel_bulk_err_restored`.
  - `bugs.md` OBS-3 → updated to **RESOLVED (spec phase, turn 2)** with the
    `kframe.rs:84` / `identity_map_page` evidence retained.

### Item 13 (no cheating) — per-lemma soundness audit of the 8 remaining admits
The turn-2 offender was admitted-**and-false**. I re-audited every remaining
admitted lemma's *contract* (bodies stay `admit()` as the legitimate spec→proof
hand-off; the question is whether each ensures is *true*, i.e. dischargeable):
- `lemma_manager_attached` (`m@ == phys_view().frames`) — true via the global
  ghost-token attachment (design §8). Deferred, true.
- `lemma_free_count_bounded` (`free_count() <= usize::MAX`) — true; finite frame
  count. Deferred, true.
- `lemma_kernel_alloc_one` (free→reserved `alloc_one`, wf preserved) — true
  Ok-path transition. Deferred, true.
- `lemma_kernel_alloc_contiguous` (contiguity + `book_all`, wf) — true; matches
  `frame::alloc_contiguous`. Deferred, true.
- `lemma_contig_no_overflow` (offset stays in `usize`) — true arithmetic under
  its `requires`. Deferred, true.
- `lemma_user_bulk_ok` (distinctness + `book_all`, wf) — true under allocator
  non-aliasing (OBS-2). Deferred, true.
- `lemma_user_bulk_err_restored` / `lemma_kernel_bulk_err_restored` (`m@ == pre`)
  — true; `clear()` drops/frees taken frames. Deferred, true.
None is unconditionally false; none is a soundness landmine. Acceptable for the
specification phase (`external_body=2` both TCB-allowed; `assume=trusted=0`).

### Item 14 (no specs weakened) — final state vs view-design §4
- `alloc_many_kernel_frames`: contiguity (`kernel_frames_contiguous`) + Err
  no-leak (`frames@.len()==0`) — **restored** (Fix B). Matches §4.5.
- `alloc_many_user_frames`: distinctness (`user_addr_set.len()==count`) —
  **restored** (Fix C). Matches §4.4.
- `alloc_kernel_frame`: Err arm `final(self)@ == old(self)@` — this is the
  **strongest sound** statement. The §4.1 `free_count()==0` liveness fact is
  genuinely inexpressible at `FrameAllocView` (wrap-failure Err mode is invisible
  to the view); correcting an incorrect over-strong spec to the strongest correct
  one is exactly what item 14 mandates ("if a spec is incorrect, replace it with
  an equally-strong correct spec"). Not a weakening — a correctness fix.
- `init`: Err arm `manager_ready` — meaningful, sound (Fix A).

### Result: ALL ITEMS PASS/FIXED
Fix D2 is correct and complete. The unsound clause and its false admitted lemma
are removed; every Err arm now states the strongest sound postcondition; the two
bulk paths retain their restored design guarantees; `make verify-kernel` and the
non-verus build are both Exit 0; the whole-crate admit count fell to 10 with no
remaining false lemma. Bugs are recorded (OBS-1/2 as proving-phase obligations,
OBS-3 resolved). Writing STOP = RESOLVED.
