# Cheating Elimination Report: phys-manager

## Cheating Counts (before → after)

Scope: the manager module's editable files only — `manager.rs`,
`manager.spec.rs`, `manager.proof.rs`.

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 4 | 4 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 2 | 2 | 0 (both TCB-allowed) |
| assume_specification | 3 | 3 | 0 (trusted std specs; gate-neutral) |
| cfg-gated exec | 9 | 9 | 0 (pre-approved logging gates) |

`make verify-kernel MODULE=mm::phys` → **42 verified, 0 errors** (unchanged
baseline). Full-crate `make verify` → all targets **exit 0** (cached, no
regressions). The cheating gate still reports `CHEATING_DETECTED` because of the
4 residual `admit()`s.

## Items Eliminated

None could be soundly eliminated within the manager's editable scope. The
escalation ladder (search vstd → isolated reproducer → equivalent rewrite) was
followed for each `admit()`; all four are the **same root cause** — the §8
global ghost-token attachment, which is realized in the out-of-scope, still
**unverified** `frame` free-function layer (`frame.rs` carries 8 of its own
`admit()`s). Concrete reproducer: removing all four `admit()`s and re-running
`make verify-kernel MODULE=mm::phys` produces exactly four
`postcondition not satisfied` errors at the lemma seams (`38 verified, 4
errors`) with no collateral; restoring them returns to `42 verified, 0 errors`.

`external_body` (`init`, `kernel_watermark`) are explicitly on
`verus-ai-logs/tcb-allowed.md` (singleton `static mut` write; build-time
`config::kernel::KERNEL_WATERMARK` constant from a non-Verus crate) — allowed,
not blockers. The three `assume_specification`s (`Result::and_then`,
`Result::inspect_err`, `Vec::capacity`) are trusted std-library contracts vstd
does not ship; they are not flagged by the cheating gate (`assume=0`) and were
established in the specification phase. The 9 cfg-gated exec lines guard
`error!`/`warn!` logging macros (unsupported constant type under Verus) — a
codebase-wide pre-approved deviation.

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-manager/verification_todo.md)

All four are genuinely stuck on the out-of-scope `frame` layer's ghost token:

- **`lemma_manager_attached`** (`manager.proof.rs:12`) — `m@ == phys_view().frames`
  fails (`postcondition not satisfied @ :14:9`). `m@ = m.upool@` is opaque
  (`Upool` is `external_body`); `phys_view()` is a parameter-free `uninterp`
  DO-NOT-MODIFY constant. No in-scope contract links them.
- **`lemma_kernel_alloc_one`** (`manager.proof.rs:27`) —
  `post == pre.alloc_one(addr)` fails (`:31:9`, `:32:9`). Source obtains the
  frame from the free function `frame::alloc()` (no `self`), so Verus proves
  `final(self)@ == old(self)@`, not the demanded transition. (The user path is
  fully proven because `Upool::alloc` is `&mut self`.)
- **`lemma_kernel_alloc_contiguous`** (`manager.proof.rs:40`) — same as above for
  the contiguous bulk path via free function `frame::alloc_contiguous` (`:49:9`,
  `:50:9`).
- **`lemma_user_bulk_err_restored`** (`manager.proof.rs:210`) — `m@ == pre` fails
  (`:214:9`). On error `frames.clear()` relies on `UserFrame::drop → frame::free`;
  Verus does not model `Drop` side effects and `frame::free` is a free function,
  so `self.upool@` cannot be reduced back to `pre`.

**Unblock prerequisite (all four):** verify the `frame` free-function layer first
so it exposes a tracked global partition token; the four lemmas then discharge
mechanically (value invariant / step lemmas / per-handle free). No spec was
weakened and no new `external_body`/`assume` was introduced.

## AST Consistency

- `python3 ast_consistency.py --base-ref verus-ai-prove manager.rs summary` →
  `Consistent: ✅ YES (matched=8 mismatched=0 missing=0 extra=0)`.
- Source files are byte-identical to the phase-start commit
  (`git diff a8d643993 -- manager.rs manager.spec.rs manager.proof.rs` is empty).
- Zero mismatches confirmed: **YES**

## Result: BLOCKER

Four `admit()`s remain in `manager.proof.rs`. They are genuine §8 ghost-token /
`Drop`-model proof gaps whose discharge requires verifying and token-instrumenting
the out-of-scope `frame` free-function layer (itself still 8 `admit()`s). They
cannot be eliminated within the manager module's editable scope without either an
unsound axiom (banned) or an out-of-scope exec-signature change to `frame.rs`.
Recorded honestly in `verification_todo.md` with reproducers; the cheating gate
therefore remains tripped.
