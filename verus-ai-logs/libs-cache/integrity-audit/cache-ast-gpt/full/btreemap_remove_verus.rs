fn btreemap_remove<K: Ord, V>(m: &mut BTreeMap<K, V>, k: &K) -> Option<V> {
    m.remove(k)
}
