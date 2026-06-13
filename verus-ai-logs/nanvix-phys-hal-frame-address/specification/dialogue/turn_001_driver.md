## Turn 1: Full checklist walkthrough — Specification phase, `hal::mem::types::address::frame`

### Progress
- Done (PASS/FIXED with evidence): all 17 checklist items (see Verification).
- Current: full sweep completed.
- Remaining: none.

### Verification

Commands run:
- `make verify-kernel` → module `hal::mem::types::address::frame` verified, **Exit code 0**
  (cached, no recompilation). Global cheating tally `assume=0 external_body=12
  admit=28 trusted=0` are pre-existing in `mm/phys` etc., **not** introduced here.
- `./z build -- all-kernel` → **Exit code 0** (`Build complete.`); BUG-001 (duplicate
  `vstd::prelude` import) stays fixed, non-ghost build clean.
- `fn_coverage.py /tmp/frame_orig.rs frame.rs` → 9 source exec / 9 verus exec, **0 missing,
  0 extra**.
- `git diff verus-ai/sys-virt-address -- frame.rs` → drift inspection (below).
- Per-file pattern counts: `frame.rs` admit=0 external_body=0 trusted=0
  assume_specification=1 (`::arch::mem::PAGE_SIZE`); `frame.spec.rs`
  assume_specification=1 (`<PhysicalAddress as Address>::from_raw_value`);
  `frame.proof.rs` admit=2 (lemma bodies; a 3rd grep hit is the header comment).

In-scope functions (per `caller_analysis.md` / `view_design.md`): `from_raw_value`,
`into_raw_value`, `from_frame_number`, `into_frame_number`, `FrameAddress` (type).
Out of scope: `new`, `into_physical_address`, `into_page_address`, `Debug::fmt`,
`PartialEq::eq`.

**1. Every in-scope exec fn has requires/ensures — PASS.**
All four in-scope exec fns carry `#[verus_spec]` with `ensures` (and `into_frame_number`
also `requires`). `fn_coverage.py` confirms full presence; the unspecced fns are all
out of scope.

**2. Caller coverage — PASS.** Each `caller_analysis.md` expectation is met:
- `from_raw_value`: callers need `fa@ == raw_addr` and `Ok ⇒ inv()` → ensures
  `Ok(fa) => fa.inv() && fa@ == raw_addr as int`. ✓ (note: base branch only had
  `Ok ⇒ inv()`; `fa@ == raw_addr` was added — strengthened.)
- `into_raw_value`: `result as int == self@`. ✓
- `from_frame_number`: callers need `fa@ == n·PAGE_SIZE`, `Ok ⇒ inv()`, never fails →
  ensures `result is Ok`, `inv()`, `@ == spec_from_number(spec_frame_raw_value(frame))`. ✓
- `into_frame_number`: inverse of `from_frame_number` → ensures
  `spec_frame_raw_value(result) == spec_frame_number(self@)` and
  `spec_from_number(spec_frame_raw_value(result)) == self@`. ✓ Round-trips
  `from_frame_number(n).into_frame_number() == n` and
  `into_frame_number(fa)·PAGE_SIZE == fa@` both follow.

**3. View consistency — PASS.** Specs are stated purely on `self@` (the `int` view) and
`inv()`, exactly the `type V = int` / `inv() = self@ % spec_page_size() == 0` design in
`view_design.md`. No representation (`PageAligned<PhysicalAddress>`) leaks. `inv()` is
established on every `Ok` and consumed by `into_frame_number`'s `requires`.

**4. No tautological ensures — PASS (with note).** `from_raw_value` has `Err(_) => true`.
Investigated: its `Err` arises from EITHER `PhysicalAddress::from_raw_value` (physical-RAM
validity, deliberately *unmodeled* — its own assumed spec is `Err ⇒ true`) OR
`PageAligned::from_address` (`Err ⇒ !spec_aligned`). Because one failure mode is
unmodeled and the two are disjunctive, **no stronger `Err` postcondition is provable**
(unlike sibling `PageAligned::from_address`, which has a single failure mode and so can
write `Err ⇒ !spec_aligned`). Callers (`boot_init`, `mm/phys/manager`) propagate with `?`
and ignore the reason. This is intentional, evidence-backed nondeterminism, not a lazy
tautology. `from_frame_number` uses `result is Ok` (no vacuous arm). Advisory only:
the equivalent `result matches Ok(fa) ==> ...` form would drop the literal `=> true` arm.

**5. No subsumed ensures — PASS.**
- `from_frame_number`: `inv()` is not auto-derivable by the verifier from `@ == frame_raw·page_size`
  (needs nonlinear `lemma_frame_base_aligned`); callers also want `inv()` directly. Not subsumed.
- `into_frame_number`: the second ensures (`spec_from_number(...) == self@`) is not
  auto-derivable from the first + `inv()` without `lemma_aligned_div_mul` (nonlinear).
  Both independently useful. Not subsumed.

**6. Error paths meaningful — PASS (ties to #4).** `from_frame_number` proves `result is Ok`
(strongest possible). `from_raw_value`'s `Err` is unconstrainable (unmodeled validity);
acceptable and matches caller needs.

**7. No assume_specification for workspace-internal code — PASS (tracked).**
`frame.spec.rs` assumes `<PhysicalAddress as ::sys::mm::Address>::from_raw_value`.
Confirmed: `impl Address for PhysicalAddress` in `phys.rs` carries **no** `#[verus_spec]`
(lines 167–184), i.e. it is a genuinely not-yet-verified intra-crate dependency for the
module-by-module pipeline. This mirrors the documented, sanctioned pattern in
`phys.spec.rs` (the `VirtualAddress::new` placeholder that was later removed once the
real spec landed). The frame comment commits to removing it "once phys's Address impl
carries its own `#[verus_spec]`." No `vstd` spec exists for a kernel-defined type.
Acceptable as a temporary dependency contract; **must be removed when phys's `Address`
impl is verified.**

**8. vstd searched before assume_specification — PASS.** Both assume_specifications target
non-`vstd` items: `::arch::mem::PAGE_SIZE` (arch constant) and a kernel-type trait method.
No vstd equivalents exist.

**9. Specs written for the caller — PASS.** Contracts are expressed in caller vocabulary
(`self@`, `inv()`, frame-number identities via `FrameNumber`'s view), directly usable in
the page-table/allocator caller proofs without exposing internals.

**10. Trait obligations satisfied — PASS.** `Debug::fmt` prints `into_raw_value()` as the
raw page-aligned address — honored by `result as int == self@`. `PartialEq::eq` is
structural on the inner address; since the view is the address, equality ⇔ equal `self@` ⇔
same physical frame, matching CoW caller expectations. Both trait fns are out of scope
(no spec required) and the in-scope specs are consistent with their semantics.

**11. Spec completeness (advisory) — PASS.** Ok-paths fully pin the view
(`from_frame_number`, `from_raw_value`, both queries deterministic). Only `from_raw_value`'s
Err is nondeterministic, which matches caller expectations (#4). `into_frame_number`'s
`requires spec_frame_number(self@) <= spec_max_frame_number()` is a deliberate,
view-design-documented frame-representability precondition inherited from
`PhysicalAddress::into_frame_number`; not foldable into `inv()` without strengthening the
invariant relied on by already-verified `mm/phys` callers.

**12. Loop invariants — PASS (N/A).** No loops in any in-scope function.

**13. No cheating on module's own functions — PASS for spec phase (tracked).** Exec code is
clean: `frame.rs` has 0 `admit`/`assume`/`external_body`/`trusted`. Notably both
`from_raw_value` and `into_raw_value` had their base-branch `#[verus_verify(external_body)]`
**removed** and are now genuinely verified. Remaining items, each addressed individually:
- `frame.proof.rs:13 lemma_frame_base_aligned` — `admit()`. Spec-phase placeholder; an
  elementary divisibility fact (`frame_raw·page_size % page_size == 0`). Proving-phase obligation.
- `frame.proof.rs:22 lemma_aligned_div_mul` — `admit()`. Spec-phase placeholder
  (`page-aligned ⇒ (addr/page)·page == addr`). Proving-phase obligation.
- `frame.spec.rs:20 assume_specification` (PhysicalAddress::from_raw_value) — see #7.
- `frame.rs:41 assume_specification` (arch `PAGE_SIZE`) — external arch constant, allowed.
These two `admit`s are the documented end-of-specification-phase state (proof.rs header
says the proving phase discharges them); they live in proof lemmas, not in any spec or
exec body, so the **specifications delivered by this phase are sound**.

**14. No specs weakened — PASS.** `git diff` vs base branch shows only strengthening:
`from_raw_value` gained `fa@ == raw_addr` and dropped `external_body`; `into_raw_value`
unchanged and dropped `external_body`; `from_frame_number`/`into_frame_number` gained full
contracts (were unspecced). No `requires` added that weakens callers; no `ensures` removed.

**15. Bug awareness — PASS.** `bugs.md` records BUG-001 (duplicate import, auto-fixed;
build re-confirmed clean). No fundamentally incorrect in-scope code: `from_frame_number`
body (`from_number` → always-aligned base → `from_address`) genuinely always succeeds;
`into_frame_number` delegates correctly under its precondition.

**16. Cross-module regression — PASS.** `make verify-kernel` (all modules) exits 0; all
verified modules pass. The `CHEATING_DETECTED` banner reflects pre-existing global
`admit/external_body` in `mm/phys`/`manager`/etc. — unchanged by this phase.

**17. Verification + build — PASS.** `make verify-kernel` exit 0 (frame module verified);
`./z build -- all-kernel` exit 0. Error count: 0.

### Fix Request
None. All 17 checklist items PASS (several with tracked, evidence-backed notes). No spec
defect requires a code change in the specification phase.

Tracked obligations carried into the PROVING phase (not spec defects):
1. Discharge `lemma_frame_base_aligned` and `lemma_aligned_div_mul` (`frame.proof.rs`) —
   remove both `admit()`.
2. Remove the `<PhysicalAddress as Address>::from_raw_value` `assume_specification`
   (`frame.spec.rs`) once `phys`'s `Address` impl carries its own `#[verus_spec]`.

Verdict: **RESOLVED.**
