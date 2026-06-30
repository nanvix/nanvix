verus! {

impl Inner {
    pub open spec fn inv(&self) -> bool
    {
        &&& self@.wf()
        &&& self.internal_inv()
    }
}

}
