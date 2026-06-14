## Turn 1: Full checklist walkthrough — proving phase, `mm::phys` (mod)

**Scope (from `verus-ai.toml` background + `verification-plan.json`):** the only
target functions for this proving unit are `init`, `book_physical_memory_regions`,
and `book_mmio_regions` in `src/kernel/src/mm/phys/mod.rs`, with their
`mod.spec.rs` / `mod.proof.rs` support files. Admit/`external_body` in `frame.rs`,
`manager.rs`, `manager.proof.rs`, `upool.rs`, `kframe.rs`, `hal/*`, `mm/virt/*`
belong to *other* proving targets and are out of scope here; they are recorded in
`tcb-allowed.md` as cross-module dependencies "eliminated when their module is
verified."

### Progress
- Done (PASS/FIXED, all verified with tool output):
  1. No specs weakened — PASS
  2. Zero remaining `admit()` — PASS (FIXED during proving)
  3. Zero `external_body` unless TCB-allowed — PASS
  4. Zero `assume`/`assume_specification` — PASS
  5. No cfg-gated exec code — PASS
  6. Cheating audit (counts + locations) — PASS (reported)
  7. Isolated reproducer for claimed Verus limitation — PASS
  8. Exec rewrites minimal & semantically equivalent — PASS (no exec rewrites)
  9. Cross-module regression (`make verify-kernel`) — PASS
  10. Verification + build (0 errors / 0 warnings) — PASS
- Current: n/a
- Remaining: none

### Verification

**1. No specs weakened.**
`git diff 19e519240^ HEAD -- mod.spec.rs` → **empty**: the specification file is
byte-identical to the pre-proving baseline. `git diff … mod.rs` shows the proving
phase changed **only the exec body** of `init` (removed two `proof!{…}` calls to
admit-lemmas); every `#[verus_spec]` `requires`/`ensures` contract on `init`,
`book_physical_memory_regions`, `book_mmio_regions` is unchanged. The proof was
*strengthened*, not weakened. PASS.

**2. Zero remaining `admit()`.**
`git diff … mod.proof.rs` shows the two admit-bearing lemmas (`lemma_frame_initialized`,
`lemma_manager_ready`, each `admit();`) were **deleted**; `mod.proof.rs` is now a
comment-only `verus!{}` block with no admit. `init` now discharges its postcondition
(`live()` + `all_reserved(...)` + the MMIO `forall`) directly from the callees'
contracts. No `admit` in any in-scope file (`grep` of cheating-detail confirms the
only `mm/phys/mod*` entries are the two `book_*` external_body and `ExLinkedList`).
The global `admit=34` are all in out-of-scope targets (`frame.rs`,
`manager.proof.rs`, `upool`, `hal/*`, `mm/virt/*`). PASS.

**3. Zero `external_body` unless in `tcb-allowed.md`.**
In-scope `external_body` per `cheating-detail.txt`:
- `mod.rs:73 book_physical_memory_regions` — **listed** in `tcb-allowed.md`
  ("iterates `alloc::collections::LinkedList`… Verus has no LinkedList model").
- `mod.rs:103 book_mmio_regions` — **listed** in `tcb-allowed.md` (same limitation).
- `mod.spec.rs:69 ExLinkedList` — `external_type_specification` registering the
  foreign `LinkedList` type so it may appear in spec signatures (external-bottom
  for a std/external type; no body verified, no contract asserted). Permitted.
`init` is body-verified (not in the external_body list). PASS.

**4. Zero `assume`/`assume_specification`.**
`grep -nE '\bassume\b|assume_specification'` over `mod.rs`/`mod.spec.rs`/`mod.proof.rs`
→ **no matches**. Global summary: `assume=0`. PASS.

**5. No cfg-gated exec code (branches, expressions, match arms).**
`cfg` occurrences in `mod.rs`: `#[cfg(feature="test")]` on `mod test;` (l.15) and
`pub fn test()` (l.195) — a test harness that is on the skip/coverage-excluded list,
not one of the three target functions; and `#[cfg(verus_keep_ghost)]` (l.36/40/42)
guarding the spec/proof `include!`s and the ghost `vstd` import — the standard
verus-ai ghost gating. None are cfg attributes on a branch / expression / match arm
*inside* a verified exec function. The bodies of `init`, `book_physical_memory_regions`,
`book_mmio_regions` contain no `cfg`. PASS.

**6. Cheating audit — exact counts + locations.**
In-scope (`mm/phys/mod*`): `admit=0`, `external_body=2`
(`book_physical_memory_regions` @ mod.rs:73, `book_mmio_regions` @ mod.rs:103 — both
TCB-allowed), `external_type_spec=1` (`ExLinkedList` @ mod.spec.rs:69 — allowed),
`assume=0`, cfg-gated exec branches/expressions/match-arms = 0.
Global kernel (context only, all out of scope): `assume=0 external_body=12 admit=34
trusted=0 no_decreases=0 cfg_gate=15`. None of the global admits/external_body fall
inside the phys-mod target. PASS.

**7. Isolated reproducer for the claimed Verus limitation.**
Claimed limitation (`bugs.md`): the two `book_*` helpers iterate
`alloc::collections::LinkedList` via `for region in list.iter()`; verifying the loop
requires implementing vstd's `View`/`ForLoopGhostIterator` for the foreign
`LinkedList`, which Rust's **orphan rule (E0117)** forbids from the kernel crate.
I reproduced the *specific construct* in isolation (foreign trait on a foreign type,
structurally identical to `impl vstd::view::View for alloc::collections::LinkedList`):
```
impl std::fmt::Display for std::collections::LinkedList<u8> { ... }
```
→ `error[E0117]: only traits defined in the current crate can be implemented for
types defined outside of the crate … LinkedList is not defined in the current crate`.
This confirms the limitation is a determinate Rust language rule, not an unverified
hand-wave. PASS.

**8. Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`).**
`grep -rn "VERUS REWRITE"` over the in-scope files → **no matches**. The proving diff
introduced no exec rewrite; it only *removed* ghost `proof!{}` blocks from `init`,
which has zero effect on compiled (exec) semantics. PASS.

**9. Cross-module regression — `make verify-kernel` (whole crate).**
Exit code 0; every module verified including `mm::phys::{frame,kframe,manager,upool}`,
`mm::virt::*`, `hal::*` ("note: verifying module …" for all, 0 errors). No regression.
PASS.

**10. Verification + build — 0 errors / 0 warnings.**
- `make verify-kernel MODULE=mm::phys` → Exit code 0, 45 verified, 0 errors.
- `make verify-kernel` (full) → Exit code 0, 0 errors.
- `./z build -- all` → `[OK] Build complete.` No compiler errors, no compiler
  warnings (the only "Warning" emitted is the benign environmental
  `Sysroot directory … not found; skipping symlink update`, unrelated to source).
PASS.

### Fix Request
None. All ten checklist items are PASS (item 2 was FIXED earlier in the proving phase
by deleting the two `admit()` lemmas and body-verifying `init`; verified here against
the actual diff and live tool output). Marking the unit RESOLVED.
