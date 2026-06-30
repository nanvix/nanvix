verus! {

// Alignment predicate over an abstract address value `v` (`self@`): the integer
// address is an exact multiple of the alignment's byte value. This is the
// declarative meaning callers attach to `is_aligned` — independent of how the
// alignment check is computed. `spec_align_value` is the existing spec companion
// of `Alignment`.
pub open spec fn spec_addr_is_aligned(v: int, align: Alignment) -> bool {
    v % crate::mm::spec_align_value(align) == 0
}

} // verus!
