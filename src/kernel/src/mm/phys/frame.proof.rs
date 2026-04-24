verus! {

impl View for Inner {
    type V = UpoolView;

    uninterp spec fn view(&self) -> UpoolView;
}

impl Inner {
    pub closed spec fn internal_inv(&self) -> bool
    {
        true
    }
}

}
