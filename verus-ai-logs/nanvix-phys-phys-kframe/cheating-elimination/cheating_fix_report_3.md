# Cheating-Fix Pass 3: response to `cheating_report_3.md`

`cheating_report_3.md` is **byte-identical** to reports 1 and 2 (`diff` ⇒ no differences): the
same 15 `external_body` + 4 `admit`, all in unlisted sibling modules (`frame`, `manager`, `mod`,
`upool`), **none in `kframe.rs` / `kframe.spec.rs` / `kframe.proof.rs`**. This pass did not
re-state the prior conclusion — it made a fresh, destructive attempt to *prove* the 4 admits and
captured the exact Verus output.

## Direct response to each demand in the driver message

- **"`admit()`/`assume()` must be replaced with real proofs."** Attempted (see live run below).
  The 4 lemmas are **not theorems**: each takes the post-state (`post` / `m@`) as a *free
  parameter* and asserts it equals a value derived from `pre`. Universally quantified, this is
  false, so a sound proof is impossible. There are **0 `assume()`** in scope.
- **"`trusted` and `external_body` on proof fns must be removed."** There are **0 `trusted`** and
  **0 `external_body` on proof fns** in `mm::phys` (`grep` ⇒ none). All 15 `external_body` are on
  **exec** fns / a **type-spec**, and all 15 are ledgered in `tcb-allowed.md` (item 15,
  `ExLinkedList`, was added in pass 2).
- **"Multi-line `limitation_assume` bodies must be reduced (R20c)."** There are **0**
  `limitation_assume` / `VERUS-AI LIMITATION` annotations in `mm::phys` — rule does not apply.
- **"`#[verifier::exec_allows_no_decreases_clause]` (R20p) must be removed."** There are **0** of
  these in `mm::phys` (`no_decreases=0` in every scan) — rule does not apply.

## Live destructive attempt this pass (the real proof attempt)

Removed **all four** `admit()`s from `manager.proof.rs` (lines 16/35/55/159) and ran
`make verify-kernel MODULE=mm::phys::manager`:

```
error: postcondition not satisfied   (×4 — one per lemma)
  lemma_manager_attached        — ensures m@ == phys_view().frames
  lemma_kernel_alloc_one        — ensures post == pre.alloc_one(addr)
  lemma_kernel_alloc_contiguous — ensures post == pre.book_all(kernel_addr_set(frames))
  lemma_user_bulk_err_restored  — ensures m@ == pre
verification results:: 14 verified, 4 errors (exit 101)
  cheating scan: assume=0 external_body=15 admit=3 trusted=0 no_decreases=0
```

This is the proof: with `post`/`m` universally quantified, `post == pre.alloc_one(addr)` (etc.)
is **mathematically false** — pick any `post ≠ pre.alloc_one(addr)`. The lemmas exist only to
inject the §8 global-token attachment as an assumption, exactly as their doc-comments declare
("supplied here and discharged by the global-token attachment **in the proving phase**"). The
structural gap: the backing `frame::alloc` / `frame::free` free-functions are ledgered
`external_body` with **no view-postcondition**, so no fact exists at any call site to chain into a
real proof. The file was restored; the module re-verifies `18 verified, 0 errors`.

## Why no skill-compliant elimination exists (re-confirmed)

| Avenue | Verdict |
|---|---|
| Prove the lemmas as written | Impossible — false for arbitrary `post`/`m` (shown above). |
| Add real preconditions tying `post` to `pre` | None available: `frame::alloc/free` are `external_body` with no view-effect spec; the global mutation is unmodeled. |
| Restructure lemmas to *compute* `post` | Doesn't close the gap — caller still needs `final(self)@ == computed`, which only the §8 token supplies; also ripples to unlisted exec callers. |
| Self-author an `axiom` / `assume_specification` for the attachment | **Forbidden** (verus-constraints: human-approval-only; "report it, do not claim it as Verus's limitation"). |
| Relocate to an `external_body` exec wrapper | **Unsound** — would trust a provably-false / self-contradictory contract (`lemma_user_bulk_err_restored` contradicts a *proven* loop invariant), letting the module derive `false`. |
| Weaken the unlisted manager external-top specs to match the impl | **Forbidden** — unlisted functions + stealth spec-drift; and no locally-correct spec exists (`== old` falsely claims the watermark free-count is unchanged after a kernel alloc). |
| Thread the §8 `tracked` ghost token | **Forbidden** — source-integrity bans exec signature/struct changes. |

## Verification (this pass)

```
make verify-kernel MODULE=mm::phys::kframe   → 3 verified, 0 errors, status: CLEAN
                                               (assume=0 external_body=0 admit=0 trusted=0
                                                no_decreases=0 cfg_gate=0 in-module)
make verify-kernel MODULE=mm::phys::manager  → 18 verified, 0 errors (after restore)
make verify-kernel (full crate)              → exit 0; status: CHEATING_DETECTED
                                               (global admit=7 external_body=15 — all out-of-scope)
```
Source working tree restored to the verifying baseline (the destructive experiment was
auto-committed by `verify.sh` then reverted; net source diff = 0). The `kframe` module remains
CLEAN with zero cheating of any kind.

## Result: BLOCKER (crate-global gate); phys-kframe scope CLEAN — unchanged, now with proof

The repeated identical re-flag is the automated crate-global cheating scanner counting the 4
`admit()` in `manager.proof.rs` and 15 ledgered `external_body`. I have now **demonstrated by
live Verus run** that the 4 admits are non-theorems and cannot be discharged by any
skill-compliant proof. Per the original task: *"any remaining `admit()` still trips the cheating
gate and the phase FAILs … an honest hand-off of genuinely-stuck proofs, not a way out."* This is
that hand-off; documented in `nanvix-phys-phys-frame/verification_todo.md` (pass-3 reproduction)
and `nanvix-phys-phys-manager/bugs.md` OBS-4.

**Human decision required (one of):**
1. Approve a single `axiom`/`assume_specification` realizing `self@ == phys_view().frames` (the §8
   attachment), added to `manager.spec.rs` and ledgered in `tcb-allowed.md`; or
2. Authorize the proving-phase §8 `tracked`-token infrastructure threaded through the `frame`
   free-function layer + `PhysMemoryManager`/`Upool`, accepting the exec-signature changes
   source-integrity otherwise forbids.

No unsound work-around was applied; no exec signatures or unlisted specs were changed.
