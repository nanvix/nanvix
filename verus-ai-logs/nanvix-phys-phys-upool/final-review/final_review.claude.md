# Final Verification Review — `phys-upool`

**Reviewer:** independent strict final verification (Claude)
**Module:** `mm::phys`, target `src/kernel/src/mm/phys/upool.rs` (+ `.spec.rs`, `.proof.rs`)
**Base:** `verus-ai/phys-manager` → **Current:** `verus-ai/phys-upool` (working tree clean)

## VERDICT: **PASS**

- Blockers: **0**
- Caller coverage: **8/8** (all expressible facts covered; transition-deltas are inherent single-state limitations, documented)
- Guardrails (upool.rs): admit=0, assume=0, external_body=0 — all clean
- Central result (ground truth): `make verify-kernel MODULE=mm::phys` exit 0, PASS. Independently re-derived all static facts below; **no discrepancy** with the supplied central facts.

---

## 1. Spec Quality (per `spec-design`)

The eight in-scope functions are annotated directly on the original exec code via `#[verus_spec]` (no copied-into-`verus!{}` shadow). The `View for UserFrame` (`int` = physical address) and `UserFrame::inv()` (page-alignment) abstractions are caller-honest and minimal — exactly what `spec-design` Part-2 prescribes for an RAII handle over an external trust boundary.

Per-function judgement (upool.rs line refs):

- **`new`** (54–67): `requires addr.inv()`; `ensures ret@ == addr@ && ret.inv()`. Correct, minimal, declarative. `addr.inv()` (`FrameAddress::inv == addr@ % spec_page_size()==0`, `frame.rs:address/frame.rs:52`) is exactly what's needed to discharge `ret.inv()`. New precondition imposed on callers, but every caller feeds allocator/page-table addresses that are already page-aligned (and the upstream `alloc_user_frame` ensures `frame.inv()`).
- **`address`** (78–85): `ret@ == self@`. Pure getter, correct. Non-mutation guaranteed structurally by `&self`.
- **`leak`** (96–106): `ret@ == self@`. Correct. The "does not free / frame stays allocated" property is **structural** (`ManuallyDrop` suppresses `Drop`) and is **not expressible** as a `phys_view()` fact (single-state). Honest.
- **`share`** (124–147): success arm `handle@==self@ && handle.inv() && allocated_frames.contains(handle@)`; `inv()`+`initialized` preserved on both paths. The caller-relevant "**+1 refcount**" is *not* captured — only "frame is still allocated (gained a reference)". This is an honest single-state limitation (no `old(phys_view())`), not a careless omission; the weaker "still allocated" is the strongest monotone fact available.
- **`refcount`** (159–182): the strongest spec in the set. Ok arm: allocated + `count as int == refcounts[self@]`. **Err arm is meaningful** (`!allocated_frames.contains(self@)`), not tautological.
- **`drop`** (187–202): `ensures phys_view().inv()`, `opens_invariants none`, `no_unwind`. Matches the `frame::free` shim contract. "Releases exactly one reference / frees on last" is **not expressible** (single-state); honest limitation, documented in the spec header.
- **`Upool::new`** (233–236): trivial ZST constructor, `#[verus_verify]`, no contract needed.
- **`Upool::alloc`** (247–270): success arm captures `uf.inv() && allocated_frames.contains(uf@) && refcounts.contains_key(uf@) && refcounts[uf@]==1` — strong, including the exact refcount==1. Err arm `Err(_) => true`.

**Tautological `Err(_) => true` on `share` and `alloc`:** judged an **honest single-state limitation**, not a gap. `phys_view()` is a zero-arg `uninterp spec fn` (`mod.spec.rs:171`) — a single fixed value with no `old(phys_view())`, so "nothing changed on failure / parent untouched" is literally inexpressible. Crucially, `inv()` and `initialized` are stated **outside** the `match` (upool.rs:130–131, 252–253), so the error path still yields useful facts. "Parent untouched" for `share` is additionally guaranteed structurally by the `&self` signature.

**Subsumed conjunct (minor):** `refcounts.contains_key(x)` is implied by `allocated_frames.contains(x)` under `FrameAllocView::wf` (`mod.spec.rs:39–40`). It is restated in `alloc` (upool.rs:261), `refcount` (upool.rs:174), and the `share`/`refcount` `frame.rs` shims. **Not wrong** — it spares the caller an `wf` unfold and is good ergonomics (`spec-design` "written for the caller"). Logged as a nit only.

## 2. Caller Coverage (per `caller_analysis.md`)

Read `caller_analysis.md` in full and mapped each of the 8 caller expectations to a contract clause:

| Function | Caller expectation | Covered by | Verdict |
|---|---|---|---|
| `new` | `view()==addr@`, no alloc/refcount change, alignment | `ret@==addr@`, `ret.inv()`; no `phys_view` claim (correct) | ✅ |
| `address` | pure getter `==self@` | `ret@==self@` | ✅ |
| `leak` | consume w/o free, frame stays allocated | `ret@==self@`; no-free is structural (`ManuallyDrop`) | ✅ (no-free not expressible) |
| `share` | new alias `==self@`, **refcount +1**, **parent untouched on fail** | `handle@==self@`, `handle.inv()`, `allocated_frames.contains`; parent-untouched structural (`&self`) | ✅ (+1 not expressible; weaker "still allocated") |
| `refcount` | pure count query, count==refcount | `count as int==refcounts[self@]`, meaningful Err arm | ✅ (full) |
| `drop` | **releases exactly one reference**, frees on last, no unwind | `phys_view().inv()`, `no_unwind`, `opens_invariants none` | ✅ (exactly-one not expressible) |
| `Upool::new` | no alloc/global mutation | trivial constructor `#[verus_verify]` | ✅ |
| `Upool::alloc` | freshly allocated page-aligned frame, in `allocated_frames`, one ref; **nothing allocated on fail** | `uf.inv()`, `allocated_frames.contains`, `refcounts[uf@]==1`; fail = `Err(_)=>true` | ✅ (one-ref captured; "nothing on fail" not expressible) |

**Caller coverage: 8/8 covered + 0 missing.**

The four caller items singled out in the prompt:
- `share` "refcount +1" — **not** captured (single-state); weaker "still allocated" is. Honest limitation.
- `share` "parent untouched on failure" — guaranteed **structurally** by `&self`; `inv`/`initialized` preserved.
- `alloc` "nothing allocated on failure" — **not** expressible single-state; `inv`/`initialized` preserved.
- `drop` "releases exactly one reference" — **not** expressible single-state; `inv()` preserved, matches `frame::free`.

All four unexpressible deltas are inherent to the **do-not-modify** `phys_view()` design (an uninterpreted constant) and are consistent with the established `frame.rs`/`manager.rs` shim contract style. They are properly disclosed in `upool.spec.rs` (header, lines 12–19) and `bugs.md`.

## 3. Proof Completeness

- `admit()` in upool.rs: **0** (blocker count 0)
- `external_body` in upool.rs: **0** (blocker count 0)
- `upool.proof.rs` = `verus! { }` (empty — no lemmas needed; specs discharge directly through the `frame::*` shim contracts).

All eight contracts discharge by forwarding to already-specced `frame::*` shims + `UserFrame::new`. No proof obligations remain open.

## 4. TCB Compliance

`external_body` in the code this work touched:
- **upool.rs: 0** external_body (grep-confirmed).
- **frame.rs: 4** shims gained `#[verus_verify(external_body)]` + `#[verus_spec]` (bodies unchanged, additive-only diff): `frame::{alloc, free, share, refcount}` at frame.rs:710/762/849/874. **All four are listed in `tcb-allowed.md`** (lines 54–64). ✅

**Stale-doc finding (important):** `bugs.md` (Note, lines 11–15) and `tcb-allowed.md` (section "Allowed `external_body` — `UserFrame::drop`", lines 66–79) both describe `UserFrame::drop` as `external_body` (citing the `error!`/`{:?}` "Unsupported constant type" limitation). **The final committed `drop` is NOT `external_body`** (upool.rs:185–203): it carries `#[verus_verify]` + `#[verus_spec]`, retains the `error!("...{:?}", e)` call (upool.rs:200), and **verifies** (central exit 0). This is a **net positive — a previously-trusted `external_body` was eliminated** (likewise `Upool::new` dropped `external_body` → trivial `#[verus_verify]`, upool.rs diff). The corresponding allow-list/bugs entries are now **stale**. They are *not* a compliance violation (the allow-list is a ceiling, not a floor — an unused entry doesn't fail TCB), but they should be removed/updated to avoid masking a future regression. Logged under Issues.

## 5. AST Consistency (per `ast-consistency`)

- `// VERUS REWRITE` comments in upool.rs/.spec.rs/.proof.rs: **0** (confirmed) → no rewrite-equivalence to audit.
- Exec bodies are **byte-for-byte unchanged** from base (diff adds only attributes/contracts; `share`, `refcount`, `drop`, `alloc`, `new`, `address`, `leak` bodies untouched). No exec divergence between verified and runtime builds.
- The only relocation: `impl View for UserFrame` moved from a `#[cfg(verus_keep_ghost)] verus!{}` block in `upool.rs` into `upool.spec.rs` (included under `#[cfg(verus_keep_ghost)]`). The definition is **verbatim identical** (`closed spec fn view(&self)->int { self.addr@ }`), ghost-only, no exec impact.
- `drop` body (with `error!`) is identical in verified and exec builds — no `cfg`-gated swap, so AST consistency holds by construction.

**AST mismatches: 0.**

## 6. Verification

Relied on central ground truth (no local `make` run, per environment constraint): `make verify-kernel MODULE=mm::phys` → exit 0, PASS. Logically consistent with the static evidence: every contract forwards to a specced `frame::*` shim or `UserFrame::new`, no `admit`/`assume`, empty proof file, clean working tree on branch tip. `spec_drift git-diff base..tip` for upool.rs: contract-drift items = 4 but **ensures removed = 0** (the 4 are `requires`-added on functions that previously had no spec); `tip..tip` drift = 0 (committed == working tree).

## 7. Guardrails counts

**upool.rs (incl. .spec.rs/.proof.rs): admit=0, assume=0, external_body=0, assume_specification=0, cfg-gated-exec=0** (the 3 `cfg(...)` are `cfg(verus_keep_ghost)` ghost `include!`/`use`, not exec gating). Also no_decreases=0, trusted=0, `// VERUS REWRITE`=0. **No admit/assume → no guardrail blocker.**

## 8. Bug Reconciliation (per `bug-reporting`)

`bugs.md` records **"None"** for code bugs in the eight in-scope functions. Independently confirmed: each spec faithfully reflects (and does not over-claim about) the forwarding body; no buggy implementation satisfies these specs that the real code violates. No surviving verification failure exists to classify (exit 0).

Reconciling the two `bugs.md` "Notes":
1. **`drop` external_body note** — **STALE / superseded.** Final `drop` is not external_body and verifies; the `error!`/VIR limitation it cites no longer applies in the committed state. Net positive (cheating removed).
2. **`phys_view()` single-state / omitted `spec_add_ref`/`spec_drop_ref`/`spec_release` transitions** — **still valid.** Correctly explains why `share` "+1", `drop` "exactly-one-release", and error-path "no change" are inexpressible. Consistent with the do-not-modify design.

No unrecorded bug found.

## Blockers

_None._

## Issues / nits

1. **Stale TCB/bugs docs for `drop`** (non-blocking): `tcb-allowed.md:66–79` and `bugs.md:11–15` describe `UserFrame::drop` as `external_body`, but the final code (`upool.rs:185–203`) is `#[verus_spec]` and verifies. Remove/update both entries (the `external_body` was eliminated — a positive). Risk if left: a stale allow-list entry could mask a future re-introduction of `external_body` on `drop`.
2. **Doc drift in `caller_analysis.md`/`bugs.md`** (cosmetic): both state the `View for UserFrame` impl "already exists in upool.rs"; it was moved (verbatim) to `upool.spec.rs`. Semantically identical; update wording.
3. **Subsumed conjunct** (cosmetic, defensible): `refcounts.contains_key(x)` is implied by `allocated_frames.contains(x)` under `wf` (`mod.spec.rs:39–40`); restated in `alloc`/`refcount`/`share` ensures. Kept intentionally for caller ergonomics — acceptable, no action required.
4. **Inherent single-state coverage limits** (not fixable in scope): `share` "+1", `drop` "exactly-one-release", `alloc`/`share` error-path "no change", `leak` "no-free". All blocked by the do-not-modify `phys_view()` being an uninterpreted constant; properly disclosed. Would only be closeable if/when the frame-singleton bridge is verified with a two-state `phys_view`.
5. **New precondition on `new`** (`requires addr.inv()`): imposes page-alignment on callers that previously had no contract. Satisfied by all current callers (allocator/page-table addresses); benign within the verified build.
