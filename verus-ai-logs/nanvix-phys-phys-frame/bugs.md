# Bugs — `src/kernel/src/mm/phys/frame.rs`

None found.

This proving pass did not uncover any code bugs (no overflow, off-by-one, missing
bounds check, or unchecked cast). The work was entirely proof-side:

- Restored 10 TCB-justified `external_body` attributes (the 8 `Inner::*` methods,
  `instance`, and `init`) that a prior strip pass had removed; these are required
  because Verus cannot translate the `static mut` singleton accessor and the
  `MaybeUninit` interior-mutability bodies. All are listed in
  `verus-ai-logs/tcb-allowed.md`.
- Discharged the `free_count` shim's deferred `admit()` with a new sound lemma
  (`lemma_free_count` in `frame.proof.rs`).

The 6 remaining `admit()`s on the mutating shims are **not bugs**: they are a
spec-architecture limitation (post-state contracts over a fixed, uninterpreted
`phys_view()` with no `old()`), documented in
`verus-ai-logs/nanvix-phys-phys-frame/verification-todo.md` and tracked in
`tcb-allowed.md`.
