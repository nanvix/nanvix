// Reproducer A: Verus requires the ENTIRE trait impl to be verified to spec any
// single method. Annotating just one method errors.
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

impl Address for VirtualAddress {
    #[verus_spec(result =>
        ensures result as int == self@,
    )]
    fn into_raw_value(self) -> usize { self.0 }

    fn as_ptr(&self) -> *const u8 { self.0 as *const u8 }
}

fn main() {}
