// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Cache - Proofs
//
// This file contains proof function stubs for invariant preservation
// of cache spec transitions. Bodies use admit() as placeholders for
// the specification phase; they will be filled during the proving phase.

verus! {

//==================================================================================================
// Invariant Preservation Lemmas
//==================================================================================================

/// `spec_new` produces a well-formed cache view.
proof fn lemma_spec_new_inv<K, V>(capacity: nat)
    ensures
        CacheView::<K, V>::spec_new(capacity).inv(),
{
    admit();
}

/// `spec_get` preserves the invariant.
proof fn lemma_spec_get_inv<K, V>(cache: CacheView<K, V>, key: K)
    requires
        cache.inv(),
    ensures
        cache.spec_get(key).0.inv(),
{
    admit();
}

/// `spec_put` preserves the invariant.
proof fn lemma_spec_put_inv<K, V>(cache: CacheView<K, V>, key: K, value: V)
    requires
        cache.inv(),
    ensures
        cache.spec_put(key, value).inv(),
{
    admit();
}

/// `spec_remove` preserves the invariant.
proof fn lemma_spec_remove_inv<K, V>(cache: CacheView<K, V>, key: K)
    requires
        cache.inv(),
    ensures
        cache.spec_remove(key).inv(),
{
    admit();
}

/// `spec_clear` preserves the invariant.
proof fn lemma_spec_clear_inv<K, V>(cache: CacheView<K, V>)
    requires
        cache.inv(),
    ensures
        cache.spec_clear().inv(),
{
    admit();
}

} // verus!

