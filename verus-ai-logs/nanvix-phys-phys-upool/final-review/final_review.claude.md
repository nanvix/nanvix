# Final Verification Review — `mm::phys::upool`

> Independent, STRICT final review of the `phys-upool` module.
> Reviewer: Copilot CLI (claude). Branch: `verus-ai-prove-bottom-up`.
> Date: 2026-06-15. No source/spec/proof code was modified during this review.

In-scope functions (8): `UserFrame::share`, `UserFrame::refcount`, `Upool::new`,
`UserFrame::leak`, `UserFrame::drop`, `Upool::alloc`, `UserFrame::new`,
`UserFrame::address`.

Files read in full: `upool.rs`, `upool.spec.rs`, `upool.proof.rs`,
`caller_analysis.md`, `view_design.md`, `bugs.md`, `tcb-allowed.md`, and all
six referenced skills.

---

## Checklist

- [x] 1. Spec quality reviewed (with one substantive completeness concern — see Issues)
- [ ] 2. Caller coverage complete — **PARTIAL**: refcount-discipline of `share`/`drop`/`leak` not realized as ensures (documented §8 deferral)
- [x] 3. Proof completeness — 0 `admit()` in upool, only TCB-approved `external_body`
- [x] 4. TCB compliance — both upool `external_body` are listed in `tcb-allowed.md`
- [x] 5. AST consistency — 1 reported MISMATCH, but it is a pre-approved cfg-gate of a logging macro (no semantic change, no `// VERUS REWRITE`)
- [x] 6. Verification — `make verify-kernel MODULE=mm::phys` exits 0 (PASS), 0 errors
- [x] 7. Guardrails — admit=0, assume=0 in upool; 2 `external_body` (both TCB); cfg-gate only on a logging macro
- [x] 8. Bug reconciliation — `bugs.md` "no bugs"; no new code bugs; the deferral is correctly classified
- [ ] Spec sufficiency ("sufficient to reject bugs") for `share`/`drop`/`leak` refcount effects — **NOT met** (deferred)

Hard blockers (admit>0, assume>0, unauthorized `external_body`, semantic AST
mismatch): **NONE**. The single failing item is completeness of the
refcount-discipline contracts (caller coverage / spec sufficiency), a documented
and technically-forced deferral rather than a soundness or cheating defect.

---

## Spec Quality

Per `spec-design`, evaluating the 8 contracts:

**Strong / correct:**
- `UserFrame::new` (upool.rs:83-92) — `requires addr.inv()`; `ensures result@ == addr@`, `result.inv()`. Faithful thin-wrapper round-trip; infallible (returns `Self`). Matches caller round-trip need.
- `UserFrame::address` (upool.rs:103-112) — `ensures result@ == self@`, `result.inv()`. Correct pure accessor; basic-ensures level is appropriate (trivial getter per spec-design §"Basic ensures only").
- `UserFrame::refcount` (upool.rs:182-196) — `match` over Ok/Err; Ok: `allocated_frames.contains(self@)` ∧ `count as int == refcounts[self@]`; Err: `!allocated_frames.contains(self@)`. Both arms meaningful, error path bidirectional. Directly usable by the CoW `== 1` probe. Good.
- `Upool::new` (upool.rs:250-257) — `ensures result@.wf()`. Exactly the one fact the boot caller needs. (`external_body`, TCB.)
- `Upool::alloc` (upool.rs:271-291) — full `match`: Ok `free_frames.contains(uf@)` ∧ `final@ == old@.alloc_one(uf@)`; Err `final@ == old@` ∧ `old@.free_count() == 0`; `wf()` preserved. Complete by construction (uses `alloc_one`/struct-update vocabulary), error-path state-preservation + exhaustion present. Strong. (`external_body`, TCB.)

**Mathematical types / view consistency:** Views use `int` (`UserFrame@`) and
`FrameAllocView` (`Upool@`) — abstract, caller-facing, survives the substitution
test (view_design §5). Error paths use `match` (not split `is_ok()==>`),
satisfying spec-design anti-patterns #7. No tautological clauses in the
expressed facts.

**Trait obligations:**
- `View for UserFrame` (`= int`, `closed`) — correct, caller-abstract.
- `View for Upool` (`= FrameAllocView`, **`uninterp`**) — a mechanical consequence of the two `external_body` facade methods (`new`/`alloc`) that own the trust; documented in tcb-allowed.md:101-117 and view_design §2.2/§7.3. Not flagged as a separate cheating dimension by the detector. Acceptable under the TCB regime but worth noting it is the one `uninterp spec fn` in upool.
- `Drop for UserFrame` (upool.rs:200-211) — `opens_invariants none`, `no_unwind`. **Carries no functional postcondition** — see the substantive concern below.

**Substantive concern (completeness, not soundness)** — the refcount-affecting
trio is under-specified relative to its own committed design target
(view_design §4.4/§4.6/§4.3) and to caller_analysis:

- `UserFrame::share` (upool.rs:151-170): Ok arm states `uf@ == self@`, `uf.inv()`, `allocated_frames.contains(self@)`; Err arm states the failure cause (`!contains || refcounts >= 255`). It does **not** state the `+1` increment (`F' == F.add_ref(self@)`), nor the `refcounts[self@] < 255` headroom, nor the Err-arm frame condition (`self`+frame unchanged). Per caller_analysis: *"Would break callers: succeeding without actually incrementing the refcount (would cause premature free)."* The current spec would be satisfied by a buggy `share` that returns Ok without incrementing — i.e. it is **not "sufficient to reject bugs"** (spec-design §1.3) for the very property the caller flags as critical.
- `UserFrame::drop`: **no functional ensures at all**. The defining RAII service (`F' == F.release(self@)`, view_design §4.6 "the committed design semantics") is absent. A drop that leaks or double-frees satisfies the empty contract.
- `UserFrame::leak` (upool.rs:123-133): only `result@ == self@`, `result.inv()`. The defining *no-release* guarantee (view_design §4.3: *"after leak, no decrement … a regression that freed on leak would double-free"*) is absent. A leak that erroneously freed still satisfies `result@ == self@`.

These three gaps are **documented and intentionally deferred** (bugs.md "Notes",
view_design §8, tcb-allowed.md:118-124). Root technical cause: `phys_view()` is a
**0-argument `uninterp` constant**, so a before/after transition
(`old(phys_view())` vs `phys_view()`) cannot be expressed — both sides are the
same logic constant, making any such clause tautological. The genuine transition
must be threaded through the §8 ghost token in the **frame free-function layer**
(`frame::share`/`frame::free`, currently `external_body` / `ensures true`
best-effort), which is not yet verified. This is a real, forced limitation — not
proof laziness — but it does mean the module's central safety contract (the
reference-count discipline) is **not yet realized as ensures**.

---

## Caller Coverage (Covered 5/8 fully; 3/8 partial)

Per-function reconciliation against `caller_analysis.md`:

| Function | Caller's essential expectation | In contract? |
|---|---|---|
| `UserFrame::new` | view == addr@; infallible; no alloc/refcount change | **Covered** (round-trip present; infallible by type). "No global effect" omitted (deferred, would be tautological). |
| `UserFrame::address` | result@ == self@; pure read | **Covered** fully. |
| `UserFrame::refcount` | Ok: count == refcounts[self@] for owned frame; Err: not allocated; no mutate | **Covered** fully (both arms). |
| `Upool::new` | `@.wf()` holds | **Covered** fully. |
| `Upool::alloc` | Ok: `free_frames.contains(uf@)` ∧ `alloc_one`; Err: unchanged ∧ `free_count()==0`; `wf()` | **Covered** fully (both arms). |
| `UserFrame::share` | Ok: **refcount incremented** ∧ same frame; Err: self+frame unchanged | **Partial** — alias (`uf@==self@`) + `contains` + failure-cause present; **`+1` increment and Err frame condition deferred**. |
| `UserFrame::leak` | **suppress Drop / no release** (no refcount decrement) | **Partial** — only `result@==self@`; the **no-release guarantee is absent** (deferred). |
| `UserFrame::drop` | **release exactly one reference** (RAII cleanup) | **Partial** — **no functional ensures** (`release` deferred). |

**Covered: 5/8 fully, 3/8 partial.** The three partials are exactly the
refcount-affecting methods (`share`, `drop`, `leak`) that caller_analysis
(lines 63-74, 52-61, 17-24, 110-118) identifies as the module's core
safety-critical surface.

**Missing important properties (all the documented §8 deferral):**
1. `share`: `F' == F.add_ref(self@)` (the increment), `refcounts[self@] < 255`, and the Err-arm `phys_view()` frame condition.
2. `drop`: `F' == F.release(self@)` (any functional postcondition at all).
3. `leak`: the no-release / `phys_view()`-unchanged guarantee.

**Acceptability assessment (as requested):** The deferral is **sound and
honestly documented**, and its root cause (0-arg `uninterp phys_view()` +
not-yet-verified `frame::free`/`frame::share`) is a genuine modeling boundary,
not a verification escape. The expressible snapshot facts are all correctly
captured. However, judged strictly against caller expectations and spec-design
§1.3 ("sufficient to reject bugs"), it is a **real coverage gap**: the central
reference-count invariant that fork/CoW callers depend on is not yet a verified
postcondition. It is acceptable *as a phase boundary* in bottom-up proving, but
it is **not** complete for a final, all-properties-realized sign-off.

---

## Proof Completeness

Counts scoped to upool files (`upool.rs`, `upool.spec.rs`, `upool.proof.rs`):

- `admit()`: **0** in upool. (The 4 `admit` the global detector reports are all in `manager.proof.rs:12/27/40/153` — **outside upool scope**; global admit total = 7.)
- `external_body` (actual attributes): **2** — `Upool::new` (upool.rs:250) and `Upool::alloc` (upool.rs:271). Both are TCB-approved.
- `UserFrame::{new, address, leak, share, refcount, drop}`: **fully machine-verified** (no `external_body`, no `admit`).
- `upool.proof.rs` is empty (`verus! { }`); `upool.spec.rs` defines only `UserFrame::inv()` (page-alignment).

No `admit()` ⇒ **no upool BLOCKER on this axis.**

---

## TCB Compliance

Exactly two `external_body` exist in upool; both are explicitly listed in
`tcb-allowed.md`:
- `Upool::new` — tcb-allowed.md:106-117 ("real contract `result@.wf()`; unprovable from an uninterpreted view ⇒ assumed §8 ghost-attachment axiom").
- `Upool::alloc` — tcb-allowed.md:118-124 ("delegates to `frame::alloc`; `external_body` until the frame free-function layer is verified").

`Upool` struct is explicitly **not** `external_body` (tcb-allowed.md:101-105,
"ELIMINATED"). **TCB compliance: PASS.**

---

## Guardrails Compliance (exact counts, upool scope)

| Dimension | Count | Locations | Verdict |
|---|---|---|---|
| `admit()` | **0** | — | PASS |
| `assume(...)` | **0** | — | PASS |
| `external_body` | **2** | upool.rs:250 (`Upool::new`), upool.rs:271 (`Upool::alloc`) | PASS — both in `tcb-allowed.md` |
| `assume_specification` | **0** | — | PASS |
| cfg-gated **exec** code | **0 semantic** | upool.rs:207 `#[cfg(not(verus_keep_ghost))]` gates only the `error!(...)` **logging macro** | PASS — logging-macro gating is explicitly sanctioned (verus-constraints) |
| `uninterp spec fn` | 1 | upool.rs:63 `View for Upool::view` | Note — mechanical consequence of the TCB `external_body` facade methods (documented); not an enumerated cheating dimension |
| `trusted` / `verifier::external` / `spinoff` / `rlimit` / `exec_allows_no_decreases` | **0** | — | PASS |

Other cfg gates (upool.rs:9,11,37 `#[cfg(verus_keep_ghost)]`) are the standard
`include!` of `.spec.rs`/`.proof.rs` and the `verus! { }` block guard — sanctioned
boilerplate, not exec gating.

`admit==0` and `assume==0` ⇒ **no guardrail BLOCKER.**

---

## AST Consistency (PASS, with documented benign deviation)

`ast_consistency.py … count` ⇒ "1 mismatched (7 functions match)".

`summary`: the single MISMATCH is `UserFrame::drop`. `diff --name "UserFrame::drop"`:

```
     fn drop(&mut self) {
         if let Err(e) = frame::free(self.addr) {
+            
             error!("failed to free user frame: {:?}", e);
         }
     }
```

The only difference is a blank line where the checker stripped
`#[cfg(not(verus_keep_ghost))]` from the `error!` logging macro (upool.rs:207).
The `error!` call itself is **preserved identically**; the exec behavior in a
normal `cargo build` is unchanged (the macro still runs). cfg-gating a logging
macro is an **explicitly pre-approved deviation** (verus-constraints: "Only
allowed on non-semantic items: derive, debug_assert!, logging macros") and
appears in the ast-consistency skill's pre-approved table. There is **no
`// VERUS REWRITE`** anywhere in upool. This is a known tooling artifact for
cfg-gated statements, **not a semantic exec mismatch** ⇒ **AST consistency: PASS
(no semantic blocker).**

---

## Verification (PASS)

`make verify-kernel MODULE=mm::phys` ⇒ **exit code 0 (PASS)**, cached run,
**0 errors** (commit `363553c54`: "86 verified, 0 errors"). **0 warnings** in the
log (the "low-confidence trigger" *notes* are informational and located in
`frame.rs`, outside upool scope; upool produces no warnings).

`spec_drift.py git-diff upool.rs --before HEAD` ⇒ exit 0, **no contract drift**
(0 ensures removed, 0 requires added). Working tree is clean vs HEAD; the
do-not-touch spec/view defs are unmodified.

Global cheating detector (whole `mm::phys`): admit=7, external_body=14,
cfg_gate=12 — **upool's share of these is external_body=2 (both TCB) and 0 admit**;
the admits live in `manager.proof.rs` (out of scope).

---

## Bug Summary

`bugs.md` records **"No code bugs found"** plus the §8 deferral note. Reconciling
against the final code:

- **No new code bugs discovered.** `new`/`address`/`leak`/`share`/`refcount` bodies are thin, correct wrappers; `drop` is a best-effort `frame::free` with logged error. The verified contracts hold (verus PASS).
- The bugs.md deferral note is **still valid and accurate**: `share`/`drop` (and `leak`) refcount transitions are not realized as ensures; correctly classified as an *intentional, sound limitation* (not a bug, not a False Positive code defect) per bug-reporting. No surviving *verification failure* exists (verus passes), so there is nothing to (re)classify as a True/Context-Dependent bug.
- The completeness gap on `share`/`drop`/`leak` is a **spec-coverage deferral**, not a code defect — the code is correct; the contract simply does not yet assert the refcount discipline.

---

## Issues (highest priority first)

1. **[Coverage / Spec sufficiency — HIGH]** `UserFrame::share`, `UserFrame::drop`,
   and `UserFrame::leak` do not realize the reference-count discipline that
   caller_analysis flags as the module's core safety property:
   - `share` Ok arm omits the `+1` increment (`add_ref`) and the headroom bound; Err arm omits the self/frame-unchanged frame condition.
   - `drop` has **no functional ensures** (the `release` transition is absent).
   - `leak` omits the no-release guarantee.
   These are spec-design §1.3 "sufficient to reject bugs" failures (a
   non-incrementing `share`, a leaking/double-freeing `drop`, or a freeing
   `leak` would each satisfy the current contracts). **Documented, sound,
   technically-forced deferral** (0-arg `uninterp phys_view()` + not-yet-verified
   `frame::free`/`frame::share`), to be lifted by the §8 ghost token when the
   frame free-function layer is proven. *No soundness/cheating violation; this is
   a completeness gap.*

2. **[Note — LOW]** `View for Upool::view` is `uninterp` (upool.rs:63). Sound and
   documented as a mechanical consequence of the TCB `external_body` `new`/`alloc`
   facade, but it is the single `uninterp spec fn` in upool — surfaced for
   transparency.

3. **[Note — INFO]** AST checker reports a MISMATCH on `UserFrame::drop` that is
   purely the stripped `#[cfg(not(verus_keep_ghost))]` on the `error!` logging
   macro — pre-approved, semantically equivalent, no action needed.

---

## Result: **FAIL** (strict; completeness only — no soundness/cheating/TCB/AST/verification blockers)

**Justification.** Every *hard* gate is clean: verus **PASSES** (exit 0, 0 errors,
0 warnings in scope); upool **admit=0, assume=0**; the only two `external_body`
(`Upool::new`, `Upool::alloc`) are **TCB-approved**; the single AST MISMATCH is a
pre-approved logging-macro cfg-gate (no semantic change, no `// VERUS REWRITE`);
**no spec drift**; **no code bugs**. There are **NO BLOCKERS** under the task's
blocker definition.

The review nonetheless returns **FAIL** under the STRICT rubric ("PASS only if ALL
checklist items pass") on a single checklist item — **Caller Coverage / Spec
Sufficiency**: the reference-count discipline of `UserFrame::share` (the `+1`),
`UserFrame::drop` (`release` — currently no functional ensures), and
`UserFrame::leak` (no-release) is **not yet realized as `ensures`**, so the
contracts for these three methods are not "sufficient to reject bugs" the caller
analysis explicitly identifies (share-without-increment, drop-that-leaks,
leak-that-frees). This is a **documented, sound, technically-forced §8 deferral**
(0-argument `uninterp phys_view()` constant + best-effort/unverified
`frame::free`/`frame::share`), to be discharged when the frame free-function
ghost-token layer is verified — not a cheating, soundness, or TCB violation.

A reviewer who accepts the documented deferral as an in-scope phase boundary could
reasonably score this **PASS**; under the strict "all properties realized for
final sign-off" reading applied here, it is **FAIL on completeness**, with a clear
and bounded remediation path (lift `add_ref`/`release` through the frame
free-function layer's §8 ghost token).
