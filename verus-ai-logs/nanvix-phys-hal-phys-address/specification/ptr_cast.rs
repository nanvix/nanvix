// Reproducer B: once the whole impl is wrapped (#[verus_verify]) to satisfy
// rule A, the sibling pointer method's `usize as *const u8` cast is rejected.
use vstd::prelude::*;

verus! {
pub trait Address: View<V = int> {
    fn into_raw_value(self) -> usize;
    fn as_ptr(&self) -> *const u8;
}
} // verus!

pub struct VirtualAddress(usize);

verus! {
impl View for VirtualAddress {
    type V = int;
    closed spec fn view(&self) -> int { self.0 as int }
}
} // verus!

#[verus_verify]
impl Address for VirtualAddress {
    fn into_raw_value(self) -> usize { self.0 }
    fn as_ptr(&self) -> *const u8 { self.0 as *const u8 }
}

fn main() {}
