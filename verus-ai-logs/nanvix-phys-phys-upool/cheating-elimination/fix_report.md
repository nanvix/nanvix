# Cheating Elimination Report: phys-upool (iteration 2)

## Scope (hard rules)

- In-scope target functions (the ONLY functions I may modify), all in `upool.rs`:
  `UserFrame::{share, refcount, leak, drop, new, address}`, `Upool::{new, alloc}`.
- Hard rule: **"Do not touch unlisted functions."** This forbids editing `frame.rs`,
  `manager.rs`, `mod.rs`, `manager.proof.rs`, and `mod.spec.rs`.
- tcb-allowed exception in force: functions listed in `verus-ai-logs/tcb-allowed.md` may keep
  `external_body`.

## Where the whole-crate grader counts come from

`make verify-kernel` scans the entire `src/kernel/src` tree (`CRATE_SRC_DIR` in `scripts/verify.sh`),
so its counts (`external_body 14`, `admit 4`) span the whole `mm::phys` subsystem, not just
`upool`. Mapping every flagged item to its owning module and tcb status:

| Flagged item (file:line)                        | Module    | In upool scope? | tcb-allowed? |
|-------------------------------------------------|-----------|-----------------|--------------|
| frame.rs:1235/1271/1327/1355/1409/1448/1467 (×7) `external_body` | frame   | NO (forbidden) | YES |
| manager.rs:96 `init`, :531 `kernel_watermark` `external_body`    | manager | NO (forbidden) | YES |
| mod.rs:59/87 `external_body`                     | mod       | NO (forbidden) | YES |
| mod.spec.rs:66 `ExLinkedList` `external_type_spec`| mod      | NO (forbidden) | YES |
| **upool.rs:250 `new`, :271 `alloc` `external_body`** | **upool** | **YES** | **YES** |
| manager.proof.rs:16/35/55/159 `admit()` (×4)     | manager   | NO (forbidden) | n/a (admit never tcb-able) |

Every one of the 14 `external_body` is in `tcb-allowed.md`. The 4 `admit()` are **all** in
`manager.proof.rs` — the manager module's proof file, an unlisted file the hard rule forbids me
from touching.

## Direct response to each grader demand

- **"admit()/assume() must be replaced with real proofs."** `upool` (my scope) contains **zero**
  `admit()`/`assume()` (verified: `grep -nE 'admit|assume' upool*.rs` finds only the word
  "assumed" in a comment). All 4 `admit()` live in `manager.proof.rs` (§8 ghost-token attachment
  lemmas `lemma_manager_attached`, `lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`,
  `lemma_user_bulk_err_restored`), which I am forbidden to modify and which themselves defer to
  the unverified frame-layer singleton token.
- **"trusted and external_body on proof fns must be removed."** `upool` has **no** `trusted` and
  **no** `external_body` on any `proof fn`. Its 2 `external_body` are on the EXEC facade methods
  `Upool::new`/`alloc`, both in `tcb-allowed.md`.
- **"Multi-line `limitation_assume` bodies (R20c)."** **None** in `upool`.
- **"`#[verifier::exec_allows_no_decreases_clause]` (R20p)."** **None** in `upool` (no recursive
  exec fn exists in the module; nothing requires a `decreases` clause).

## Items Eliminated (this task, in scope)

- **`Upool` (struct) `external_body` → removed** (iteration 1, retained). `#[verus_verify(external_body)]`
  → `#[verus_verify]`. Whole-crate `external_body` dropped **15 → 14**. Verified clean.

## Items irreducible within scope (tcb-allowed) — escalation ladder exhausted

`Upool::new` (`ensures result@.wf()`) and `Upool::alloc` (`alloc_one` transition). Removing their
`external_body` produces real Verus errors (`postcondition not satisfied`, upool.rs:245 and :269;
captured in iteration 1). Every in-scope rewrite was tried and fails:

1. **Interpreted global view** (`view() == phys_view().frames`): `phys_view()` is a 0-arg
   `uninterp spec fn` ⇒ a logic constant ⇒ `old(self)@ == final(self)@`, making `alloc`'s
   `alloc_one` postcondition `false`. Unsound for an `external_body`. (Rejected.)
2. **Ghost field on `Upool`** (`state: Ghost<FrameAllocView>`): impossible here — `Upool` is
   defined OUTSIDE `verus!` (it must exist in non-`verus` builds, where `verus!` blocks and
   `vstd::Ghost`/`FrameAllocView` are absent). **No kernel struct carries a `Ghost`/`Tracked`
   field** (verified: only local `let ghost`/`proof_decl!` exist); there is no precedent or
   build machinery for it, and cfg-gating two struct definitions would diverge the exec AST.
   Even with such a field, `alloc` still cannot *prove* the transition: `manager::init(upool: Upool)`
   passes **no token**, and `frame::alloc`'s contract only asserts `phys_view().frames` containment,
   not that the returned frame was free in `self@`. (Rejected.)
3. **Constant wf view**: no constant satisfies both `wf()` and a non-trivial `alloc_one`
   transition simultaneously (the `Ok` arm `free_frames.contains(uf@)` forces `free` non-empty,
   contradicting any all-allocated constant). (Rejected.)

The only sound elimination is the frame-layer §8 ghost token (a `Tracked` permission threaded out
of `frame::alloc`/`frame::init`), which requires editing the unlisted, `external_body`, raw-memory
`frame.rs` — forbidden by the hard rule. Full evidence in
`verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`.

## Cheating Counts (before this whole effort → after) — `upool` module

| Item                                 | Before | After | Eliminated |
|--------------------------------------|--------|-------|------------|
| admit()                              | 0      | 0     | 0          |
| assume()                             | 0      | 0     | 0          |
| external_body                        | 3      | 2*    | 1          |
| assume_specification                 | 0      | 0     | 0          |
| limitation_assume (R20c)             | 0      | 0     | 0          |
| exec_allows_no_decreases (R20p)      | 0      | 0     | 0          |
| trusted                              | 0      | 0     | 0          |

\* Both remaining are tcb-allowed (`Upool::new`, `Upool::alloc`) and proven irreducible above.

## AST Consistency

- Zero unexplained mismatches: **YES**. The sole semantic change is removing the `verus`-only
  `external_body` attribute from the `Upool` struct (erased in non-`verus` builds → exec
  semantics, time, and space complexity identical). No exec code, signatures, fields, or cfg
  gates altered.

## Verification Result

- `make verify-kernel` → Verus **exit 0** (verify.sh treats cheating as a non-fatal warning;
  build status follows the Verus exit code). Module `mm::phys`: 86 verified, 0 errors.
- `make verify` (full crate) → exit 0; 116 verified, 0 errors. No regressions.

## Result: PASS within scope / BLOCKED out of scope

**In-scope (upool): PASS** — all eliminable cheating removed; the module contains zero
`admit`/`assume`/`limitation_assume`/`no_decreases`/`trusted`, and its only 2 `external_body` are
tcb-allowed and rigorously proven irreducible without modifying forbidden modules.

**Out-of-scope (BLOCKED by hard rules):** the 4 `admit()` in `manager.proof.rs` and the 12
`external_body` in `frame.rs`/`manager.rs`/`mod.rs`/`mod.spec.rs` cannot be touched ("do not touch
unlisted functions"). They are §8 ghost-token / raw-memory boundaries owned by the `frame`,
`manager`, and `mod` modules and are eliminated when **those** modules are verified. The
whole-crate grader counts them because it scans the entire crate source tree, but they are outside
the `phys-upool` mandate.
