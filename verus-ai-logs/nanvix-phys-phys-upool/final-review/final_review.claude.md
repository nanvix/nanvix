# Final Verification Review — `mm::phys::upool`

> Independent, strict review. All findings were re-derived from the source, spec,
> proof, frame-layer dependency contracts, and freshly re-run tooling — not taken
> on trust from the supplied summaries.

- **Module**: `kernel::mm::phys::upool`
- **Baseline (HEAD)**: `791e0c655`
- **In-scope functions** (8 + `Drop`): `UserFrame::new`, `UserFrame::address`,
  `UserFrame::leak`, `UserFrame::share`, `UserFrame::refcount`, `UserFrame::drop`,
  `Upool::new`, `Upool::alloc`.
- **Files**: `upool.rs`, `upool.spec.rs` (only `UserFrame::inv`), `upool.proof.rs`
  (empty `verus! { }`).

---

## Spec Quality

The contracts are **internally consistent, non-tautological, and faithful to the
dependency (frame) layer**, but several caller-relevant guarantees are **deferred**
because the global-state model (`phys_view()`) cannot express before/after
transitions. Details per function:

| Fn | Contract present | Assessment |
|----|------------------|------------|
| `UserFrame::new` | `req addr.inv()`; `ens result@==addr@, result.inv()` | Captures the address round-trip (the caller's mental model). `result.inv()` is derivable from `requires` (harmless, useful). Missing "no global effect" — not expressible (see below). |
| `UserFrame::address` | `req self.inv()`; `ens result@==self@, result.inv()` | Pure-read value captured. Missing no-side-effect clause. |
| `UserFrame::leak` | `req self.inv()`; `ens result@==self@, result.inv()` | Address preserved. The **defining** suppress-`Drop` guarantee (no refcount decrement) is **not** captured in the spec; correctness rests on the exec `ManuallyDrop` (which AST-consistency confirms is unchanged). |
| `UserFrame::share` | Ok: `uf@==self@, uf.inv(), allocated_frames.contains(self@)`; Err: `!contains(self@) \|\| refcounts[self@]>=255` | **Exactly forwards** `frame::share`'s snapshot contract (frame.rs:910-919). Prevents the "aliases a different frame" break (`uf@==self@`). Does **not** capture the refcount **+1** transition — the operation's stated raison d'être. Err arm is meaningful (failure cause), **not** `Err(_)=>true`. Missing "self/frame unchanged on Err". |
| `UserFrame::refcount` | Ok: `contains(self@) && count==refcounts[self@]`; Err: `!contains(self@)` | **Exactly forwards** `frame::refcount` (frame.rs:928-938). Strong and complete for the value read. Missing no-mutation clause. |
| `UserFrame::drop` | `opens_invariants none, no_unwind` only | **No functional postcondition.** `frame::free` is `ensures true` (frame.rs:832-834), so no `release` semantics can be derived. Honest but empty. |
| `Upool::new` | `ens result@.wf()` | Complete for its single boot caller. `external_body` (opaque-struct constructor). |
| `Upool::alloc` | full `requires/ensures` (`alloc_one`, `free_count()==0` Err arm) | **Strong and complete** — both arms, incl. error-path state preservation and exhaustion. `external_body`. |

**No tautological ensures** (`Err(_) => true`) anywhere. **No subsumed/contradictory
clauses.** The error arms of `share`, `refcount`, and `alloc` all carry real meaning.

**Root cause of the gaps (single, documented):** `phys_view()` is a parameter-free
`uninterp spec fn` (a global accessor), so `old(phys_view())` cannot be written and
before/after global-partition transitions (`F' == F.add_ref(self@)` for `share`,
`F' == F.release(self@)` for `drop`) are **not expressible** at this layer. The
frame-layer wrappers `frame::share`/`frame::free`/`frame::refcount` themselves carry
only snapshot/`true` postconditions ("`external_body` until the free-function layer
is verified"). `upool` therefore claims **exactly** what its dependency guarantees —
no weaker, no stronger — which is the correct behavior for a thin facade. The
stronger transitions designed in `view_design.md` §4 were explicitly deferred to a
proving-phase ghost token that was **not** realized (`proof.rs` is empty;
`bugs.md` "Proving phase outcome"). These are documented intentional deferrals, not
silent weakenings (drift = 0) and not cheating (admit/assume = 0).

---

## Caller Coverage

Source of expectations: `caller_analysis.md`. Counting whether each function's
**primary** caller-relied fact is captured by a `requires`/`ensures`:

**Covered 6 / 9** (fully): `UserFrame::new`, `UserFrame::address`,
`UserFrame::refcount`, `Upool::new`, `Upool::alloc`, and `View for UserFrame`
(address abstraction).

**Partial 2 / 9**:
- `UserFrame::leak` — address preserved ✓, but the *suppress-Drop / no-double-free*
  invariant (caller_analysis lines 57-59) is enforced only by exec `ManuallyDrop`,
  not by the spec.
- `UserFrame::share` — same-frame alias (`uf@==self@`) ✓, but the **refcount
  increment** that fork/CoW callers depend on (caller_analysis lines 64-74:
  "succeeding without actually incrementing the refcount … would cause premature
  free") is **not** in the contract; nor is "parent untouched on `Err`".

**Missing 1 / 9**:
- `UserFrame::drop` (trait obligation) — the "releases exactly one reference,
  reclaims on last" service (caller_analysis lines 18-24) has **no** functional
  postcondition. Forced by `frame::free`'s `ensures true`.

All "Missing/Partial" items trace to the same frame-layer / `phys_view()` limitation
and are documented in `bugs.md` and `view_design.md` §8. None is a silent omission.

---

## Proof Completeness

- **`admit()` in upool files: 0** (BLOCKER threshold: any > 0). `upool.rs` 0,
  `upool.spec.rs` 0, `upool.proof.rs` 0 (file is `verus! { }`).
- **`external_body` in upool files: 2 attributes** —
  - `upool.rs:246` `Upool::new`
  - `upool.rs:272` `Upool::alloc`
  (3 further textual `external_body` matches at lines 57, 240, 266 are **prose
  comments**, not attributes.)
- All six `UserFrame` methods verify against their contracts with **no proof body**
  (the `UserFrame::inv ⇔ FrameAddress::inv` alignment discharges the frame layer's
  `frame.inv()` preconditions). Confirmed: `proof.rs` empty, verify 0 errors.

---

## TCB Compliance — **PASS**

Both `external_body` functions are already listed in `tcb-allowed.md`:
- `Upool` (struct) and `Upool::new` — "opaque type/callee … Verified when upool is."
- `Upool::alloc` — "pool allocation primitive … Verified when upool is."

No new trust boundary is introduced. Rationale matches `bugs.md`: `Upool::new`
constructs an opaque datatype (Verus forbids a constructor in a checked body), and
`Upool::alloc`'s `old(self)@ → final(self)@` (`alloc_one`) is over the `uninterp`
`Upool::view`, which has no axiom tying it to the `phys_view()` that `frame::alloc`
mutates — so the transition is underivable in-body. The `Upool` struct is genuinely
stateless (`_private: ()`); `uninterp` view + `external_body` is the only honest
model (view_design §7, rejected alt #3).

*Note (non-blocking):* `tcb-allowed.md` files these two under "eliminated when their
module is verified." This **is** that review, and they are not eliminated. Per the
analysis above this elimination is not achievable without the deferred ghost-token
machinery; it is a faithful, sound trust boundary, not a verification escape. The
task's blocker criterion ("`external_body` not in `tcb-allowed.md`") is **not**
triggered — both are listed.

---

## Guardrails Compliance (exact counts, upool files only)

| Dimension | Count | Verdict |
|-----------|-------|---------|
| `admit(` | **0** | OK (>0 would BLOCK) |
| `assume(` | **0** | OK (>0 would BLOCK) |
| `external_body` (attributes) | **2** (`Upool::new`, `Upool::alloc`) | OK — both in `tcb-allowed.md` |
| `assume_specification` | **0** | OK |
| cfg-gated **exec** | **0** | OK |

The single `#[cfg(not(verus_keep_ghost))]` (upool.rs:205) guards an `error!` logging
macro inside `Drop` — the **allowed logging exception**, not an exec gate. The three
`#[cfg(verus_keep_ghost)]` (lines 9, 11, 37) gate the `.spec.rs`/`.proof.rs`
includes and the `View` impl `verus! { }` block — spec material, allowed.

**No `admit`, `assume`, `assume_specification`, or exec cfg-gating.** PASS.

---

## AST Consistency — **PASS**

`ast_consistency.py --base-ref HEAD upool.rs count` → `✅ Consistent: 8 functions,
2 structs match.` Exit 0. No `// VERUS REWRITE` markers (grep = 0). No exec
divergence; all changes are spec/attribute additions only.

---

## Verification — **PASS (0 errors)**

`make verify-kernel MODULE=mm::phys` → exit 0 (cached, no recompilation). Underlying
Verus result: **42 verified, 0 errors**. Module-level "CHEATING_DETECTED" reflects
counts across the **whole** `mm::phys` module (admit=24, external_body=17, cfg=15 —
all in `frame.rs`/`manager.rs`/`mod.rs`, out of scope and in `tcb-allowed.md`). The
**upool** contribution is `upool.rs:251 new` and `upool.rs:289 alloc` (both TCB),
**0 admit**, **0 assume**.

---

## Bug Summary

`bugs.md` reconciled against final code — **accurate**:
- "No code bugs found" (spec + proving phases) — **confirmed**. No arithmetic,
  off-by-one, bounds, or cast bug exists in the eight thin functions.
- "All six `UserFrame` methods verify with no proof body" — **confirmed** (empty
  `proof.rs`, 0 errors).
- "`Upool::new`/`Upool::alloc` are `external_body` (in tcb-allowed)" — **confirmed**.
- Deferred-modeling note (no `add_ref`/`release` transition for `share`/`drop`;
  snapshot-only facts) — **confirmed** and traced to the frame layer. Classified as
  **verification limitation / intentional deferral**, not a code bug.

No additional bug surfaced during this review. The `share`/`drop` spec gaps are
**not** code defects (the exec is correct: `share` calls `frame::share`, `drop`
calls `frame::free`); they are contract-strength limitations of the dependency layer.

---

## Issues (highest priority first)

1. **[Medium — documented deferral] `UserFrame::share` omits the refcount-increment
   transition.** The contract proves the frame stays allocated but not that the
   refcount actually increased (`F' == F.add_ref(self@)`). CoW/fork callers
   *semantically* depend on this. Root cause: `frame::share` exposes only a snapshot
   contract and `phys_view()` cannot express `old`/`new`. Not fixable within upool;
   requires lifting `Inner::share`'s transition + the proving-phase ghost token.

2. **[Medium — documented deferral] `UserFrame::drop` has no functional
   postcondition.** The RAII `release` guarantee (the basis of automatic error-path
   cleanup) is unspecified because `frame::free` is `ensures true`. Honest but empty.

3. **[Low — documented deferral] `leak`/`new`/`address`/`refcount` lack
   no-side-effect (`phys_view()` unchanged) clauses.** Same `phys_view()`
   param-free-global limitation; `leak`'s suppress-Drop correctness rests on the
   (AST-verified-unchanged) exec `ManuallyDrop`.

4. **[Informational] TCB "eliminate when module verified" not achieved for
   `Upool::new`/`Upool::alloc`.** Sound and listed, but the aspirational elimination
   noted in `tcb-allowed.md` remains open, pending the ghost-token realization.

All four are **intentional, documented deferrals** rooted in a single frame-layer /
global-accessor modeling limitation — none is a silent weakening (spec drift = 0),
a cheating primitive (admit/assume = 0), or an unlisted trust boundary.

---

## Result — **PASS**

All hard gates are clean:

- Proof completeness: **0 admit**, **0 assume** in upool files.
- TCB: both `external_body` (`Upool::new`, `Upool::alloc`) are in `tcb-allowed.md`.
- Guardrails: admit 0 / assume 0 / external_body 2 (TCB) / assume_specification 0 /
  exec cfg-gate 0.
- AST consistency: PASS (8 fns, 2 structs match; 0 rewrites).
- Verification: PASS — 42 verified, **0 errors**.
- Spec drift vs HEAD: **0** — no original guarantee weakened.
- Bug reconciliation: `bugs.md` accurate; no undiscovered code bug.

The specs are weaker than `view_design.md` §4 intended for `share`/`drop`/`leak`,
but those gaps are **faithful forwards of the deferred frame-layer contracts**, are
fully documented as intentional deferrals, and constitute **zero** of the defined
blocker categories. Verdict: **PASS** (with the Issues above recorded as known,
non-blocking deferrals to be closed when the frame free-function layer and the
singleton ghost token are verified).
