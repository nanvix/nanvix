## Turn 1: Full checklist walkthrough — mm::virt::identity_map (specification)

### Progress
- Done (PASS, with evidence below): ALL 17 checklist items.
- Current: full sweep complete on turn 1; no FAIL items found.
- Remaining: none.

In-scope functions (per `caller_analysis.md` — "the three verification-order
target functions only"): `identity_map_page`, `ensure_pt`, `ensure_pte`.

Tools run this turn:
- `make verify-kernel` → exit 0, module `mm::virt::identity_map` verified.
- `make verify` (all crates) → every crate exit 0 (cross-module regression).
- `./z build -- all-kernel` → `[OK] Build complete.`
- `fn_coverage.py` (source vs verus exec file) → 14/14 matched, 0 missing/extra.
- `spec_drift.py check nanvix-phys-virt-identity-map` → 0 contract drift.
- Cheating scan (`verus-logs/cheating-detail.txt`): assume=0, trusted=0,
  admit=4, external_body=29 (25 in other modules; 3 in-scope shims + 4 arch
  `external_type_specification` in this module).

---

### Verification per item

**1. Every in-scope exec function has requires/ensures — PASS.**
All three target functions carry `#[verus_spec(... ensures ...)]`:
- `identity_map_page` (rs:689-697): `inv()` + guarded `maps`.
- `ensure_pt` (rs:516-520): `inv()`.
- `ensure_pte` (rs:601-609): `inv()` + `maps` on success.
None require a `requires`: alignment is enforced by the `PageAligned` type at the
call site (caller_analysis "Alignment precondition"), so no exec precondition is
owed.

**2. Caller coverage — PASS.**
Checked each expectation in `caller_analysis.md`:
- `identity_map_page` success ("page covering phys_addr is identity-mapped") →
  `Ok(_) => identity_map_view().initialized ==> identity_map_view().maps(phys_addr@)`.
  The `initialized` guard correctly encodes the required pre-init no-op success.
- Idempotence ("safe no-op on already-mapped page") → modeled by
  `Set::insert` in `spec_identity_map_page` + `lemma_map_idempotent`.
- `ensure_pte` success ("PTE present, identity-maps phys_addr") →
  `Ok(_) => maps(phys_addr as int)`.
- `ensure_pt` ("PDE present; returned usize internal") → callers "don't care"
  about PT address/PDE structure; PDE-presence is a structural fact deliberately
  kept out of the View (view_design Rejected Alt. #2/#3), so `inv()` is the
  correct caller-relevant contract.
- All-or-nothing on failure → captured by the unconditional `inv()` clause (no
  out-of-range/partial frame recorded). See item 6.

**3. View consistency — PASS.**
Specs reference View fields exactly as `view_design.md` prescribes:
`initialized` (pre-init guard) and `mapped` (via `maps()`), both spec'd in
`identity_map.spec.rs`. `inv()` is `ensures`'d unconditionally on all three
functions, so the well-formedness invariant is maintained across every path.

**4. No tautological ensures — PASS (justified, not waved).**
Both `ensure_pte` and `identity_map_page` contain `Err(_) => true`. This is the
flagged anti-pattern, so I scrutinized it directly: the meaningful error-path
guarantee is hoisted OUT of the match into the unconditional
`identity_map_view().inv()` clause (proved on the Err path too). The accessor is a
single uninterpreted `identity_map_view()` with no `old()` (documented limitation,
mirroring `mm::phys::phys_view()`), so "mapped unchanged on error" is not
expressible, and the only single-state error fact — `inv()` — is already stated
unconditionally. Putting `Err(_) => identity_map_view().inv()` would be a SUBSUMED
ensures (item 5). Therefore `true` is the correct minimal arm and the match adds
only the success fact. Not a defect.

**5. No subsumed ensures — PASS.**
`inv()` is the base guarantee (not derivable from anything else). The `Ok` arm
`maps(phys_addr)` is not derivable from `inv()`. The `Err(_) => true` arm adds
nothing redundant (see item 4). No clause is implied by the others.

**6. Error paths have meaningful ensures — PASS.**
The meaningful failure guarantee ("all-or-nothing: no partial/out-of-range frame
recorded") is the unconditional `identity_map_view().inv()`, which holds on the
Err path. This is the strongest single-state error fact available without an
`old()` view. Caller (`KernelFrame::new`) only propagates the error and builds no
handle; no stronger error fact (e.g. error-code classification) is a caller
expectation per caller_analysis ("callers don't care").

**7. No assume_specification for workspace-internal code — PASS.**
`assume=0` in this module's files. The four `external_type_specification`
registrations (`ExTableIndex`, `ExPageDirectoryEntry`, `ExPageTableEntry`,
`ExTable`) register `arch`-crate (external dependency) types, not workspace-
internal kernel code, and are sanctioned trust-boundary registrations.

**8. vstd searched before any assume_specification — PASS (N/A).**
No `assume_specification` is used in this module. Nothing to search/replace.

**9. Specs written for the caller — PASS.**
`identity_map_page`'s contract is phrased over `identity_map_view().maps(phys_addr@)`
— directly usable in caller proofs. (The live caller `KernelFrame::new` is itself
currently an `mm::phys` trust boundary, so consumption is deferred, but the spec
form is caller-ready.)

**10. Trait obligations satisfied — PASS.**
None of the three target functions participate in a trait impl (caller_analysis
"Trait Obligations: None"; the only impl is `Drop for Cr3Guard`, out of scope).

**11. Spec completeness (advisory) — PASS (advisory).**
Reviewed the abstraction for gaps: pre-init no-op (`initialized` guard),
idempotence (`Set::insert`), map-on-success (`maps`), monotone growth and
inv-preservation (proof lemmas) are all covered. The pre-init no-op is intentional
nondeterminism that matches the caller expectation ("callers must tolerate the
pre-init no-op"). No missing observable behavior.

**12. Loop invariants — PASS.**
The three in-scope functions contain no loops. The only loops in the file live in
OUT-OF-SCOPE functions (`init`, `sync_kernel_pdes`, `ensure_identity_mapped_range`),
which are non-`external_body` and ARE verified by Verus; `make verify-kernel`
returns exit 0, so Verus accepted them (it would reject any loop that needed but
lacked an invariant).

**13. No cheating on module's own functions — PASS (each violator addressed).**
`assume=0`, `trusted=0`. Per-function challenge:
- `identity_map_page` (external_body): reads `static KERNEL_PD_PADDR`, raw
  `Table` pointer ops, delegates to trusted shims — unmodellable in Verus.
  Explicitly sanctioned in `verus-ai-logs/tcb-allowed.md:209`.
- `ensure_pt` (external_body): reads/writes raw kernel PD, draws from interior-
  mutable `static PAGE_TABLE_ALLOCATOR`, builds `arch` PDE — unmodellable.
  Sanctioned `tcb-allowed.md:212`.
- `ensure_pte` (external_body): raw PT writes + inline-asm `invlpg` — unmodellable.
  Sanctioned `tcb-allowed.md:215`.
- `ExTableIndex/ExPageDirectoryEntry/ExPageTableEntry/ExTable`
  (external_type_specification): register foreign `arch` paging types (external
  dependency), same idiom as `ExLinkedList` in `mm::phys`.
- `lemma_map_idempotent`, `lemma_map_on_success`, `lemma_map_monotone`,
  `lemma_map_preserves_inv` (admit×4): proof bodies intentionally `admit()` for
  the SPECIFICATION phase per the workflow ("Bodies left as admit() during the
  specification phase; the proving phase discharges them" — proof.rs:16-17).
  These are scheduled obligations for the next phase, not unjustified cheats.
None are unjustified; `tcb-allowed.md` lists every external_body for this module.

**14. No specs weakened — PASS.**
`spec_drift.py check` against baseline `4a5a2f93` (specification-phase start):
"Functions with changes: 0; Contract drift: 0; Ensures removed: 0; Requires
added: 0 — ✅ No contract drift detected." The contracts are net-new (no prior
guarantee to weaken).

**15. Bug awareness — PASS.**
No fundamentally incorrect code found in the in-scope functions or their specs.
View/address arithmetic is consistent (`maps` divides by `spec_page_size()`;
`phys_addr@`/`phys_addr as int` are the byte address). No bug to record.

**16. Cross-module regression — PASS.**
`make verify` (bitmap, sys, nanvix-slab, bump-allocator, arch, kernel): every
crate "Exit code : 0". No verified module regressed.

**17. Verification + build — PASS.**
- `make verify-kernel`: `Exit code : 0`, "verifying module mm::virt::identity_map".
- `./z build -- all-kernel`: `[OK] Build complete.`
- Error count: 0.

---

### Fix Request
None. All 17 checklist items PASS with concrete tool evidence and source
citations. The two scrutiny-worthy items (`Err(_) => true`; in-scope
`external_body`/`admit`) were challenged individually and are correct/justified:
the Err arm's content is hoisted to an unconditional `inv()` (avoiding a subsumed
clause), and every trusted shim is an unmodellable-state boundary explicitly
sanctioned in `tcb-allowed.md`, with proof `admit()`s deferred to the proving
phase by design. Specification phase is RESOLVED.
