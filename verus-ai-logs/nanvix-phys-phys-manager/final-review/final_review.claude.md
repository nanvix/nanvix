# Final Independent Review — `mm::phys::manager` (`PhysMemoryManager`)

**Reviewer:** Independent strict final review (Claude)
**Date:** 2026-06-15
**Branch:** `verus-ai/phys-manager`  •  **HEAD:** `759a6857f`  •  **Pre-verus base:** `2cccace54` (`caller-analysis START`)
**Module:** `kernel::mm::phys` (target file `src/kernel/src/mm/phys/manager.rs`)
**In-scope functions:** `init`, `alloc_user_frame`, `check_user_watermark`, `alloc_many_user_frames`, `alloc_many_kernel_frames`, `alloc_kernel_frame`

> **Headline:** No *hard* blockers — verification passes (exit 0, 0 errors), `admit=0`, `assume=0`,
> all 6 `external_body` shims are pre-approved in `tcb-allowed.md`, AST is byte-for-byte consistent,
> and no spec was weakened. **However**, the realized contracts have several genuine
> **spec-completeness gaps** that fail the strict quality bar: tautological `Err(_) => true` arms,
> an effectively vacuous `init` contract, a missing **no-double-allocation/distinctness** guarantee on
> the user bulk path (the spec admits a buggy implementation), missing fresh-ownership (refcount=1)
> facts, and a `uninterp` watermark spec fn that `verus-constraints` bans. **Final Result: FAIL on
> quality grounds.**

---

## 0. Evidence Index (commands run)

| Check | Command | Result |
|---|---|---|
| Verification | `make verify-kernel MODULE=mm::phys` | exit 0, 0 errors (cached) |
| AST consistency | `ast_consistency.py --base-ref 2cccace54 manager.rs count/summary` | ✅ 7 fns + 1 struct MATCH |
| Fn coverage | `fn_coverage.py manager.orig.rs manager.rs` | 7/7 matched, 0 missing, 0 extra |
| Spec drift (instructed) | `spec_drift.py git-diff manager.rs --before HEAD` | exit 0 — no drift |
| Spec drift (vs base) | `spec_drift.py git-diff manager.rs --before 2cccace54` | exit 1 — only *additions* (new contracts), 0 ensures removed |
| `VERUS REWRITE` scan | `grep -rn "VERUS REWRITE\|DEVIATION\|BUG FIX"` | none |
| Cheating grep | per-file `grep` of admit/assume/external_body/cfg | see §7 |

---

## 1. Spec Quality (external-top API contracts)

The six methods are `#[verus_verify(external_body)]` shims with `#[verus_spec]` contracts stated over
the do-not-modify `phys_view()` / `FrameAllocView`. Because the carrier is the **global** `phys_view()`
with **no `old(phys_view())` handle** (per `view_design.md`), contracts are *monotone post-state facts*.
This is an acknowledged, pre-approved architectural choice — but it does not exempt the contracts from
the spec-design quality bar, and several clauses fall short.

### 1.1 Tautological error postconditions — `Err(_) => true` (anti-pattern #5/#8)

Three functions specify nothing on the error path:

- `alloc_user_frame` — `manager.rs:264` → `Err(_) => true`
- `check_user_watermark` — `manager.rs:304` → `Err(_) => true`
- `alloc_kernel_frame` — `manager.rs:349` → `Err(_) => true`

`caller_analysis.md` (L73, L101–102, L127) states callers rely on the error path meaning **"nothing was
allocated; no frame leaks; allocator left untouched."** None of that is captured. The unconditional
`phys_view().inv()` + `phys_view().initialized` ensures (e.g. `manager.rs:255–256, 298–300, 342–343`)
*partially* mitigate (the allocator stays well-formed on error), but "no leak / state-preserved" is **not
formally guaranteed**. This is precisely spec-design Anti-Pattern #5 (One-Sided Error Spec) and #8
(Tautological Postcondition).

*Mitigation note:* with no `old(phys_view())`, state-preservation on a value-less `Err` is genuinely
inexpressible for the **single-frame** paths — `Err(_) => true` is the maximum achievable there. This is
a limitation of the pre-approved carrier, not laziness. It is still a real completeness gap that the
caller-required guarantee is not met.

### 1.2 `init` contract is effectively vacuous (missing the property the caller needs)

`init` (`manager.rs:99–106`): `requires phys_view().initialized, phys_view().inv()` and
`ensures phys_view().inv(), phys_view().initialized`. Since the postcondition merely **restates the
precondition**, the contract is a frame-preservation no-op. The caller-relevant effect that
`caller_analysis.md` (L53–61) names — *"after Ok, the singleton is established and every later
`get_mut()`/`alloc_*` is valid; one-shot/monotonic; second call errors"* — is **not captured**: the
manager-singleton `AtomicBool` lifecycle has no abstract model (`manager.rs:91–96`). The `Err`
(double-init) condition is entirely unspecified (the ensures are unconditional, no `match`). A caller
cannot prove from this spec that `get_mut()` will not panic after `init`. **Weak / incomplete.**

The precondition `phys_view().initialized` is *correct and satisfiable* (boot calls `frame::init` →
region booking → `Upool::new` **before** `PhysMemoryManager::init`, per `caller_analysis.md` L14–17), but
it means `init` proves nothing new.

### 1.3 Missing "no-double-allocation"/distinctness on the user bulk path (admits a buggy impl)

`alloc_many_user_frames` Ok arm (`manager.rs:187–194`):
```
final(frames)@.len() == count
forall|i| 0 <= i < count ==> allocated_frames.contains(final(frames)@[i]@)
free_frames.finite()
spec_watermark_ok(frames, 0)
```
`allocated_frames` is a `Set<int>`. **Nothing forbids the `count` returned frames from sharing the same
address.** A buggy implementation that pushes the *same* frame `count` times satisfies every clause
(`len==count`, the single address is `contains`-ed, watermark holds). Per spec-design quality test #3
("if you can imagine a buggy implementation that still satisfies the spec, the spec is incomplete"),
this contract is **insufficient to reject bugs**. `caller_analysis.md` L91 and the Key-Invariants
("no double-allocation", L146–147) explicitly require distinctness/exclusivity.

Contrast `alloc_many_kernel_frames`, whose `kernel_frames_contiguous(frames, base)`
(`manager.rs:403–404`) **does** imply pairwise-distinct addresses (proved by
`lemma_contiguous_run_distinct`, `manager.proof.rs:32–58`). The user bulk path has **no** such
distinctness clause — an asymmetric gap.

### 1.4 Missing freshness / exclusive-ownership (refcount = 1)

`view_design.md` (L133–148, §"Design Rationale") and `caller_analysis.md` (refcount rationale) intend
"freshly allocated ⇒ refcount 1" for user CoW. The realized success arms only assert
`allocated_frames.contains(frame@)` — never `refcounts[frame@] == 1`, and never that the frame was
**previously free** (`free_frames.contains(addr)` in `old`). Via `wf()` an allocated frame has *some*
refcount in `1..=255`, but not specifically `1`, and not "fresh". The exclusive-ownership guarantee the
CoW caller relies on (`vmem.rs:879`) is therefore weaker than designed. **Incomplete.**

### 1.5 What is done well

- **Watermark policy split is correct and surfaced.** User success arms ensure
  `spec_watermark_ok(phys_view().frames, 0)` (`manager.rs:193, 262`) — i.e. ≥ `KERNEL_WATERMARK` frames
  remain free *after* servicing. Kernel paths (`alloc_kernel_frame`, `alloc_many_kernel_frames`)
  correctly omit any watermark clause (`manager.rs:344–348, 397–405`). Matches `caller_analysis.md`
  L151–152.
- **Page alignment** asserted on single-frame allocs (`frame@ % spec_page_size() == 0`,
  `manager.rs:260, 347`).
- **Contiguity** for kernel stacks via `exists|base| kernel_frames_contiguous(...)`
  (`manager.rs:403–404`) — load-bearing and well-modeled, backed by a real distinctness lemma.
- **All-or-nothing vector rollback** captured on both bulk error arms via `final(frames)@.len() == 0`
  (`manager.rs:195, 406`).
- `match`-on-result form (not separate `is_ok()/is_err()`) is used throughout — complete by
  construction for the arms present.

**§1 verdict:** Contracts are well-structured and the success-path watermark/contiguity/alignment facts
are strong, but error-path rigor and the core ownership guarantees (distinctness, freshness, init
lifecycle) are missing. **Quality criterion NOT fully met.**

---

## 2. Caller Coverage (`caller_analysis.md`)

6 in-scope functions, success **and** failure expectations each. "Covered" = a `requires`/`ensures`
formally provides the caller-relied guarantee.

| # | Expectation (from caller_analysis) | Status | Evidence |
|---|---|---|---|
| init — Ok (singleton live, get_mut won't panic) | ❌ Missing | lifecycle unmodeled, `manager.rs:103–105` only restates precond |
| init — Err (only when already init) | ❌ Missing | no `match`/Err clause |
| alloc_user_frame — Ok (owned, watermarked, distinct) | ⚠ Partial | allocated+aligned+watermark ✅; exclusivity/refcount ❌ (`:258–263`) |
| alloc_user_frame — Err (no alloc, no leak) | ❌ Missing | `Err(_) => true` (`:264`) |
| alloc_many_user_frames — preconds (empty Vec) | ✅ Covered | `requires old(frames)@.len()==0` (`:182`); capacity via runtime Err |
| alloc_many_user_frames — Ok (exactly count, owned) | ⚠ Partial | len+allocated+watermark ✅; **distinctness ❌** (`:187–193`) |
| alloc_many_user_frames — Err (emptied, freed, no leak) | ⚠ Partial | vector emptied ✅ (`:195`); no-leak/state ❌ |
| alloc_kernel_frame — Ok (owned, watermark bypass) | ✅ Covered | allocated+aligned, no watermark (`:345–348`) (minus refcount) |
| alloc_kernel_frame — Err (no leak, raw freed) | ❌ Missing | `Err(_) => true` (`:349`) |
| alloc_many_kernel_frames — preconds (empty Vec) | ✅ Covered | `requires old(frames)@.len()==0` (`:393`) |
| alloc_many_kernel_frames — Ok (count, contiguous) | ✅ Covered | len+allocated+contiguity (`:398–405`) |
| alloc_many_kernel_frames — Err (cleared, all freed) | ⚠ Partial | vector emptied ✅ (`:406`); no-leak ❌ |
| check_user_watermark — Ok iff free≥WM+count | ✅ Covered (one-way) | `Ok(()) => spec_watermark_ok(.., count)` (`:302`); no liveness |
| check_user_watermark — Err (OutOfMemory on breach) | ❌ Missing | `Err(_) => true` (`:304`) |

**Coverage: 6/14 fully covered, 4 partial, 4 missing.** Success-path coverage is reasonable (5 of 6
success arms carry meaningful guarantees, user-bulk missing distinctness). **Failure-path coverage is
weak** — 4 of the listed failure expectations resolve to `Err(_) => true`/unmodeled, and the two bulk
error arms capture only vector-emptiness, not "no leak / allocator untouched."

---

## 3. Proof Completeness

- **`admit()` count: 0** across `manager.rs`, `manager.spec.rs`, `manager.proof.rs`
  (the only "admit" hit is the prose in `manager.proof.rs:8` — a comment). ✅ No BLOCKER.
- **`external_body` count (manager.rs): 6** — all six target methods (lines 98, 177, 249, 292, 336, 388).
  Each is listed in `tcb-allowed.md` (see §4). No `external_body` in `.spec.rs`/`.proof.rs`.
- The two abstract laws in `manager.proof.rs` are **fully discharged** (no `admit`/`assume`):
  - `lemma_watermark_monotone` (`:17–24`) — trivial arithmetic, correct.
  - `lemma_contiguous_run_distinct` (`:32–58`) — sound multiplication argument via
    `vstd::arithmetic::mul` lemmas; correctly proves distinct indices ⇒ distinct addresses.

**§3 verdict:** ✅ No admit, proofs complete. (Caveat §1.5/§5: `lemma_watermark_monotone`'s
`ensures spec_watermark_ok(v,1)` is never invoked from any in-scope exec contract — the single-frame
alloc specs assert `spec_watermark_ok(frames, 0)`, not a 1-vs-count relation — so the lemma is currently
a **floating/unreferenced** abstract law, see §5.)

---

## 4. TCB Compliance

All `external_body` on `manager.rs` own functions must already be in `tcb-allowed.md`. Verified:

| Function | manager.rs line | In tcb-allowed.md? |
|---|---|---|
| `PhysMemoryManager::init` | 107 | ✅ (L54) |
| `PhysMemoryManager::alloc_user_frame` | 267 | ✅ (L57) |
| `PhysMemoryManager::check_user_watermark` | 306 | ✅ (L61) |
| `PhysMemoryManager::alloc_many_user_frames` | 198 | ✅ (L65) |
| `PhysMemoryManager::alloc_kernel_frame` | 352 | ✅ (L69) |
| `PhysMemoryManager::alloc_many_kernel_frames` | 409 | ✅ (L71) |

**No new/unapproved trust boundary introduced.** `get_mut` (uses `assume_init_mut`) is explicitly on the
TCB **Skip** list (`tcb-allowed.md:37`) and is out of scope. ✅ **No BLOCKER.**

---

## 5. AST Consistency

```
ast_consistency.py --base-ref 2cccace54 manager.rs count   → ✅ Consistent: 7 functions, 1 structs match.
ast_consistency.py --base-ref 2cccace54 manager.rs summary → matched=7 mismatched=0 missing=0 extra=0
```
Every exec function (`init`, `alloc_user_frame`, `check_user_watermark`, `alloc_many_user_frames`,
`alloc_many_kernel_frames`, `alloc_kernel_frame`, `get_mut`) and the `PhysMemoryManager` struct hash
**MATCH** the pre-verus baseline. No `// VERUS REWRITE` / `VERUS DEVIATION` comments exist (grep: none).
Exec code was not mutated — only attributes/specs added.

**Result: PASS.**

(Floating-law note: `lemma_watermark_monotone` is defined but no in-scope exec `requires`/`ensures`
references it; the realized single-frame specs use `spec_watermark_ok(.., 0)` on the post-state rather
than relating `count` to `1`. It is a harmless but currently-orphan internal law — spec-design §"No
floating specs". `lemma_contiguous_run_distinct` is likewise not invoked from an exec contract, though it
backs the *meaning* of the contiguity clause. Neither affects soundness; both are noted as minor
hygiene issues.)

---

## 6. Verification

```
make verify-kernel MODULE=mm::phys
  Exit code : 0
  Results   : cached (no recompilation)
  Global    : assume=0 external_body=22 admit=0 trusted=0 no_decreases=0 cfg_gate=9
  status    : CHEATING_DETECTED   (← solely due to TCB-approved external_body shims)
```
Verus ran and reported **0 verification errors**. The `CHEATING_DETECTED` banner is the tooling flagging
the 22 module-wide `external_body` (12 in `frame.rs`, 6 in `manager.rs`, 2 in `mod.rs`, plus
`upool.rs::new`) — **all** of which are enumerated in `tcb-allowed.md`. No concurrent-build error
occurred; the cached result is authoritative and matches the canonical HEAD commit message
("verify PASS … external_body=22 cfg_gate=9").

**Result: PASS, 0 errors.**

---

## 7. Guardrails Compliance — Exact Cheating Counts

Counts are for the three manager files only (`manager.rs`, `manager.spec.rs`, `manager.proof.rs`):

| Dimension | Count | Locations | Blocker? |
|---|---:|---|---|
| `admit()` | **0** | — (only prose mention at `manager.proof.rs:8`) | No |
| `assume(...)` (verification) | **0** | `manager.rs:147` is `MaybeUninit::assume_init_mut()` — a std exec call, **not** `assume()`; `manager.proof.rs:8` is a comment | No |
| `external_body` | **6** | `manager.rs:98, 177, 249, 292, 336, 388` | No — all in `tcb-allowed.md` |
| `assume_specification` | **0** | — | No |
| cfg-gated **exec** code | **0** | `manager.rs:9,11` gate `include!` of spec/proof files; `manager.rs:97,291` are `cfg_attr(..., allow(lint))` — both non-semantic, permitted | No |
| `uninterp spec fn` | **1** | `manager.spec.rs:35` `spec_kernel_watermark()` | ⚠ see below |

**`admit=0`, `assume=0`** → no guardrail BLOCKER on those axes. **All 6 `external_body` are
pre-approved** → no TCB BLOCKER. **No exec cfg-gating.**

**⚠ `uninterp spec fn spec_kernel_watermark()` (`manager.spec.rs:35`)** — `verus-constraints.md` lists
`uninterp spec fn` as **Banned** ("all spec functions must have concrete definitions … same effect as
`assume` when paired with `external_body` proof axioms"). This is **not** in the task's explicit
hard-blocker list (admit/assume/external_body), and three factors soften it: (a) **no axiom injects any
property** about it — it is a pure opaque constant, so it cannot smuggle in unsound facts; (b) it mirrors
the accepted do-not-modify `phys_view()` `uninterp` pattern (`mod.spec.rs:171`); (c) it genuinely models
an **external build-time constant** (`config::kernel::KERNEL_WATERMARK` from another crate). Still, per a
strict reading it could/should be given a concrete definition (e.g. `config::kernel::KERNEL_WATERMARK as
int`) — leaving it `uninterp` means the tie between the spec and the real constant is *trusted via the
`external_body` shims* rather than expressed. **Flagged as a quality/guardrail concern (Medium), not a
hard blocker.**

---

## 8. Bug Reconciliation (`bugs.md`)

`bugs.md` records **"None."** Reconciliation against final code:

- **No TRUE code bug found** in this review. The exec logic is defensive and correct:
  - watermark overflow guarded by `KERNEL_WATERMARK.checked_add(count)` (`manager.rs:307–313`);
  - user bulk all-or-nothing: `frames.clear()` on mid-loop failure (`manager.rs:225`);
  - kernel bulk two-phase cleanup frees both wrapped (`frames.clear()`) and un-wrapped
    (`frame::free` loop) frames (`manager.rs:433–443`) — leak-free on error;
  - `alloc_kernel_frame` frees the raw frame if `KernelFrame::new` fails (`manager.rs:354–359`).
- The items I raise in §1 are **spec-completeness gaps, not code defects** — per `bug-reporting.md` they
  are correctly *not* classified as bugs. `bugs.md` "None" is **accurate**.
- **No new bugs** discovered. `bugs.md`'s "Notes" section attributes the `external_body` to "genuine
  Verus front-end limitations" (`static mut`, `error!/warn!` macros) — consistent with `tcb-allowed.md`;
  acceptable.

**§8 verdict:** ✅ `bugs.md` reconciles correctly; no unrecorded bugs.

---

## Prioritized Issues

**Blockers (hard):** none.

**High (quality — spec admits buggy behavior / core guarantee absent):**
- **H1.** `alloc_many_user_frames` Ok arm lacks any **distinctness / no-double-allocation** clause
  (`manager.rs:187–193`); the spec is satisfied by an impl returning `count` aliases. Add a pairwise-
  distinct-addresses clause (the kernel path gets this for free via contiguity). — §1.3
- **H2.** Error paths `Err(_) => true` on `alloc_user_frame` (`:264`), `alloc_kernel_frame` (`:349`),
  `check_user_watermark` (`:304`) guarantee nothing; the caller-relied "no leak / state untouched" is
  absent. (Partly carrier-limited; see §1.1.) — §1.1, §2

**Medium (quality):**
- **M1.** `init` contract is a precondition-restating no-op; does not capture the lifecycle
  establishment the caller depends on, and leaves `Err` unspecified. — §1.2
- **M2.** Missing fresh-ownership facts (`refcounts[addr]==1`, `addr ∈ old free_frames`) on all alloc
  success arms — weaker than the CoW caller needs. — §1.4
- **M3.** `spec_kernel_watermark()` is `uninterp` (`manager.spec.rs:35`) — `verus-constraints` bans it;
  prefer a concrete definition. — §7
- **M4.** Bulk error arms (`:195, :406`) capture only `len==0`, not "no frame leaked." — §2

**Low (hygiene):**
- **L1.** `lemma_watermark_monotone` (and arguably `lemma_contiguous_run_distinct`) are not referenced by
  any in-scope exec `requires`/`ensures` — floating/orphan abstract laws. — §3, §5

---

## Summary Scorecard

| Item | Result |
|---|---|
| Cheating counts (manager.*): admit / assume / external_body / assume_specification / cfg-gated exec | **0 / 0 / 6(all TCB-approved) / 0 / 0** |
| `uninterp` spec fn | 1 (`spec_kernel_watermark`) — flagged |
| AST consistency | **PASS** (7 fns + 1 struct MATCH) |
| Fn coverage | 7/7 matched, 0 missing/extra |
| Verification | **PASS** — exit 0, **0 errors** |
| Spec drift (`--before HEAD`, instructed) | **No drift** (exit 0); vs base = additions only, **0 ensures removed** (no weakening) |
| Caller coverage | **6/14 full, 4 partial, 4 missing** (success-path strong, failure-path weak) |
| Bugs | None (bugs.md accurate; no new bugs) |
| Hard blockers | **0** |
| Quality criteria fully met | **No** (H1, H2, M1–M4) |

---

## Final Result: **FAIL**

**Rationale.** There are **no hard blockers** — verification passes with zero errors, there are zero
`admit()`/`assume()`, every `external_body` is pre-approved in `tcb-allowed.md`, AST is byte-identical to
the baseline, and no existing spec was weakened. The proof artifacts are clean and the success-path
watermark/contiguity/alignment guarantees are genuinely strong.

The effort nonetheless **fails the strict quality bar**: the realized contracts do not satisfy
spec-design's core test of being *"sufficient to reject bugs."* Most critically, `alloc_many_user_frames`
admits an implementation that hands out `count` aliases of the same physical frame (H1), the three
single-result error arms are tautological `Err(_) => true` (H2), `init` proves nothing beyond its own
precondition (M1), and the fresh-ownership/refcount and "no-leak" guarantees the callers rely on are
absent (M2, M4). Several of these (single-frame error state-preservation) are inherent to the
pre-approved no-`old(phys_view())` carrier and are reasonable concessions; but H1, M1, and the
distinctness/refcount facts are expressible and should be added before this can pass a strict review.

Per the instruction *"PASS only if there are zero blockers **AND** every quality criterion is met,"* the
unmet quality criteria force a **FAIL** verdict, with the explicit note that all mechanical
verification/cheating/TCB/AST gates are **green**.
