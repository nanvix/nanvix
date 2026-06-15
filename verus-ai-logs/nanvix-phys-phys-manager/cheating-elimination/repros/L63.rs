// Reproducer for VERUS-AI LIMITATION id=L63
//
// lemma_user_bulk_err_restored(m, pre): ensures `m@ == pre`.
//
// On a mid-bulk failure, `alloc_many_user_frames` calls `Vec::clear()` on the partially-filled
// vector, which runs `Drop` on every already-taken frame, freeing it and restoring the global
// partition to its pre-call state `pre`. `Drop` side effects are NOT modeled in Verus exec
// semantics — the destructor calls `frame::free()` (a free function, no `self`), so Verus sees no
// state change to `self@` and cannot derive `m@ == pre`.
//
// Minimal shape (Drop side effect not modeled): a destructor that calls an external free function
// produces no Verus-visible postcondition relating `m@` to its prior value.
//
//   proof fn lemma(m: S, pre: V) ensures view(m) == pre { }   // ERROR: postcondition not satisfied
//
// Empirical evidence (in-tree, with the assume removed):
//   error: postcondition not satisfied
//     --> src/kernel/src/mm/phys/manager.proof.rs:165:9   (m@ == pre)
//
// Verdict: VERUS_LIMITATION_SILENT_MODEL — Drop/free-function side effect invisible to `self`.
// Pre-approved in verus-ai-logs/tcb-allowed.md (lemma_user_bulk_err_restored).
