pub(super) fn free_count() -> usize {
    let inner = instance();
    inner.bitmap.number_of_bits() - inner.bitmap.usage()
}
