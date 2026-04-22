    pub const fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            counter: 0,
            capacity,
        }
    }
