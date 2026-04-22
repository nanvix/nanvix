    fn evict(&mut self) {
        // VERUS REWRITE: extracted iterator chain into find_lru_victim
        if let Some(key) = Self::find_lru_victim(&self.entries) {
            // VERUS REWRITE: originally self.entries.remove(&key)
            btreemap_remove(&mut self.entries, &key);
            proof! {
                Self::lemma_evict_view(self, key, old(self).entries, old(self).capacity);
            }
        }
    }
