verus! {

// No module-local proof helpers are required: `init` discharges its
// postcondition directly from the dependency contracts of `frame::init`
// (initialized + well-formed partition) and `PhysMemoryManager::init`
// (manager ready), composed over the parameter-free `phys_view()` accessor.

} // end verus!
