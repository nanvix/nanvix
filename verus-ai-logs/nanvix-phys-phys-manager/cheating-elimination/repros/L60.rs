// Reproducer for VERUS-AI LIMITATION id=L60
//
// lemma_manager_attached: ensures `m@ == phys_view().frames`.
//
// `phys_view()` is a 0-argument `uninterp spec fn` (a logic constant) and the manager's
// `View::view()` is `self.upool@`, where `Upool::view` is also `uninterp`. There is NO in-module
// fact relating these two uninterpreted functions, so the equality is genuinely unprovable: it is
// an external-bottom §8 ghost-token attachment, established only by a token over the
// `frame::INSTANCE` / `PhysMemoryManager` / `Upool` singletons in the (not-yet-done) proving phase.
//
// Minimal shape (uninterpreted constant vs. uninterpreted view) Verus cannot relate:
//
//   uninterp spec fn g() -> int;          // ~ phys_view().frames
//   uninterp spec fn view(x: S) -> int;   // ~ self.upool@
//   proof fn lemma(x: S) ensures view(x) == g() { }   // ERROR: postcondition not satisfied
//
// Empirical evidence (in-tree, with the assume removed):
//   error: postcondition not satisfied
//     --> src/kernel/src/mm/phys/manager.proof.rs:25:9   (m@ == phys_view().frames)
//
// Verdict: VERUS_LIMITATION_SILENT_MODEL — external-bottom singleton/ghost-token attachment.
// Pre-approved in verus-ai-logs/tcb-allowed.md (lemma_manager_attached).
