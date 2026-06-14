## Turn 3: Verify fixer turn 2, re-walk checklist

### Progress
- FIXED (carried): Verus runs (parse OK).
- FIXED this turn:
  - **Inner::refcount** now verifies (the turn-2 precondition error at frame.rs:452 is gone; the
    fixer added the unaligned-address rejection guard).
  - **Inner::book** now verifies (turn-2 postcondition error at frame.rs:497 is gone).
  - **frame.spec.rs assume_specification removed** — `grep assume_specification
    src/kernel/src/mm/phys/frame.spec.rs` → none (only comments). See note below on relocation.
- PASS (unchanged): external_body all TCB-listed; no cfg-gated exec; cheating audit reported;
  limitation reproducers documented.
- **Current FAILs (still open):**
  1. **FAIL — Zero admit()**: 3 admits remain — `alloc`(frame.rs:137), `alloc_contiguous`(214),
     `alloc_range`(679).
  2. **FAIL — Verification 0 errors**: 5 errors — 3 in the new transition lemmas in
     `frame.proof.rs`, plus `Inner::free` and `Inner::share` postconditions.
- BLOCKED until 1–2 fixed: spec-drift (no weakening), exec-rewrite equivalence, cross-module
  regression (kernel: 78 verified / 5 errors).

---

### Verification (run by me this turn)

`make verify-kernel`:
```
verification results:: 78 verified, 5 errors   (was 71/4 in turn 2)
status: VERIFY_FAILED (exit 101)
cheating: assume=0 external_body=19 admit=19 ...
```

**Admits — STILL PRESENT** (`grep -n admit frame.rs` + cheating-detail.txt):
- `frame.rs:137` `Inner::alloc` — `proof! { admit(); }`
- `frame.rs:214` `Inner::alloc_contiguous` — `proof! { admit(); }`
- `frame.rs:679` `Inner::alloc_range` — `proof! { admit(); }`  (was line 613; shifted by edits)
These are why `alloc`/`alloc_contiguous`/`alloc_range` do **not** appear among the 5 errors — the
admits still mask their unproven specs. NOT fixed.

**The 5 verification errors** (all in the frame module proof work):
| Loc | Site | Kind | Root cause |
|---|---|---|---|
| frame.proof.rs:341 / :344 | `lemma_reserve_one_v` | assertion failed (`rc2[fnx]==1u8`, `rc2[i]==rc[i]`) | lemma requires `0 <= fnx < nb` but **not** `nb <= rc.len()`; without it `rc.update(fnx,_)` may be a no-op and `rc[i]` may be out of bounds |
| frame.proof.rs:430 | `lemma_release_one_v` | assertion failed (`rc2[i]==rc[i]`) | same missing `nb <= rc.len()` bound |
| frame.proof.rs:478 / :481 | `lemma_update_refcount_v` | assertion failed (`rc2[fnx]==nv`, `rc2[i]==rc[i]`) | same missing `nb <= rc.len()` bound |
| frame.rs:270 | `Inner::free` | postcondition not satisfied | depends on `lemma_release_one_v` (currently unproven) |
| frame.rs:359 | `Inner::share` | postcondition not satisfied | depends on `lemma_update_refcount_v` (currently unproven) |

There is also a trigger-inference warning at frame.proof.rs:406/291 (the
`forall|a:int| ... .contains(a)` quantifiers auto-pick a trigger) — fix the bound first; if the
assertion still doesn't go through, add an explicit `#![trigger]`.

**Confirmed lemma signatures (frame.proof.rs):**
- `lemma_reserve_one_v` (line 270): `requires spec_page_size()>0, addr%PS==0, fnx==addr/PS,
  0<=fnx<nb, !sb.contains(fnx)` — **no `nb <= rc.len()`**.
- `lemma_release_one_v` (line 358) and `lemma_update_refcount_v` (line 443): same gap.

The view's `refcounts` value reads `rc[addr/PS]` and the lemma bodies read `rc2[fnx]`/`rc2[i]`; all
require the index `< rc.len()`. `internal_inv` already guarantees
`self.refcount@.len() >= self.bitmap@.num_bits` (frame.spec.rs:49), so the bound is available at
every call site — it just isn't threaded into the lemmas.

---

### Note: assume_specification relocation (frame.spec.rs → page.spec.rs)
The fixer removed the two intra-crate `assume_specification`s from `frame.spec.rs` and added them to
`src/kernel/src/hal/mem/types/address/aligned/page.spec.rs:29,36`
(`<PageAligned<T> as Address>::into_raw_value`, `<PageAligned<T> as Deref>::deref`).
- For the **phys-frame proving target**, frame.spec.rs is now clean → the checklist item is
  satisfied **for this module**.
- This is a relocation to `PageAligned`'s own home module, consistent with the established
  pattern in `verus-ai-logs/tcb-allowed.md` ("Intra-crate ... trusted only until the HAL
  address/region layer is verified"). Accepted as legitimate for this target.
- **Tracking requirement (not blocking this turn):** update `tcb-allowed.md` so its
  "Intra-crate (`kernel` crate `hal::mem::*`)" bullet points to the new home
  (`page.spec.rs`) rather than `frame.spec.rs`, keeping the audit trail accurate. The fixer's own
  finding that `Address::into_raw_value` carries a trait-level `#[verus_spec]` should be
  re-tested: if the trait contract actually applies to the `PageAligned` impl, remove the
  `into_raw_value` assume entirely instead of relocating it.

---

### Fix Request (this turn)

#### FIX A — make the 3 transition lemmas verify (clears 5 errors)
In `src/kernel/src/mm/phys/frame.proof.rs`, add the missing length bound to each transition lemma
and thread it from the call sites:
1. `lemma_reserve_one_v` (line 270): add `nb <= rc.len()` to `requires`.
2. `lemma_release_one_v` (line 358): add `nb <= rc.len()` to `requires`.
3. `lemma_update_refcount_v` (line 443): add `nb <= rc.len()` to `requires`.

With `0 <= fnx < nb <= rc.len()` you get `fnx < rc.len()` (so `rc.update(fnx,v)[fnx]==v`), and for
any `i` drawn from the in-range set `0 <= i < nb <= rc.len()` (so `rc.update(fnx,_)[i]==rc[i]` for
`i!=fnx`). At each call site in `Inner::free`/`Inner::share`/`Inner::book` (and reserve users),
supply `nb <= rc.len()` from `self.internal_inv()` (which gives
`self.refcount@.len() >= self.bitmap@.num_bits`). If the trigger warning at frame.proof.rs:406/291
persists after the bound is added, annotate the `forall` with an explicit trigger.

Verify: `make verify-kernel` → the 5 errors at frame.proof.rs:341/344/430/478/481 and frame.rs:270/359
are gone.

#### FIX B — remove the 3 admits (Zero admit)
Replace `proof! { admit(); }` in `Inner::alloc`(137), `Inner::alloc_contiguous`(214),
`Inner::alloc_range`(679) with real proofs discharging their existing strong specs. `alloc` and
`alloc_range` can reuse `lemma_reserve_one_v` (once FIX A lands) for the per-frame
free→allocated+refcount=1 transition; `alloc_contiguous` similarly over its contiguous run.
Verify: `grep -n admit src/kernel/src/mm/phys/frame.rs` → empty, AND
`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` lists **no** `mm/phys/frame.rs:* admit`.

#### After A–B
Re-run `make verify-kernel`: must report **0 errors, 0 warnings** kernel-wide with `admit=0`. Then
I will re-check spec-drift (no weakening of the locked `Inner::*` specs), the `// VERUS BUG FIX`
exec-rewrite equivalence (frame.rs:445-447 / 551-553), and cross-module regression.

---

### Verdict: **NOT RESOLVED.** 3 admits + 5 verification errors remain. No STOP file created.
Real progress this turn (refcount, book verify; frame.spec.rs assume removed), but the core proof
obligations for alloc/alloc_contiguous/alloc_range/free/share are still open.
