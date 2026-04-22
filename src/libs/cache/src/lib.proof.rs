// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Cache - Proofs
//
// This file contains proof functions for invariant preservation of
// cache spec transitions.

verus! {

//==================================================================================================
// Helper Lemmas
//==================================================================================================

/// Pushing an element not already present onto a no-duplicate sequence preserves no_duplicates.
proof fn lemma_push_preserves_no_dup<K>(s: Seq<K>, elem: K)
    requires
        s.no_duplicates(),
        !s.contains(elem),
    ensures
        s.push(elem).no_duplicates(),
{
    broadcast use vstd::seq_lib::group_seq_properties;

    let pushed = s.push(elem);
    assert forall |i: int, j: int|
        0 <= i < pushed.len() && 0 <= j < pushed.len() && i != j
    implies
        pushed[i] != pushed[j]
    by {
        // Both indices within original: covered by s.no_duplicates().
        // One index at s.len() (the pushed element): s[other] != elem
        // because !s.contains(elem) and s.contains(s[other]).
    }
}

/// Filtering preserves no_duplicates.
proof fn lemma_filter_preserves_no_dup<K>(s: Seq<K>, pred: spec_fn(K) -> bool)
    requires
        s.no_duplicates(),
    ensures
        s.filter(pred).no_duplicates(),
    decreases s.len(),
{
    broadcast use vstd::seq_lib::group_seq_properties;
    reveal(Seq::filter);

    if s.len() > 0 {
        let rest = s.drop_last();
        let last = s.last();

        lemma_filter_preserves_no_dup(rest, pred);

        if pred(last) {
            assert(!rest.filter(pred).contains(last)) by {
                if rest.filter(pred).contains(last) {
                    rest.lemma_filter_contains_rev(pred, last);
                }
            };
            lemma_push_preserves_no_dup(rest.filter(pred), last);
        }
    }
}

/// filter(|k| k != key).to_set() equals to_set().remove(key).
proof fn lemma_filter_neq_to_set<K>(s: Seq<K>, key: K)
    ensures
        s.filter(|k: K| k != key).to_set() =~= s.to_set().remove(key),
{
    broadcast use vstd::seq_lib::group_seq_properties, vstd::set::group_set_axioms;

    let pred = |k: K| k != key;
    let filtered = s.filter(pred);

    assert forall |x: K|
        filtered.to_set().contains(x) implies s.to_set().remove(key).contains(x)
    by {
        s.lemma_filter_contains_rev(pred, x);
        let idx = choose |idx: int| 0 <= idx < filtered.len() && filtered[idx] == x;
        s.lemma_filter_pred(pred, idx);
    };

    assert forall |x: K|
        s.to_set().remove(key).contains(x) implies filtered.to_set().contains(x)
    by {
        let idx = choose |idx: int| 0 <= idx < s.len() && s[idx] == x;
        s.lemma_filter_contains(pred, idx);
    };
}

/// For a no-duplicate sequence containing key, filter(!=key) reduces length by exactly 1.
proof fn lemma_filter_neq_len<K>(s: Seq<K>, key: K)
    requires
        s.no_duplicates(),
        s.contains(key),
    ensures
        s.filter(|k: K| k != key).len() == s.len() - 1,
{
    broadcast use vstd::set::group_set_axioms, vstd::seq_lib::seq_to_set_is_finite;

    let pred = |k: K| k != key;
    let filtered = s.filter(pred);

    lemma_filter_preserves_no_dup(s, pred);
    lemma_filter_neq_to_set(s, key);

    s.unique_seq_to_set();
    filtered.unique_seq_to_set();
}

/// Filtering by != key on a sequence not containing key is identity.
proof fn lemma_filter_neq_absent<K>(s: Seq<K>, key: K)
    requires
        !s.contains(key),
    ensures
        s.filter(|k: K| k != key) =~= s,
    decreases s.len(),
{
    reveal(Seq::filter);
    if s.len() > 0 {
        lemma_filter_neq_absent(s.drop_last(), key);
    }
}

/// Subrange of a no-duplicate sequence is no-duplicate.
proof fn lemma_subrange_no_dup<K>(s: Seq<K>, start: int, stop: int)
    requires
        s.no_duplicates(),
        0 <= start <= stop <= s.len(),
    ensures
        s.subrange(start, stop).no_duplicates(),
{
    let sub = s.subrange(start, stop);
    assert forall |i: int, j: int|
        0 <= i < sub.len() && 0 <= j < sub.len() && i != j
    implies
        sub[i] != sub[j]
    by {
        // sub[i] == s[start + i], sub[j] == s[start + j],
        // and start + i != start + j since i != j.
    }
}

/// For a no-dup sequence: subrange(1, len).to_set() == to_set().remove(first).
proof fn lemma_drop_first_to_set<K>(s: Seq<K>)
    requires
        s.no_duplicates(),
        s.len() > 0,
    ensures
        s.subrange(1, s.len() as int).to_set() =~= s.to_set().remove(s[0]),
{
    broadcast use vstd::seq_lib::group_seq_properties, vstd::set::group_set_axioms;

    let sub = s.subrange(1, s.len() as int);

    assert forall |x: K|
        sub.to_set().contains(x) implies s.to_set().remove(s[0]).contains(x)
    by {
        vstd::seq_lib::lemma_seq_subrange_elements(s, 1int, s.len() as int, x);
    };

    assert forall |x: K|
        s.to_set().remove(s[0]).contains(x) implies sub.to_set().contains(x)
    by {
        let idx = choose |idx: int| 0 <= idx < s.len() && s[idx] == x;
        vstd::seq_lib::lemma_seq_subrange_elements(s, 1int, s.len() as int, x);
    };
}

//==================================================================================================
// Invariant Preservation Lemmas
//==================================================================================================

/// `spec_new` produces a well-formed cache view.
proof fn lemma_spec_new_inv<K, V>(capacity: nat)
    ensures
        CacheView::<K, V>::spec_new(capacity).inv(),
{
    broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
        vstd::seq_lib::seq_to_set_is_finite;

    let cv = CacheView::<K, V>::spec_new(capacity);
    assert(cv.contents.dom() =~= Set::<K>::empty());
    assert(cv.lru_order.to_set() =~= Set::<K>::empty());
}

/// `spec_get` preserves the invariant.
proof fn lemma_spec_get_inv<K, V>(cache: CacheView<K, V>, key: K)
    requires
        cache.inv(),
    ensures
        cache.spec_get(key).0.inv(),
{
    broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
        vstd::seq_lib::seq_to_set_is_finite;

    if cache.contents.dom().contains(key) {
        let pred = |k: K| k != key;
        let filtered = cache.lru_order.filter(pred);

        // 1. no_duplicates for move_to_mru
        lemma_filter_preserves_no_dup(cache.lru_order, pred);
        assert(!filtered.contains(key)) by {
            if filtered.contains(key) {
                let idx = choose |idx: int| 0 <= idx < filtered.len() && filtered[idx] == key;
                cache.lru_order.lemma_filter_pred(pred, idx);
            }
        };
        lemma_push_preserves_no_dup(filtered, key);

        // 2. to_set: mru.to_set() == contents.dom()
        lemma_filter_neq_to_set(cache.lru_order, key);
        filtered.lemma_push_to_set_commute(key);

        // 3. len
        lemma_filter_neq_len(cache.lru_order, key);
    }
}

/// `spec_put` preserves the invariant.
proof fn lemma_spec_put_inv<K, V>(cache: CacheView<K, V>, key: K, value: V)
    requires
        cache.inv(),
    ensures
        cache.spec_put(key, value).inv(),
{
    broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
        vstd::seq_lib::seq_to_set_is_finite;

    if cache.capacity == 0 {
        // Zero-capacity: no-op.
    } else if cache.contents.dom().contains(key) {
        // Overwrite existing key.
        let pred = |k: K| k != key;
        let filtered = cache.lru_order.filter(pred);

        // no_duplicates
        lemma_filter_preserves_no_dup(cache.lru_order, pred);
        assert(!filtered.contains(key)) by {
            if filtered.contains(key) {
                let idx = choose |idx: int| 0 <= idx < filtered.len() && filtered[idx] == key;
                cache.lru_order.lemma_filter_pred(pred, idx);
            }
        };
        lemma_push_preserves_no_dup(filtered, key);

        // to_set
        lemma_filter_neq_to_set(cache.lru_order, key);
        filtered.lemma_push_to_set_commute(key);
        assert(cache.contents.insert(key, value).dom() =~= cache.contents.dom());
        lemma_filter_neq_len(cache.lru_order, key);

    } else if cache.contents.dom().len() >= cache.capacity {
        // At capacity, new key: evict LRU victim.
        let sub = cache.lru_order.subrange(1, cache.lru_order.len() as int);
        let new_lru = sub.push(key);

        // sub.no_duplicates
        lemma_subrange_no_dup(cache.lru_order, 1, cache.lru_order.len() as int);

        // key not in sub
        assert(!sub.contains(key)) by {
            if sub.contains(key) {
                vstd::seq_lib::lemma_seq_subrange_elements(
                    cache.lru_order, 1int, cache.lru_order.len() as int, key,
                );
            }
        };

        // new_lru.no_duplicates
        lemma_push_preserves_no_dup(sub, key);

        // to_set: new_lru.to_set() == result.contents.dom()
        lemma_drop_first_to_set(cache.lru_order);
        sub.lemma_push_to_set_commute(key);

        // len
        new_lru.unique_seq_to_set();

    } else {
        // Below capacity, new key: insert directly.

        // no_duplicates
        lemma_push_preserves_no_dup(cache.lru_order, key);

        // to_set
        cache.lru_order.lemma_push_to_set_commute(key);
    }
}

/// `spec_remove` preserves the invariant.
proof fn lemma_spec_remove_inv<K, V>(cache: CacheView<K, V>, key: K)
    requires
        cache.inv(),
    ensures
        cache.spec_remove(key).inv(),
{
    broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
        vstd::seq_lib::seq_to_set_is_finite;

    if cache.contents.dom().contains(key) {
        let pred = |k: K| k != key;
        let filtered = cache.lru_order.filter(pred);

        // no_duplicates
        lemma_filter_preserves_no_dup(cache.lru_order, pred);

        // to_set
        lemma_filter_neq_to_set(cache.lru_order, key);

        // len
        lemma_filter_neq_len(cache.lru_order, key);
        filtered.unique_seq_to_set();
    }
}

/// `spec_clear` preserves the invariant.
proof fn lemma_spec_clear_inv<K, V>(cache: CacheView<K, V>)
    requires
        cache.inv(),
    ensures
        cache.spec_clear().inv(),
{
    broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
        vstd::seq_lib::seq_to_set_is_finite;

    let cv = cache.spec_clear();
    assert(cv.contents.dom() =~= Set::<K>::empty());
    assert(cv.lru_order.to_set() =~= Set::<K>::empty());
}

//==================================================================================================
// Cache::new Verification Lemma
//==================================================================================================

impl<K: Ord + Clone, V> Cache<K, V> {
    /// Proves that a freshly constructed Cache matches CacheView::spec_new.
    /// Called from Cache::new after removing external_body.
    proof fn lemma_new_view(result: &Self, capacity: usize)
        requires
            btreemap_view_spec(result.entries) == Map::<K, CacheEntry<V>>::empty(),
            result.counter == 0u64,
            result.capacity == capacity,
        ensures
            result@ == CacheView::<K, V>::spec_new(capacity as nat),
            result@.inv(),
    {
        broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
            vstd::seq_lib::seq_to_set_is_finite;

        reveal(<Cache<_, _> as View>::view);
        reveal(cache_contents_of);
        reveal(cache_lru_of);

        assert(result@.contents =~= Map::<K, V>::empty());
        assert(result@.lru_order == Seq::<K>::empty());
        assert(result@.lru_order.to_set() =~= Set::<K>::empty());
    }
}

//==================================================================================================
// Cache::clear Verification Lemma
//==================================================================================================

impl<K: Ord + Clone, V> Cache<K, V> {
    /// Proves that Cache::clear produces the correct spec_clear view.
    /// Called from Cache::clear after removing external_body.
    proof fn lemma_clear_view(new_self: &Self, old_capacity: usize)
        requires
            btreemap_view_spec(new_self.entries) == Map::<K, CacheEntry<V>>::empty(),
            new_self.capacity == old_capacity,
        ensures
            new_self@ == (CacheView::<K, V> {
                contents: cache_contents_of(new_self.entries),
                capacity: old_capacity as nat,
                lru_order: cache_lru_of(new_self.entries),
            }).spec_clear(),
            new_self@.inv(),
    {
        broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
            vstd::seq_lib::seq_to_set_is_finite;

        reveal(<Cache<_, _> as View>::view);
        reveal(cache_contents_of);
        reveal(cache_lru_of);

        assert(new_self@.contents =~= Map::<K, V>::empty());
        assert(new_self@.lru_order == Seq::<K>::empty());
        assert(new_self@.lru_order.to_set() =~= Set::<K>::empty());
    }
}

//==================================================================================================
// BTreeMap LRU Axiom
//==================================================================================================

// Axiom: removing a key from entries produces the old LRU ordering filtered to
// exclude that key. Sound because BTreeMap::remove doesn't change last_used
// counters of remaining entries, so their relative sort order is preserved.
// Trust boundary: documented in trust.md.
#[verifier::external_body]
proof fn axiom_cache_lru_of_remove<K, V>(
    old_entries: alloc::collections::BTreeMap<K, CacheEntry<V>>,
    new_entries: alloc::collections::BTreeMap<K, CacheEntry<V>>,
    key: K,
)
    requires
        btreemap_view_spec(new_entries) == btreemap_view_spec(old_entries).remove(key),
    ensures
        cache_lru_of(new_entries) == cache_lru_of(old_entries).filter(|k: K| k != key),
{}

//==================================================================================================
// Cache::remove Verification Lemma
//==================================================================================================

impl<K: Ord + Clone, V> Cache<K, V> {
    /// Proves that Cache::remove produces the correct spec_remove view.
    proof fn lemma_remove_view(
        new_self: &Self,
        key: K,
        old_entries: alloc::collections::BTreeMap<K, CacheEntry<V>>,
        old_capacity: usize,
    )
        requires
            btreemap_view_spec(new_self.entries) == btreemap_view_spec(old_entries).remove(key),
            new_self.capacity == old_capacity,
            // Old state was well-formed (inv fields expressed via abstraction helpers):
            cache_contents_of(old_entries).dom().len() <= old_capacity as nat,
            cache_lru_of(old_entries).no_duplicates(),
            cache_lru_of(old_entries).to_set() == cache_contents_of(old_entries).dom(),
            cache_lru_of(old_entries).len() == cache_contents_of(old_entries).dom().len(),
        ensures
            new_self@ == (CacheView::<K, V> {
                contents: cache_contents_of(old_entries),
                capacity: old_capacity as nat,
                lru_order: cache_lru_of(old_entries),
            }).spec_remove(key),
            new_self@.inv(),
    {
        broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
            vstd::seq_lib::seq_to_set_is_finite;

        reveal(<Cache<_, _> as View>::view);
        reveal(cache_contents_of);
        reveal(cache_lru_of);

        let old_view = CacheView::<K, V> {
            contents: cache_contents_of(old_entries),
            capacity: old_capacity as nat,
            lru_order: cache_lru_of(old_entries),
        };

        // Apply LRU axiom: cache_lru_of(new) == cache_lru_of(old).filter(|k| k != key)
        axiom_cache_lru_of_remove(old_entries, new_self.entries, key);

        if old_view.contents.dom().contains(key) {
            // Key was present — spec_remove returns modified view
            let pred = |k: K| k != key;
            let filtered = old_view.lru_order.filter(pred);

            // Prove inv on the new state
            lemma_filter_preserves_no_dup(old_view.lru_order, pred);
            lemma_filter_neq_to_set(old_view.lru_order, key);
            lemma_filter_neq_len(old_view.lru_order, key);
            filtered.unique_seq_to_set();
        } else {
            // Key absent — spec_remove returns old_view unchanged
            lemma_filter_neq_absent(old_view.lru_order, key);
        }
    }
}

//==================================================================================================
// Helper: filter-first equals drop-first for no-dup sequences
//==================================================================================================

/// For a no-dup sequence, filtering out s[0] equals dropping the first element.
proof fn lemma_filter_first_is_subrange<K>(s: Seq<K>)
    requires
        s.no_duplicates(),
        s.len() > 0,
    ensures
        s.filter(|k: K| k != s[0]) =~= s.subrange(1, s.len() as int),
    decreases s.len(),
{
    reveal(Seq::filter);
    broadcast use vstd::seq_lib::group_seq_properties;

    let first = s[0];
    let pred = |k: K| k != first;

    if s.len() == 1 {
        // Single element: filter removes it, subrange(1,1) is empty.
    } else {
        let rest = s.drop_last();
        let last = s.last();

        // rest has no duplicates (it's a prefix of s).
        assert(rest.no_duplicates()) by {
            assert forall |i: int, j: int|
                0 <= i < rest.len() && 0 <= j < rest.len() && i != j
            implies rest[i] != rest[j] by {
                assert(rest[i] == s[i]);
                assert(rest[j] == s[j]);
            }
        };

        // rest[0] == s[0]
        assert(rest[0] == first);

        // IH: rest.filter(pred) =~= rest.subrange(1, rest.len())
        lemma_filter_first_is_subrange(rest);

        // last != first (since s has no dups and indices differ).
        assert(last != first) by {
            assert(s[s.len() as int - 1] == last);
            assert(s[0] == first);
        };

        // filter(s, pred) = filter(rest, pred).push(last)  (since pred(last) = true)
        //                  = rest.subrange(1, rest.len()).push(last)
        //                  =~= s.subrange(1, s.len())
        assert(rest.subrange(1, rest.len() as int).push(last)
               =~= s.subrange(1, s.len() as int));
    }
}

//==================================================================================================
// Cache::evict Verification Lemma
//==================================================================================================

impl<K: Ord + Clone, V> Cache<K, V> {
    /// Proves that Cache::evict (after extracting find_lru_victim) produces
    /// the correct postconditions: the LRU victim is removed, lru_order drops
    /// its first element, capacity is preserved, and inv holds.
    proof fn lemma_evict_view(
        new_self: &Self,
        victim: K,
        old_entries: alloc::collections::BTreeMap<K, CacheEntry<V>>,
        old_capacity: usize,
    )
        requires
            // victim is the LRU key
            cache_lru_of(old_entries).len() > 0,
            victim == cache_lru_of(old_entries)[0],
            // entries after btreemap_remove
            btreemap_view_spec(new_self.entries) == btreemap_view_spec(old_entries).remove(victim),
            new_self.capacity == old_capacity,
            // Old state was well-formed (inv fields via abstraction helpers)
            cache_contents_of(old_entries).dom().len() <= old_capacity as nat,
            cache_lru_of(old_entries).no_duplicates(),
            cache_lru_of(old_entries).to_set() == cache_contents_of(old_entries).dom(),
            cache_lru_of(old_entries).len() == cache_contents_of(old_entries).dom().len(),
        ensures
            new_self@.contents == cache_contents_of(old_entries).remove(victim),
            !new_self@.contents.dom().contains(victim),
            new_self@.contents.dom().len() == cache_contents_of(old_entries).dom().len() - 1,
            new_self@.lru_order == cache_lru_of(old_entries).subrange(
                1, cache_lru_of(old_entries).len() as int,
            ),
            new_self@.capacity == old_capacity as nat,
            new_self@.inv(),
    {
        broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
            vstd::seq_lib::seq_to_set_is_finite;

        reveal(<Cache<_, _> as View>::view);
        reveal(cache_contents_of);
        reveal(cache_lru_of);

        let old_lru = cache_lru_of(old_entries);

        // Apply LRU axiom: removing victim from entries filters the LRU order.
        axiom_cache_lru_of_remove(old_entries, new_self.entries, victim);
        // Now: cache_lru_of(new_self.entries) == old_lru.filter(|k| k != victim)

        // Key insight: filter(|k| k != s[0]) == subrange(1, len) for no-dup sequences.
        lemma_filter_first_is_subrange(old_lru);

        // Invariant properties for the new state:
        // 1. no_duplicates for the subranged lru_order
        lemma_subrange_no_dup(old_lru, 1, old_lru.len() as int);

        // 2. to_set: subrange(1, len).to_set() == to_set().remove(s[0])
        lemma_drop_first_to_set(old_lru);

        // 3. contents: cache_contents_of after remove
        assert(new_self@.contents =~= cache_contents_of(old_entries).remove(victim));
    }
}

} // verus!

