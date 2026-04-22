    pub const fn new(capacity: usize) -> Self {
        let result = Self {
            entries: BTreeMap::new(),
            counter: 0,
            capacity,
        };
        proof! {
            Self::lemma_new_view(&result, capacity);
        }
        result
    }
