verus! {

/// After `frame::init` succeeds, the frame allocator singleton is initialized and its
/// abstract partition is well formed.
///
/// `frame::init` is `external_body`, so this fact is asserted as a lemma over the global
/// `phys_view()` accessor rather than derived from the exec body. The proving phase will
/// discharge it against the frame-allocator initialization contract.
pub proof fn lemma_frame_initialized()
    ensures
        phys_view().initialized,
        phys_view().frames.wf(),
{
    admit();
}

/// After `PhysMemoryManager::init` succeeds, the manager layer is ready.
///
/// `PhysMemoryManager::init` is `external_body`, so this fact is asserted as a lemma over
/// the global `phys_view()` accessor. The proving phase will discharge it against the
/// manager initialization contract.
pub proof fn lemma_manager_ready()
    ensures
        phys_view().manager_ready,
{
    admit();
}

} // end verus!
