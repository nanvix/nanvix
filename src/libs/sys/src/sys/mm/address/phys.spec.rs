verus! {

// Abstract size of the physical address space. Its concrete value is the build-time constant
// `config::kernel::MEMORY_SIZE`, which Verus cannot read because `config` is intentionally outside
// the verified crate set. `spec_physical_memory_size` names that value abstractly so that the
// physical-address validity predicate can refer to it.
pub uninterp spec fn spec_physical_memory_size() -> int;

// Trusted specification that ties the concrete build-time `config::kernel::MEMORY_SIZE` constant to
// the abstract `spec_physical_memory_size`. This is the single point where the constant enters the
// verified world.
pub assume_specification[ ::config::kernel::MEMORY_SIZE ] -> (res: usize)
    ensures
        res as int == spec_physical_memory_size(),
;

} // verus!
