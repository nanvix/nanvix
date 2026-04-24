verus! {

impl View for UserFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.addr@
    }
}

impl UserFrame {
    pub closed spec fn inv(&self) -> bool {
        self.addr.inv()
    }
}

} // verus!
