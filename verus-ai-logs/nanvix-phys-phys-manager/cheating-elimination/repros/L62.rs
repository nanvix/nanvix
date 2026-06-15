// Reproducer for VERUS-AI LIMITATION id=L62
//
// lemma_kernel_alloc_contiguous(pre, post, frames, count): ensures
//   frames.len() == count
//   && kernel_frames_contiguous(frames, count)
//   && post == pre.book_all(kernel_addr_set(frames))
//   && pre.all_free(kernel_addr_set(frames))
//   && post.wf().
//
// Region-level analogue of L61 for `frame::alloc_contiguous()`. The exec caller
// (`PhysMemoryManager::alloc_many_kernel_frames`) obtains a contiguous run from the free function
// `frame::alloc_contiguous()` (no `self`), wrapping each frame into a `KernelFrame`. `self.upool`
// is never mutated, so Verus sees `self@` unchanged. `post`, `frames` are otherwise-unconstrained
// parameters, so the region transition `post == pre.book_all(kernel_addr_set(frames))` (and the
// contiguity/length facts about `frames`, which come from the runtime allocator) are unprovable.
//
// Minimal shape: same free-parameter `post`/`frames` problem as L61, lifted to a frame set.
//
// Empirical evidence (in-tree, with the assume removed):
//   error: postcondition not satisfied
//     --> src/kernel/src/mm/phys/manager.proof.rs:58:9   (frames.len() == count)
//     --> src/kernel/src/mm/phys/manager.proof.rs:59:9   (kernel_frames_contiguous(frames, count))
//
// Verdict: VERUS_LIMITATION_SILENT_MODEL — free-function global mutation invisible to `self`.
// Pre-approved in verus-ai-logs/tcb-allowed.md (lemma_kernel_alloc_contiguous).
