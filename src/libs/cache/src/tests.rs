// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn get_returns_none_on_miss() {
    let mut cache: Cache<&str, i32> = Cache::new(4);
    assert!(cache.get(&"missing").is_none());
}

#[test]
fn put_then_get() {
    let mut cache: Cache<&str, i32> = Cache::new(4);
    cache.put("a", 1);
    assert_eq!(*cache.get(&"a").unwrap(), 1);
}

#[test]
fn put_overwrites_existing_key() {
    let mut cache: Cache<&str, i32> = Cache::new(4);
    cache.put("a", 1);
    cache.put("a", 2);
    assert_eq!(*cache.get(&"a").unwrap(), 2);
}

#[test]
fn evicts_lru_entry_when_full() {
    let mut cache: Cache<&str, i32> = Cache::new(2);
    cache.put("a", 1);
    cache.put("b", 2);
    // "a" is LRU; inserting "c" should evict it.
    cache.put("c", 3);
    assert!(cache.get(&"a").is_none());
    assert_eq!(*cache.get(&"b").unwrap(), 2);
    assert_eq!(*cache.get(&"c").unwrap(), 3);
}

#[test]
fn get_refreshes_lru_order() {
    let mut cache: Cache<&str, i32> = Cache::new(2);
    cache.put("a", 1);
    cache.put("b", 2);
    // Touch "a" so "b" becomes LRU.
    let _ = cache.get(&"a");
    cache.put("c", 3);
    assert_eq!(*cache.get(&"a").unwrap(), 1);
    assert!(cache.get(&"b").is_none());
    assert_eq!(*cache.get(&"c").unwrap(), 3);
}

#[test]
fn remove_deletes_key() {
    let mut cache: Cache<&str, i32> = Cache::new(4);
    cache.put("a", 1);
    cache.remove(&"a");
    assert!(cache.get(&"a").is_none());
}

#[test]
fn remove_nonexistent_key_is_noop() {
    let mut cache: Cache<&str, i32> = Cache::new(4);
    cache.remove(&"ghost");
    assert!(cache.get(&"ghost").is_none());
}

#[test]
fn clear_removes_all_entries() {
    let mut cache: Cache<&str, i32> = Cache::new(4);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.clear();
    assert!(cache.get(&"a").is_none());
    assert!(cache.get(&"b").is_none());
}

#[test]
fn capacity_one() {
    let mut cache: Cache<&str, i32> = Cache::new(1);
    cache.put("a", 1);
    cache.put("b", 2);
    assert!(cache.get(&"a").is_none());
    assert_eq!(*cache.get(&"b").unwrap(), 2);
}

#[test]
fn overwrite_does_not_evict() {
    let mut cache: Cache<&str, i32> = Cache::new(2);
    cache.put("a", 1);
    cache.put("b", 2);
    // Overwrite "a" — should not trigger eviction.
    cache.put("a", 10);
    assert_eq!(*cache.get(&"a").unwrap(), 10);
    assert_eq!(*cache.get(&"b").unwrap(), 2);
}
