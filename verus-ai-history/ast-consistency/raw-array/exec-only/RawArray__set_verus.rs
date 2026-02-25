    pub fn set(&mut self, index: usize, value: T)
        requires
            old(self).in_bounds(index as int),
        ensures
            self@.len() == old(self)@.len(),
            self@[index as int] == value,
            forall|i: int| 0 <= i < self@.len() && i != index as int
                ==> self@[i] == old(self)@[i],
    {
        self.storage.get_mut()[index] = value;
    }
