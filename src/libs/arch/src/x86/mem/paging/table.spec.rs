// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Equivalent to the replaced expression because it performs the same volatile read and decode.
unsafe fn env_interaction_read_table_entry<E: TableEntry>(ptr: *const PteWord) -> Option<E> {
    E::from_raw(unsafe { ::core::ptr::read_volatile(ptr) })
}

// Equivalent to the replaced statement because it performs the same encoded volatile write.
unsafe fn env_interaction_write_table_entry<E: TableEntry>(ptr: *mut PteWord, entry: E) {
    unsafe {
        ::core::ptr::write_volatile(ptr, entry.raw());
    }
}
