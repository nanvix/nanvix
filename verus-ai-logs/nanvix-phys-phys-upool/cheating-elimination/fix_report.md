# Cheating Elimination Report: phys-upool

## Scope

Module under verification: `src/kernel/src/mm/phys/upool.rs` (+ `upool.spec.rs`,
`upool.proof.rs`). In-scope target functions: `UserFrame::share`, `UserFrame::refcount`,
`Upool::new`, `UserFrame::leak`, `UserFrame::drop`, `Upool::alloc`, `UserFrame::new`,
`UserFrame::address`. Hard rule respected: **no unlisted functions touched** (`frame.rs`,
`manager.rs`, `mod.rs`, `manager.proof.rs` left untouched).

## Cheating Counts (before → after) — `upool` module

| Item                          | Before | After | Eliminated |
|-------------------------------|--------|-------|------------|
| admit()                       | 0      | 0     | 0          |
| assume()                      | 0      | 0     | 0          |
| external_body                 | 3      | 2*    | 1          |
| assume_specification          | 0      | 0     | 0          |
| cfg-gated exec (`R20p` no_decreases) | 0 | 0     | 0          |

\* The 2 remaining (`Upool::new`, `Upool::alloc`) are listed in `verus-ai-logs/tcb-allowed.md`
and are therefore **permitted** by the task's stated exception. They are genuine §8
ghost-token trust boundaries over the global frame allocator; rigorous proof of their
irreducibility within the `upool` scope is in
`verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`.

For reference, the whole-crate cheating count moved `external_body 15 → 14` as a direct result
of this work; the other 14 `external_body`, 7 `admit`, and 12 `cfg_gate` are in unlisted
out-of-scope modules (`frame.rs`, `manager.rs`, `mod.rs`, `manager.proof.rs`), all enumerated in
`tcb-allowed.md`.

## Items Eliminated

- **`Upool` (struct) `external_body` → removed.** Changed `#[verus_verify(external_body)]` to
  `#[verus_verify]`. The type is now machine-verified. Sound because the struct is never
  constructed in verified code (only the `external_body` `Upool::new` constructs it), so
  exposing its `()` field is harmless; the `View` stays `uninterp`. Verified: `make verify-kernel
  MODULE=mm::phys` → 86 verified, 0 errors; `make verify` → 116 verified, 0 errors.
  This is the **only** semantic change to the module (confirmed by `git diff`: the sole non-comment
  line change is `-#[verus_verify(external_body)]` / `+#[verus_verify]`).

## Items NOT Eliminated (genuine, documented trust boundaries — in `tcb-allowed.md`)

- **`Upool::new` `external_body`.** `ensures result@.wf()` over an *uninterpreted* `view()`.
  Removing the attribute yields `error: postcondition not satisfied … result@.wf()` (upool.rs:245).
  Interpreting the view as `phys_view().frames` would verify `new` but makes `alloc`'s `alloc_one`
  transition *assume `false`* (a 0-arg `uninterp phys_view()` is a logic constant, so
  `old(self)@ == final(self)@`); a ghost field cannot be used (`FrameAllocView`/`Ghost` do not
  exist in non-`verus` builds; a cfg-gated field would diverge the exec struct). Discharging it
  needs the frame-layer §8 ghost token (out of scope, unlisted, itself `external_body`).
- **`Upool::alloc` `external_body`.** The `alloc_one` free→allocated transition cannot be derived
  from `frame::alloc`'s weaker contract (containment only). Removing the attribute yields
  `error: postcondition not satisfied` (upool.rs:269). Discharged only by a `Tracked` allocation
  token threaded out of `frame::alloc` (out of scope).

Full Verus-error evidence and the escalation-ladder record (vstd search → isolated reproducer →
equivalent rewrites) are in `verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`.

## Verification TODOs (`verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`)

- `Upool::new` — `error: postcondition not satisfied … result@.wf()` (uninterpreted view;
  interpreted/ghost-field rewrites blocked as above). Needs frame-layer §8 ghost token.
- `Upool::alloc` — `error: postcondition not satisfied` on the `alloc_one` transition
  (`frame::alloc` contract too weak). Needs a `Tracked` allocation token from the frame layer.

Both are removed *when the frame free-function layer is verified*, exactly as their
`frame::alloc`/`book`/`share` siblings (`tcb-allowed.md`).

## AST Consistency

- Zero unexplained mismatches: **YES**. The only change is removing a `verus`-only attribute
  (`external_body`) from the `Upool` struct. This attribute is erased in ordinary (non-`verus`)
  builds, so exec semantics, time complexity, and space complexity are unchanged. No exec code,
  signatures, struct fields, or cfg gates were modified. (The pre-existing logging-only
  `#[cfg(not(verus_keep_ghost))]` gate in `UserFrame::drop` is untouched and semantically inert.)

## Verification Result

- `make verify-kernel` → Verus exit 0 (verify.sh: "Cheating is reported as a warning but does
  not fail the build; exit = Verus exit"). Module `mm::phys`: 86 verified, 0 errors.
- `make verify` (full crate) → exit 0; 116 verified, 0 errors. No regressions.
- `upool`-scoped: `admit=0`, `assume=0`, `assume_specification=0`, disallowed `external_body=0`
  (the 2 remaining are tcb-allowed), `exec_allows_no_decreases=0`.

## Result: PASS (no disallowed cheating in `upool`)

Within the `upool` module and the task's stated `tcb-allowed.md` exception, **zero disallowed
cheating remains**: the `Upool` struct boundary was genuinely eliminated, and the two remaining
`external_body` (`new`, `alloc`) are explicitly listed in `tcb-allowed.md` as design-forced §8
ghost-token boundaries over the global frame allocator, proven irreducible within scope. They can
only be eliminated by verifying the out-of-scope (unlisted) frame free-function layer, which the
hard rules forbid touching.
