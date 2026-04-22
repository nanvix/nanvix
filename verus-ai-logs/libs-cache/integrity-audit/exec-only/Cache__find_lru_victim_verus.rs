    fn find_lru_victim(entries: &BTreeMap<K, CacheEntry<V>>) -> Option<K> {
        // VERUS REWRITE: originally inlined in evict as iterator chain
        entries
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone())
    }
