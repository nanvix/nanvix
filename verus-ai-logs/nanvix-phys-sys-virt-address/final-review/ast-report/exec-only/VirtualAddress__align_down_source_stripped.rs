    pub fn align_down(&self, align: Alignment) -> Self {
        VirtualAddress::new(mm::align_down(self.0, align))
    }
