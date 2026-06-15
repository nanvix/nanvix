verus! {

// The initialization facts about the global physical-memory subsystem are established directly
// by the `external_body` trust-boundary postconditions of the singleton-bringing-up functions:
//
//   * `frame::init` ensures `phys_view().initialized && phys_view().frames.wf()` on success, and
//   * `PhysMemoryManager::init` ensures `phys_view().manager_ready`.
//
// Callers (`mm::phys::init`) therefore obtain these facts from the calls themselves; no separate
// bridge lemma over the uninterpreted `phys_view()` accessor is needed.

} // end verus!
