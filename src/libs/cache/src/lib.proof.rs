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
        assert(filtered.contains(x));
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
        assert(sub.contains(x));
        vstd::seq_lib::lemma_seq_subrange_elements(s, 1int, s.len() as int, x);
        let idx = choose |idx: int| 1 <= idx < s.len() && s[idx] == x;
        assert(s.contains(x));
        assert(x != s[0]);
    };

    assert forall |x: K|
        s.to_set().remove(s[0]).contains(x) implies sub.to_set().contains(x)
    by {
        assert(s.contains(x) && x != s[0]);
        let idx = choose |idx: int| 0 <= idx < s.len() && s[idx] == x;
        assert(idx >= 1int);
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
        let result = cache.spec_get(key).0;
        let pred = |k: K| k != key;
        let filtered = cache.lru_order.filter(pred);
        let mru = cache.move_to_mru(key);

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
        let result = cache.spec_put(key, value);
        let pred = |k: K| k != key;
        let filtered = cache.lru_order.filter(pred);
        let mru = cache.move_to_mru(key);

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

        // len
        lemma_filter_neq_len(cache.lru_order, key);

    } else if cache.contents.dom().len() >= cache.capacity {
        // At capacity, new key: evict LRU victim.
        let result = cache.spec_put(key, value);
        let victim = cache.lru_order[0];
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
        let result = cache.spec_put(key, value);
        let new_lru = cache.lru_order.push(key);

        assert(!cache.lru_order.contains(key)) by {
            if cache.lru_order.contains(key) {
                assert(cache.lru_order.to_set().contains(key));
            }
        };

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
        let result = cache.spec_remove(key);
        let pred = |k: K| k != key;
        let filtered = cache.lru_order.filter(pred);

        // no_duplicates
        lemma_filter_preserves_no_dup(cache.lru_order, pred);

        // to_set
        lemma_filter_neq_to_set(cache.lru_order, key);

        // len
        assert(cache.lru_order.contains(key)) by {
            assert(cache.lru_order.to_set().contains(key));
        };
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

} // verus!

