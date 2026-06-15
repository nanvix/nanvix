## Turn 1: Full checklist walk — FAIL on "No subsumed ensures"

### Progress
- Done (PASS): items 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 (see Verification).
- Current (FAIL): **Item 5 — No subsumed ensures.** The bidirectional liveness
  clause in `from_address` is fully derivable from its own `match` arms.
- Remaining: re-verify item 5 after fix; then RESOLVED.

### Verification

Environment / tools run:
- `make verify-kernel` → exit 0. `note: verifying module
  hal::mem::types::address::aligned::page`; `verification: cached ... (exit 0)`.
  Module verifies, cross-module run is `<all>` and passes.
- `./z build` (normal, non-ghost build) → `[OK] Build complete.` (exit 0).
- Cheating scan (`verus-logs/cheating-detail.txt`) filtered to this module:
  - `page.rs:65 from_address: external_body` (1)
  - `page.spec.rs:50 <PageAligned<T> as Address>::into_raw_value: assume_specification` (1)
  - `admit=0 assume=0 trusted=0` in the module.
- Read: `page.rs`, `page.spec.rs`, `page.proof.rs`, `caller_analysis.md`,
  `view_design.md`, `bugs.md`, `tcb-allowed.md`, sibling `frame.rs`,
  `kframe.spec.rs`.

Item-by-item:

1. **In-scope exec fns have requires/ensures** — PASS. Scope (per
   `caller_analysis.md`) = `from_address`, `into_raw_value`, type `PageAligned<T>`.
   `from_address` has `#[verus_spec] ensures`; `into_raw_value` specced via
   `assume_specification` (`result as int == addr@`). Other 15 fns explicitly
   out-of-scope ("do not modify").
2. **Caller coverage** — PASS. `caller_analysis.md` expects: `from_address` Ok ⇒
   `result@ == addr@` ∧ aligned, Err ⇒ unaligned; `into_raw_value` ⇒
   `result as int == self@`. All present.
3. **View consistency** — PASS. Specs reference `self@`/`spec_addr(&addr)` and
   `inv()`; `from_address` Ok arm establishes `p.inv()`.
4. **No tautological ensures** — PASS. Err arm is `spec_addr(&addr) % page != 0`
   (not `Err(_) => true`).
5. **No subsumed ensures** — **FAIL.** See Fix Request.
6. **Error paths meaningful (match style)** — PASS. `from_address` uses
   `match { Ok => ..., Err => ... }` with a meaningful Err arm; `into_raw_value`
   is total.
7. **No assume_specification for workspace-internal code** — PASS (accepted trust
   boundary). The lone `assume_specification` is on the external `sys::mm::Address`
   trait method `<PageAligned<T> as Address>::into_raw_value`. Body-verifying it
   requires marking the whole generic `impl Address for PageAligned<T>` verified,
   which triggers a Verus front-end panic (`vir/src/traits.rs:511 assertion
   failed: !method_impls.contains(&p)`, documented in `bugs.md`). It mirrors the
   pre-existing identical boundary `<PageAligned<T> as Address>::from_raw_value`
   in `kframe.spec.rs:33` and is allowlisted in `tcb-allowed.md:217-233`.
8. **vstd searched first** — PASS. The boundary is a kernel trait method, not a
   vstd-substitutable item; rationale documented.
9. **Specs written for the caller** — PASS. Identity + alignment + (derivable)
   liveness, directly usable in caller proofs (`vmem.rs`, `region.rs`, etc.).
10. **Trait obligations satisfied** — PASS. `into_raw_value` newtype-identity
    matches the `Address` contract callers rely on for offset/page-walk math.
11. **Spec completeness (advisory)** — PASS. No nondeterminism; identity,
    alignment, and liveness all captured.
12. **Loop invariants** — PASS (N/A: no loops in module).
13. **No cheating on module's own functions** — PASS (accepted, individually
    addressed). Two boundaries, each a genuine tool limitation, allowlisted:
    - `from_address` `external_body` (`page.rs:51`, `tcb-allowed.md:211`): body
      calls `<T as Address>::is_aligned(PAGE_ALIGNMENT)`; `is_aligned` is an
      unspecced `sys` trait method and `PAGE_ALIGNMENT` is an `arch` `Alignment`
      enum constant Verus cannot translate (`error: arch::x86::mem::constants::
      PAGE_ALIGNMENT is not supported`). Mirrors `FrameAddress::into_raw_value`
      (`frame.rs:103`).
    - `into_raw_value` `assume_specification` — see item 7.
    `admit=assume=trusted=0`.
14. **No specs weakened** — PASS. No `spec_drift_cmd` exists in-repo; reviewed by
    hand. `inv()` unchanged (`self@ % spec_page_size() == 0`). View bound was
    broadened `T: Address + View<V=int>` → `T: Address` (only *adds*
    availability) and `view` delegates through closed `spec_addr`; invisible to
    consumers, no guarantee weakened. The Item-5 fix below removes a *derivable*
    clause, so it does not weaken either.
15. **Cross-module regression** — PASS. `make verify-kernel` ran module `<all>`,
    exit 0.
16. **Verification + build** — PASS. `make verify-kernel` exit 0 (0 errors);
    `./z build` exit 0.

### Fix Request (Item 5 — subsumed ensures)

**Where:** `src/kernel/src/hal/mem/types/address/aligned/page.rs`, the
`#[verus_spec]` on `from_address`, line 63:

```rust
(result is Ok) <==> (spec_addr(&addr) % crate::hal::mem::spec_page_size() == 0),
```

**Problem:** this bidirectional clause is **fully derivable from the `match`
arms** (lines 59–62), hence subsumed:
- `Ok ⇒ aligned`: Ok arm gives `p@ == spec_addr(&addr)` and `p.inv()` (i.e.
  `p@ % page == 0`); substitute ⇒ `spec_addr(&addr) % page == 0`.
- `aligned ⇒ Ok`: contrapositive is `Err ⇒ not aligned`, which is exactly the
  Err arm `spec_addr(&addr) % page != 0` (`Result` is total).

So the Err arm already supplies liveness; the explicit iff adds nothing. (The
`view_design.md` lines 149–152 even acknowledge "the liveness clause is
derivable".)

**Change:** delete line 63 (and trim the "stated both ways for liveness"
sentence in the comment on lines 57–58 so the comment stays accurate). Keep the
`match` arms — both are needed (`p.inv()` is *not* subsumed; without it the Ok
branch carries no alignment fact).

**Verify after fix:**
1. `make verify-kernel` → expect exit 0, module
   `hal::mem::types::address::aligned::page` still verifies (the removed clause
   was derivable, so no caller proof can regress, and `frame.rs`/`region.rs`
   callers that pre-align and use `?` still get liveness from the Err arm).
2. `./z build` → expect `[OK] Build complete.`
3. Re-grep to confirm only the two allowlisted boundaries remain:
   `grep -n "spec_page_size() == 0)" page.rs` should no longer show the iff line.

Do **not** justify keeping the clause — it is logically derivable; if you
believe the Err arm is insufficient for liveness, show a Verus proof that fails
without line 63. Otherwise remove it.
