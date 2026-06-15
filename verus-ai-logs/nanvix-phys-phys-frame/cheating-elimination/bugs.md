# Bugs / Blockers — `mm::phys` cheating elimination (phys-frame phase)

## Context-Dependent — manager.proof.rs §8 ghost-token attachment never built (manager.proof.rs:16, 35, 55, 159)

**What**: Four `admit()` lemmas in `manager.proof.rs`
(`lemma_manager_attached`, `lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`,
`lemma_user_bulk_err_restored`) cannot be discharged in this phase. They all depend on the
"§8 global-state / ghost-token attachment" that the specification phase explicitly deferred to
the proving phase but which was never implemented — and which **cannot** be implemented under
the cheating-elimination source-integrity rules (no `tracked` ghost state may be threaded
through exec signatures/structs; no exec logic/signature changes).

**Why (root cause)**: `PhysMemoryManager::view(&self)` is `closed` and equals `self.upool@`,
but per `verus-ai-logs/nanvix-phys-phys-manager/view_design.md` §8 the manager actually brokers
the **global** frame allocator (`frame::INSTANCE`), not state inside the struct. §8 specifies the
intended cross-view invariant:

> `self@ == phys_view().frames` whenever `phys_view().manager_ready`
> … the proof phase pins the closed mapping via the same ghost-token attachment `PhysModView`
> uses, with the cross-View invariant.

`phys_view()` (`mod.spec.rs`) and `Upool::view`/`PhysMemoryManager::view` are **both `uninterp`
spec functions with no axioms**, and `phys_view()` takes no arguments (an immutable ghost
constant), so it cannot represent the "before"/"after" of a mutable global partition. The only
sound way to relate the two evolving views across exec calls — and to model that
`frame::alloc()` / `frame::free()` mutate the global partition — is a `tracked` ghost token
threaded through the exec signatures of `frame::alloc`/`free`/`alloc_contiguous` and stored in
the manager/pool structs (the "§8 token machinery"). That is precisely the class of change the
cheating-elimination rules forbid, and writing the relation as a bare `axiom`/`assume_specification`
is human-approval-only (spec-design: external-bottom). Hence:

- `lemma_manager_attached` (`m@ == phys_view().frames`) is the attachment axiom itself —
  underivable without the token (or a human-approved axiom).
- `lemma_kernel_alloc_one` / `lemma_kernel_alloc_contiguous` assert
  `final(self)@ == old(self)@.alloc_one(addr)` / `book_all(set)`. These are **correct under the
  §8 attachment** (a global allocation moves a frame in `phys_view().frames == self@`), but the
  *implementation* allocates via the global free functions `frame::alloc[_contiguous]()` and
  never touches `self.upool`, so with the attachment absent `self@` is decoupled and the
  transition cannot be derived.
- `lemma_user_bulk_err_restored` (`m@ == pre` after `frames.clear()`) needs the partition to
  reflect that `clear()` → `UserFrame::Drop` → global `frame::free()` rolls the frames back. Verus
  does not model `Drop` side effects, and without the §8 token the pool view does not see the
  global frees. The strengthened loop invariant added to `alloc_many_user_frames` this phase in
  fact *proves* `self@ == g_old.book_all(<non-empty set>)` on the error path, which directly
  witnesses that the un-attached `self@` is **not** restored.

**Verification Failure**: with the four `admit()`s removed, `make verify-kernel MODULE=mm::phys`
fails the corresponding `ensures` (e.g. `lemma_manager_attached` post `m@ == phys_view().frames`;
`alloc_kernel_frame` post `final(self)@ == old(self)@.alloc_one(kf@)`). With the `admit()`s in
place verification passes (84–85 verified, 0 errors) but the cheating gate reports `admit=4`.

**How Verus helped**: formal verification surfaced that two independent uninterpreted views of
the same global state are silently assumed equal, and that the user-bulk error path's "restored"
postcondition is not witnessed by the model — neither would be caught by testing (the runtime
`frame::free()` does restore real hardware state; only the *ghost model* is incoherent) and both
are easy to miss in code review.

**Severity**: correctness (verification-completeness). Not a runtime safety bug — the executable
code is correct; the gap is in the ghost model / proof infrastructure.

**Suggested fix (human / proving-phase, outside this phase's rule envelope)**: implement the §8
ghost-token attachment — a `tracked` token (e.g. a `vstd` state-machine / `GhostToken`) over the
`frame::INSTANCE` and `PhysMemoryManager`/`Upool` singletons — threaded through the exec
signatures of `frame::alloc`/`free`/`alloc_contiguous`, so `self@ == phys_view().frames` becomes
a maintained invariant and the four transitions are derived from the global allocator's verified
postconditions. Alternatively, a human may sanction the attachment as an explicit external-bottom
`axiom` with a documented justification. Either route is a deliberate design decision reserved for
a human / the dedicated proving phase; it must not be improvised by weakening the (correct) §4/§8
manager contracts, which `bug-reporting` and `verus-constraints` both forbid.

**Cross-reference**: `verus-ai-logs/nanvix-phys-phys-manager/view_design.md` §8;
`verus-ai-logs/nanvix-phys-phys-manager/bugs.md` (OBS-2, OBS-3);
`verus-ai-logs/nanvix-phys-phys-frame/verification_todo.md`. Same deferral pattern as the
bump-allocator and phys-mod global-token attachments.
