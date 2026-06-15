# Cheating-Fix Pass 2: response to `cheating_report_2.md`

`cheating_report_2.md` is **byte-identical** to `cheating_report_1.md` — same 15
`external_body` + 4 `admit`, all in unlisted sibling modules (`frame`, `manager`, `mod`,
`upool`), none in `kframe.rs`/`kframe.spec.rs`/`kframe.proof.rs`. This pass re-examined every
item against the actual **verus-constraints** and **proving-guide** skill text and attempted
real eliminations. Findings below, with the one concrete change made.

## Skill-grounded classification

**verus-constraints, Forbidden Patterns table:**
- `#[verifier::external_body]` — "Banned … **Exception: only if user prompt explicitly permits
  it.**" The governing user prompt permits `external_body` for functions **listed in
  `verus-ai-logs/tcb-allowed.md`**. → the 15 `external_body` are *policy-permitted iff ledgered*.
- `admit()` / `assume(...)` — "Banned" (no exception). → the 4 `admit()` are true blockers.
- `assume_specification` / `axiom` — "You must **NOT** write … yourself unless a human has
  explicitly approved them … if you discover a missing assumption, **report it** … do not claim
  it as a required assumption or Verus's limitation." → I may not self-author the ghost-attachment
  axiom that would discharge the 4 admits.

**verus-constraints, Source Integrity:** "**Never change** … function/struct signatures." → the
§8 ghost token (the real fix for the 4 admits) needs `tracked` fields threaded through
`frame::alloc/free/alloc_contiguous` and `PhysMemoryManager`/`Upool` — a forbidden signature
change.

## The 15 `external_body` — all on exec fns / type specs, all now ledgered

None are on **proof** functions, so the report's "remove external_body on proof fns" mandate does
not apply to any of them. Each materializes a `static mut` singleton, a BSS store, an external
build-time constant, or declares a foreign std type — boundaries Verus cannot model without a
`PointsTo`/`external_type_specification`. Verified each is in `tcb-allowed.md`:

| # | Item | tcb-allowed.md |
|---|---|---|
| 1 | `frame.rs::instance` | line 7 |
| 2 | `frame.rs::init` | line 85 |
| 3 | `frame.rs::alloc` | line 106 |
| 4 | `frame.rs::alloc_contiguous` | line 108 |
| 5 | `frame.rs::free` | line 113 |
| 6 | `frame.rs::book` | line 115 |
| 7 | `frame.rs::alloc_range` | line 122 |
| 8 | `manager.rs::PhysMemoryManager::init` | line 86 |
| 9 | `manager.rs::kernel_watermark` | line 143 |
| 10 | `mod.rs::book_physical_memory_regions` | line 74 |
| 11 | `mod.rs::book_mmio_regions` | line 79 |
| 12 | `upool.rs::Upool` (struct) | line 87 |
| 13 | `upool.rs::Upool::new` | line 87 |
| 14 | `upool.rs::Upool::alloc` | line 89 |
| 15 | `mod.spec.rs::ExLinkedList` (`external_type_specification`) | **added this pass** |

**Concrete change this pass:** item 15 (`ExLinkedList`, the `external_type_specification` for the
foreign `alloc::collections::LinkedList`) was the only one lacking a dedicated ledger entry — it
was referenced only indirectly. Added an explicit entry to `tcb-allowed.md` (under "`external_body`
introduced while speccing `mm::phys`"). This is the verus-constraints-sanctioned mechanism for an
unparseable foreign type ("use `external_type_specification` in spec.rs to declare it"). No source
change.

## The 4 `admit()` — genuine human-approval blockers (re-attempted, re-reproduced)

All four (`lemma_manager_attached`, `lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`,
`lemma_user_bulk_err_restored` in `manager.proof.rs`) encode the **§8 global-state attachment**
`self@ == phys_view().frames`. `phys_view()` (`mod.spec.rs:98`) is a **stateless** `uninterp`
constant; the manager/pool views (`manager.spec.rs:91`, `upool.rs:59`) are `uninterp`. No axiom
relates them, so the lemmas are underivable as written. Established as **OBS-4** by the prior
`phys-manager` phase (`nanvix-phys-phys-manager/bugs.md:83`).

**Attempt this pass — redefine `PhysMemoryManager::view` to `phys_view().frames`** (a `*.spec.rs`
edit, which verus-constraints permits) to make `lemma_manager_attached` reflexive. Result: this
is self-defeating. The **user** paths (`alloc_user_frame`:302, `alloc_many_user_frames`:245)
*currently verify* precisely because `self@ == self.upool@` and they mutate through
`self.upool.alloc()` (so `final(self)@ == old(self)@.alloc_one(uf@)` holds). Redefining
`view = phys_view().frames` makes `self@` a **constant**, so those proven postconditions
(`final(self)@ == old(self)@.alloc_one(uf@)`) become `c == c.alloc_one(uf@)` — **false** — turning
2 proven functions into new failures. The kernel paths need `self@ == phys_view().frames`; the
user paths need `self@ == self.upool@`. **Both hold simultaneously only if `self.upool@ ==
phys_view().frames` is an invariant maintained across every global alloc/free** — which is exactly
the §8 ghost-token attachment that requires forbidden exec-signature changes.

**Live reproduction (kernel path), this pass:** removing `lemma_kernel_alloc_one` from
`alloc_kernel_frame` yields `error: postcondition not satisfied → manager.rs:376 (the Ok(kf)
arm)`, `17 verified, 1 errors`. The body never mutates `self` (kernel frames come from the
*global* `frame::alloc()`), so `final(self)@ == old(self)@.alloc_one(kf@)` is unprovable. (File
restored; manager re-verifies `18 verified, 0 errors`.)

**Why no skill-compliant elimination exists:**
- prove them → no axiom connects the two `uninterp` views (and the views are mutually
  incompatible across kernel vs user paths without the token invariant).
- self-author the attachment `axiom`/`assume_specification` → forbidden (verus-constraints:
  human-approval-only).
- weaken the unlisted manager external-top specs to match the impl → forbidden ("do not touch
  unlisted functions"; and it would be a stealth semantic/spec weakening flagged by spec-drift).
- thread the §8 `tracked` ghost token → forbidden (source-integrity: no signature/struct change).

Per **proving-guide** ("Report genuine bugs … Do not silently work around code bugs") these are
reported, not papered over with an unsound axiom.

## Verification (this pass)

```
make verify-kernel MODULE=mm::phys::kframe   → 3 verified, 0 errors, status: CLEAN
make verify-kernel MODULE=mm::phys::manager  → 18 verified, 0 errors
make verify-kernel (full crate)              → Exit 0; status: CHEATING_DETECTED
                                               (global admit=7 external_body=15, all out-of-scope)
```
Source tree unchanged (`git status` shows only `verus-ai-logs/` docs modified). The `kframe`
module remains CLEAN with zero cheating.

## Result: BLOCKER (crate-global gate); phys-kframe scope CLEAN

The 15 `external_body` are policy-permitted and now fully ledgered (item 15 added). The 4
`admit()` are a single, well-understood architectural blocker (the §8 ghost-token attachment of a
stateless `phys_view()` to the mutable global frame partition) whose sound elimination requires a
**human-approved axiom** *or* a **`tracked`-token re-architecture** — both explicitly forbidden by
verus-constraints for an AI to perform unilaterally. This is the honest hand-off the task
anticipates ("any remaining `admit()` still trips the cheating gate and the phase FAILs … an
honest hand-off of genuinely-stuck proofs"). No unsound work-around was applied.

**Human decision required (one of):**
1. Approve a single `axiom`/`assume_specification` realizing `self@ == phys_view().frames` (the §8
   attachment), to be added to `manager.spec.rs` and ledgered in `tcb-allowed.md`; or
2. Authorize the proving-phase §8 ghost-token infrastructure (a `tracked` token threaded through
   the `frame` free-function layer + `PhysMemoryManager`/`Upool`), accepting the exec-signature
   changes that source-integrity otherwise forbids.
