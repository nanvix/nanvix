// Macro: reveals vstd's opaque ordering axioms (obeys_cmp_spec and friends).
// Reveals are local to the calling function; a proof fn can't propagate them.
// Use verus_proof_expr! so the macro expands inside verus! blocks.
#[allow(unused_macros)]
macro_rules! reveal_cmp_laws {
    () => {
        verus_proof_expr! {
            {
                reveal(::vstd::laws_cmp::obeys_cmp_spec);
                reveal(::vstd::laws_cmp::obeys_cmp_partial_ord);
                reveal(::vstd::laws_cmp::obeys_cmp_ord);
                reveal(::vstd::laws_cmp::obeys_partial_cmp_spec_properties);
                reveal(::vstd::laws_eq::obeys_eq_spec_properties);
            }
        }
    };
}

verus! {

proof fn lemma_insert_replace_maintains_inv<T: Ord>(old_seq: Seq<T>, idx: int, value: T)
    requires
        spec_strictly_sorted(old_seq),
        0 <= idx < old_seq.len(),
        old_seq[idx].cmp_spec(&value) is Equal,
        obeys_cmp_spec::<T>(),
    ensures
        old_seq.remove(idx).insert(idx, value) =~= old_seq.update(idx, value),
        spec_strictly_sorted(old_seq.update(idx, value)),
        old_seq.update(idx, value).len() == old_seq.len(),
        spec_contains(old_seq.update(idx, value), value),
        old_seq.update(idx, value).contains(value),
        old_seq.contains(old_seq[idx]),
        forall|v: T| #![auto] old_seq.update(idx, value).contains(v)
            ==> (old_seq.contains(v) || v == value),
{
    reveal_cmp_laws!();
    let new_seq = old_seq.update(idx, value);

    assert forall|i: int, j: int|
        #![trigger new_seq[i], new_seq[j]]
        0 <= i < j < new_seq.len()
        implies new_seq[i].cmp_spec(&new_seq[j]) is Less
    by {
        if i == idx {
            assert(old_seq[idx].cmp_spec(&old_seq[j]) is Less);
        } else if j == idx {
            assert(old_seq[i].cmp_spec(&old_seq[idx]) is Less);
        } else {
            assert(old_seq[i].cmp_spec(&old_seq[j]) is Less);
        }
    }

    assert(new_seq[idx] == value);

    assert forall|v: T|
        new_seq.contains(v)
        implies (old_seq.contains(v) || v == value)
    by {
        let k = choose|k: int| 0 <= k < new_seq.len() && new_seq[k] == v;
        if k == idx {
            assert(v == value);
        } else {
            assert(new_seq[k] == old_seq[k]);
        }
    }
}

proof fn lemma_insert_new_maintains_inv<T: Ord>(old_seq: Seq<T>, idx: int, value: T)
    requires
        spec_strictly_sorted(old_seq),
        0 <= idx <= old_seq.len(),
        !spec_contains(old_seq, value),
        forall|k: int| #![auto] 0 <= k < idx ==> old_seq[k].cmp_spec(&value) is Less,
        forall|k: int| #![auto] idx <= k < old_seq.len() ==> value.cmp_spec(&old_seq[k]) is Less,
        obeys_cmp_spec::<T>(),
    ensures
        spec_strictly_sorted(old_seq.insert(idx, value)),
        old_seq.insert(idx, value).len() == old_seq.len() + 1,
        spec_contains(old_seq.insert(idx, value), value),
        old_seq.insert(idx, value).contains(value),
        forall|v: T| #![auto] old_seq.insert(idx, value).contains(v)
            ==> (old_seq.contains(v) || v == value),
{
    reveal_cmp_laws!();
    let new_seq = old_seq.insert(idx, value);

    assert forall|i: int, j: int|
        #![trigger new_seq[i], new_seq[j]]
        0 <= i < j < new_seq.len()
        implies new_seq[i].cmp_spec(&new_seq[j]) is Less
    by {
        if i < idx && j == idx {
            assert(old_seq[i].cmp_spec(&value) is Less);
        } else if i < idx && j > idx {
            assert(old_seq[i].cmp_spec(&old_seq[j - 1]) is Less);
        } else if i == idx && j > idx {
            assert(value.cmp_spec(&old_seq[j - 1]) is Less);
        } else if i > idx {
            assert(old_seq[i - 1].cmp_spec(&old_seq[j - 1]) is Less);
        }
    }

    assert(new_seq[idx] == value);

    assert forall|v: T|
        new_seq.contains(v)
        implies (old_seq.contains(v) || v == value)
    by {
        let k = choose|k: int| 0 <= k < new_seq.len() && new_seq[k] == v;
        if k == idx {
            assert(v == value);
        } else if k < idx {
            assert(new_seq[k] == old_seq[k]);
        } else {
            assert(new_seq[k] == old_seq[k - 1]);
        }
    }
}

proof fn lemma_remove_maintains_inv<T: Ord>(old_seq: Seq<T>, idx: int, value: T)
    requires
        spec_strictly_sorted(old_seq),
        0 <= idx < old_seq.len(),
        old_seq[idx].cmp_spec(&value) is Equal,
        obeys_cmp_spec::<T>(),
    ensures
        spec_strictly_sorted(old_seq.remove(idx)),
        old_seq.remove(idx).len() == old_seq.len() - 1,
        !spec_contains(old_seq.remove(idx), value),
        old_seq.contains(old_seq[idx]),
        forall|v: T| #![auto] old_seq.remove(idx).contains(v) ==> old_seq.contains(v),
        forall|v: T| #![auto] old_seq.contains(v) && v != old_seq[idx] ==> old_seq.remove(idx).contains(v),
{
    reveal_cmp_laws!();
    let rem = old_seq.remove(idx);

    assert forall|i: int, j: int|
        #![trigger rem[i], rem[j]]
        0 <= i < j < rem.len()
        implies rem[i].cmp_spec(&rem[j]) is Less
    by {
        let oi = if i < idx { i } else { i + 1 };
        let oj = if j < idx { j } else { j + 1 };
        assert(rem[i] == old_seq[oi]);
        assert(rem[j] == old_seq[oj]);
    }

    assert forall|i: int| #![auto]
        0 <= i < rem.len()
        implies !(rem[i].cmp_spec(&value) is Equal)
    by {
        let oi = if i < idx { i } else { i + 1 };
        assert(rem[i] == old_seq[oi]);
        if oi < idx {
            assert(old_seq[oi].cmp_spec(&old_seq[idx]) is Less);
        } else {
            assert(old_seq[idx].cmp_spec(&old_seq[oi]) is Less);
        }
    }

    assert forall|v: T|
        rem.contains(v) implies old_seq.contains(v)
    by {
        let k = choose|k: int| 0 <= k < rem.len() && rem[k] == v;
        if k < idx { assert(rem[k] == old_seq[k]); }
        else { assert(rem[k] == old_seq[k + 1]); }
    }

    assert forall|v: T|
        old_seq.contains(v) && v != old_seq[idx] implies rem.contains(v)
    by {
        let k = choose|k: int| 0 <= k < old_seq.len() && old_seq[k] == v;
        if k < idx { assert(rem[k] == v); }
        else if k > idx { assert(rem[k - 1] == v); }
        else { assert(false); }
    }
}

/// get: uniqueness — strict sorting means at most one Ord-equal element.
proof fn lemma_get_uniqueness<T: Ord>(seq: Seq<T>, index: int, value: T)
    requires
        spec_strictly_sorted(seq),
        obeys_cmp_spec::<T>(),
        0 <= index < seq.len(),
        seq[index].cmp_spec(&value) is Equal,
    ensures
        forall|j: int| #![auto] 0 <= j < seq.len() && j != index
            ==> !(seq[j].cmp_spec(&value) is Equal),
{
    reveal_cmp_laws!();
    assert forall|j: int| #![auto]
        0 <= j < seq.len() && j != index
        implies !(seq[j].cmp_spec(&value) is Equal)
    by {
        if j < index {
            assert(seq[j].cmp_spec(&seq[index]) is Less);
        } else {
            assert(seq[index].cmp_spec(&seq[j]) is Less);
        }
    }
}

/// remove_by: forward frame — non-removed elements preserved after Vec::remove.
proof fn lemma_remove_forward_frame<T: Ord>(old_seq: Seq<T>, index: int, removed: T)
    requires
        0 <= index < old_seq.len(),
        old_seq[index] == removed,
    ensures
        forall|v: T| #![auto]
            old_seq.contains(v) && v != removed
            ==> old_seq.remove(index).contains(v),
{
    assert forall|v: T| #![auto]
        old_seq.contains(v) && v != removed
        implies old_seq.remove(index).contains(v)
    by {
        let k = choose|k: int| 0 <= k < old_seq.len() && old_seq[k] == v;
        if k < index {
            assert(old_seq.remove(index)[k] == v);
        } else if k > index {
            assert(old_seq.remove(index)[k - 1] == v);
        } else {
            assert(false);
        }
    }
}

} // verus!
