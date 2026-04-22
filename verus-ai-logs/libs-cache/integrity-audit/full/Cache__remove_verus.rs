    pub fn remove(&mut self, key: &K) {
        // VERUS REWRITE: originally self.entries.remove(key);
        // Wrapper needed because BTreeMap::remove's full generic signature
        // (Borrow<Q>, Allocator) cannot be expressed with btreemap_view_spec.
        btreemap_remove(&mut self.entries, key);
        proof! {
            Self::lemma_remove_view(self, *key, old(self).entries, old(self).capacity);
        }
    }
