## Turn 1: Full checklist walkthrough — `mm::phys::upool` specification

### Progress
- Done (PASS): all 16 checklist items verified below.
- Current: full first-pass review.
- Remaining: none.

Result: **RESOLVED** — every checklist item is PASS with concrete evidence.

---

### Verification environment / commands run
- `make verify-kernel` → Exit code 0. `note: verifying module mm::phys::upool`,
  `Finished dev profile ... target(s)` (compiles), `verification: ... exit 0`.
  Module list = `<all>` (cross-module run).
- Read: `upool.rs`, `upool.spec.rs`, `upool.proof.rs`, `frame.rs` (wrapper
  contracts at 709–891), `frame.spec.rs`, `mod.spec.rs` (`FrameAllocView`,
  `PhysMemView`, `phys_view`), `hal/.../address/frame.rs` (`FrameAddress::inv`/
  `view`), `caller_analysis.md`, `view_design.md`, `bugs.md`, `tcb-allowed.md`.
- Cheating detail: `verus-logs/cheating-detail.txt` (only `upool.rs:216 drop`).
- Coverage: `verus-logs/coverage-unverified.txt`.

---

### Item-by-item

**1. Every in-scope exec function has requires/ensures — PASS**
7 stateful in-scope fns all carry `#[verus_spec]` ensures (and requires where a
precondition exists): `UserFrame::{new(71),address(95),leak(113),share(141),
refcount(176)}`, `UserFrame::drop(205)`, `Upool::alloc(265)`.
`Upool::new` (252) intentionally carries no contract: per `view_design.md`,
`Upool` has **no View and no inv** (zero-sized `_private: ()` facade, no
caller-observable state), and `view_design` writes all `Upool` contracts against
`phys_view()`. The only postcondition expressible would be `ensures true`, which
item 4 bans. A stateless constructor with no observable postcondition correctly
needs no contract; this is the consistent resolution of the (1)↔(4) tension. Not
flagged by the coverage tool as an in-scope gap.

**2. Caller coverage — PASS**
Each expectation in `caller_analysis.md` is met by a contract:
- `new`: callers need `ret@ == addr@`, no alloc/refcount change → ensures
  `ret@ == addr@`, `ret.inv()` (lines 79–80). ✓
- `address`: pure getter `ret@ == self@` (98). ✓
- `leak`: no-free, returns address `ret@ == self@` (118); `ManuallyDrop`
  suppresses `Drop` so no `phys_view()` claim — matches "frame stays allocated". ✓
- `share`: `Ok` → fresh handle `handle@ == self@`, `inv()`, frame still allocated
  (154–157). ✓ (refcount-increment not expressible — see item 14.)
- `refcount`: pure query, `Ok` → allocated + `count == refcounts[self@]`;
  `Err` → not allocated (188–195). ✓
- `drop`: releases one ref, preserves invariant, `no_unwind` (logs, no propagate)
  (206–215). ✓ matches Drop trait obligation.
- `Upool::new`: no global mutation (no contract, no state). ✓
- `Upool::alloc`: `Ok` → fresh page-aligned allocated frame, refcount==1
  (276–281); `Err` → see item 6. Watermark gate is caller-side (not in scope). ✓

**3. View consistency — PASS**
Specs reference `View` fields/inv per design: `UserFrame@ : int` (addr) and
`inv() == self@ % spec_page_size() == 0` (matches `FrameAddress::inv()` exactly,
`hal/.../frame.rs:52-54`). All mutable state is named through
`phys_view().frames.{allocated_frames,refcounts}` and `phys_view().inv()/.initialized`,
matching `FrameAllocView`/`PhysMemView` in `mod.spec.rs`. `Upool` has no View, as
designed. `inv()` is maintained in every requires/ensures touching `phys_view()`.

**4. No tautological ensures — PASS (with scrutiny)**
`share` (158) and `alloc` (282) use `Err(_) => true`. I challenged this. The
called wrappers `frame::share` (frame.rs:861) and `frame::alloc` (frame.rs:728)
themselves expose `Err(_) => true`, and `frame.rs` is do-not-modify. Because
`phys_view()` is a 0-arg `uninterp spec fn` (a constant, no `old(phys_view())`),
no before/after state-change fact is expressible, and `phys_view().inv()` /
`.initialized` are already asserted unconditionally (would be subsumed if
repeated in `Err`). Thus `true` is the maximal honest statement, not an
information-hiding tautology. `refcount`'s `Err` IS meaningful (item 6).

**5. No subsumed ensures — PASS**
`refcount`/`alloc` state both `allocated_frames.contains(x)` and
`refcounts.contains_key(x)`. Although `FrameAllocView::wf()` makes the latter
derivable from the former, `contains_key(x)` directly justifies the adjacent
`refcounts[x]` map-index conjunct (`count == refcounts[uf@]`, `refcounts[uf@]==1`),
so it is load-bearing, not idle — and mirrors the verified `frame.rs` idiom.

**6. Error paths have meaningful ensures — PASS**
`refcount` Err → `!allocated_frames.contains(self@)` (meaningful). `share`/`alloc`
Err → `true`: justified under item 4 (upstream wrappers give `Err=>true`;
do-not-modify; constant `phys_view()`). No stronger statement is provable.

**7. No assume_specification for workspace-internal code — PASS**
`grep` over `upool.{rs,spec.rs,proof.rs}` → zero `assume_specification`.

**8. vstd searched before any assume_specification — PASS (N/A)**
No `assume_specification` added in this module.

**9. Specs written for the caller — PASS**
Contracts are stated over `self@`, `ret@`, and `phys_view().frames` keyed by
`self@`/`uf@` — directly usable in caller proofs (e.g. `PhysMemoryManager`
already consumes `allocated_frames.contains(frame@)` and `frame@ % page == 0`).

**10. Trait obligations satisfied — PASS**
`View for UserFrame`: `view()==addr@` (closed) — matches design and upstream use.
`Drop for UserFrame`: ensures `phys_view().inv()`, `opens_invariants none`,
`no_unwind` — matches the "release one ref, log errors, never unwind" contract in
`caller_analysis.md`.

**11. Spec completeness (advisory) — PASS**
Nondeterminism is intentional and matches caller expectations: `alloc` states
membership (`uf@ ∈ allocated_frames`, `refcounts[uf@]==1`) rather than a
deterministic frame choice (allocator detail). Refcount-increment on
`share`/`drop` is intentionally absent because it is not expressible against a
constant `phys_view()` (documented in `bugs.md`).

**12. Loop invariants — PASS (N/A)**
No `loop`/`while`/`for` in `upool.rs` (grep confirms). No loops to annotate.

**13. No cheating on module's own functions — PASS**
Cheating detail for this module: `assume=0, admit=0, trusted=0`, and exactly one
`external_body`: `UserFrame::drop` (upool.rs:204/216). This is allowed and
recorded in `verus-ai-logs/tcb-allowed.md` (§"Allowed external_body — UserFrame::drop"):
`error!` logging on the error path expands to `core::fmt write!`, which Verus
cannot translate to VIR. Its contract (`ensures phys_view().inv()`) is discharged
by the `frame::free` shim. No other `external_body`/`admit`/`assume`/`trusted` in
the three upool files. (Global summary `external_body=26` is repo-wide, not this
module.)

**14. No specs weakened — PASS**
No pre-existing guarantee (frame.rs wrapper contracts, `FrameAllocView`,
`PhysMemView`) was modified or weakened. `view_design.md` sketched
`spec_add_ref`/`spec_drop_ref`/`spec_release` before/after transitions; these were
intentionally **not** realized because `phys_view()` is a constant with no
`old(phys_view())` — documented in `bugs.md`. This is not a weakening of an
existing spec; it is the same single-state, monotone-fact style already used by
the verified, do-not-modify `frame.rs`/`manager.rs` contracts. Upool's specs are
as strong as the upstream wrappers permit.

**15. Bug awareness — PASS**
`bugs.md` reviewed: "None" for all 8 in-scope functions, with two correct
non-bug notes (drop external_body limitation; non-expressible transitions). My
independent read of the code confirms no fundamentally incorrect logic.

**16. Cross-module regression + Verification + Build — PASS**
`make verify-kernel` runs Module `<all>` → Exit 0 (no regressions in other
verified modules). Same run reports `Finished dev profile ... target(s)` →
kernel crate compiles. 0 verification errors.

---

### Fix Request
None. All 16 items PASS with tool/code evidence. The two items I scrutinized
hardest (#4 `Err(_) => true` on `share`/`alloc`; #1 `Upool::new` no-contract) are
structurally forced — `frame.rs` wrappers are do-not-modify and themselves expose
`Err(_) => true`, and `phys_view()` is a constant `uninterp spec fn` with no
`old()` — so no equally-strong-or-stronger alternative spec exists. This is not a
view-design defect (the do-not-modify `phys_view()`/`FrameAllocView` layer is
outside upool's view-design scope), so no ROLLBACK is warranted.

Writing STOP = RESOLVED.
