// Reproducer for VERUS-AI LIMITATION id=L61
//
// lemma_kernel_alloc_one(pre, post, addr): ensures
//   pre.free_frames.contains(addr) && post == pre.alloc_one(addr) && post.wf().
//
// This encodes the runtime effect of `frame::alloc()` on the *global* frame partition. The
// exec caller (`PhysMemoryManager::alloc_kernel_frame`) allocates via the free function
// `frame::alloc()` (which takes NO `self`), so `self.upool` is never mutated and Verus sees
// `self@` unchanged. `post`/`pre`/`addr` are otherwise-unconstrained parameters supplied by the
// caller from `frame::alloc()`'s `phys_view().frames`-level postcondition; there is no in-module
// fact forcing `post == pre.alloc_one(addr)`, so it is genuinely unprovable.
//
// Minimal shape (free parameter `post` cannot be pinned to a function of `pre`):
//
//   proof fn lemma(pre: V, post: V, addr: int)
//       ensures post == pre.alloc_one(addr) { }   // ERROR: postcondition not satisfied
//
// Empirical evidence (in-tree, with the assume removed):
//   error: postcondition not satisfied
//     --> src/kernel/src/mm/phys/manager.proof.rs:41:9   (pre.free_frames.contains(addr))
//     --> src/kernel/src/mm/phys/manager.proof.rs:42:9   (post == pre.alloc_one(addr))
//
// Verdict: VERUS_LIMITATION_SILENT_MODEL — free-function global mutation invisible to `self`.
// Pre-approved in verus-ai-logs/tcb-allowed.md (lemma_kernel_alloc_one).
