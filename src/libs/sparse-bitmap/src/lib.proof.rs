verus! {

//==================================================================================================
// Helper Lemmas for lifted_set_bits
//==================================================================================================

/// Every element of `lifted_set_bits(chunks, idx)` is covered by some chunk at index >= `idx`.
proof fn lemma_lifted_set_bits_subset(chunks: Seq<Chunk>, idx: int)
    requires
        idx >= 0,
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
    ensures
        forall|g: int| lifted_set_bits(chunks, idx).contains(g) ==>
            exists|k: int|
                #![trigger chunks[k]]
                idx <= k < chunks.len()
                && chunks[k].spec_covers(g),
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
    } else {
        lemma_lifted_set_bits_subset(chunks, idx + 1);
        assert forall|g: int| lifted_set_bits(chunks, idx).contains(g) implies
            exists|k: int|
                #![trigger chunks[k]]
                idx <= k < chunks.len()
                && chunks[k].spec_covers(g)
        by {
            let bv = chunks[idx].bitmap@;
            let offset = chunks[idx].offset as int;
            if bv.set_bits.contains(g - offset) {
                assert(chunks[idx].bitmap@.wf());
                assert(chunks[idx].spec_covers(g));
            } else {
                assert(lifted_set_bits(chunks, idx + 1).contains(g));
            }
        }
    }
}

/// `lifted_set_bits(chunks, idx)` is finite.
proof fn lemma_lifted_set_bits_finite(chunks: Seq<Chunk>, idx: int)
    requires
        idx >= 0,
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.set_bits.finite(),
    ensures
        lifted_set_bits(chunks, idx).finite(),
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
    } else {
        lemma_lifted_set_bits_finite(chunks, idx + 1);

        let offset = chunks[idx].offset as int;
        let bv = chunks[idx].bitmap@;
        let shifted = Set::<int>::new(|g: int| bv.set_bits.contains(g - offset));
        let rest = lifted_set_bits(chunks, idx + 1);

        // shifted ⊆ [offset, offset + num_bits), which is finite
        let range = vstd::set_lib::set_int_range(offset, offset + bv.num_bits);
        vstd::set_lib::lemma_int_range(offset, offset + bv.num_bits);
        assert(shifted.subset_of(range)) by {
            assert forall|g: int| #![auto] shifted.contains(g) implies range.contains(g) by {
                assert(chunks[idx].bitmap@.wf());
            }
        };
        vstd::set_lib::lemma_set_subset_finite(range, shifted);

        // Union of two finite sets is finite
        vstd::set_lib::lemma_set_union_finite_iff::<int>(shifted, rest);
    }
}

/// If chunk `ci`'s bitmap contains `local`, then `lifted_set_bits`
/// (from index `idx` onwards) contains the globally-shifted index.
proof fn lemma_lifted_set_bits_contains_chunk(
    chunks: Seq<Chunk>, idx: int, ci: int, local: int,
)
    requires
        0 <= idx,
        idx <= ci,
        ci < chunks.len(),
        chunks[ci].bitmap@.set_bits.contains(local),
    ensures
        lifted_set_bits(chunks, idx).contains(chunks[ci].offset as int + local),
    decreases ci - idx
{
    let g = chunks[ci].offset as int + local;
    if idx == ci {
        let offset = chunks[idx].offset as int;
        let bv = chunks[idx].bitmap@;
        let shifted = Set::<int>::new(|g: int| bv.set_bits.contains(g - offset));
        assert(shifted.contains(g));
    } else {
        lemma_lifted_set_bits_contains_chunk(chunks, idx + 1, ci, local);
    }
}

/// If chunk `ci`'s bitmap does NOT contain `local`, and no other chunk
/// has a set bit at the corresponding global index, then
/// `lifted_set_bits` does not contain the global index.
proof fn lemma_lifted_set_bits_not_contains_chunk(
    chunks: Seq<Chunk>, idx: int, ci: int, local: int,
)
    requires
        0 <= idx,
        0 <= ci < chunks.len(),
        !chunks[ci].bitmap@.set_bits.contains(local),
        // chunks are sorted and non-overlapping
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        // chunk ranges don't overflow
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits <= usize::MAX as int,
        // all bitmaps are valid
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        // local is in range for chunk ci
        0 <= local < chunks[ci].bitmap@.num_bits,
    ensures
        !lifted_set_bits(chunks, idx).contains(chunks[ci].offset as int + local),
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
    } else {
        lemma_lifted_set_bits_not_contains_chunk(chunks, idx + 1, ci, local);
    }
}

//==================================================================================================
// Main Proof
//==================================================================================================

impl SparseBitmap {
    /// After successful construction, the invariant holds.
    pub proof fn lemma_new_establishes_inv(sb: &SparseBitmap)
        requires
            sb.internal_inv(),
        ensures
            sb@.wf(),
    {
        // (2) offsets >= 0 (offset is usize, so offset as int >= 0)
        assert forall|i: int| #![auto] 0 <= i < sb@.chunks.len()
            implies sb@.chunks[i].0 >= 0
        by {
            assert(sb@.chunks[i].0 == sb.chunks@[i].offset as int);
        }

        // (3) sizes > 0 (from strengthened internal_inv: num_bits > 0)
        assert forall|i: int| #![auto] 0 <= i < sb@.chunks.len()
            implies sb@.chunks[i].1 > 0
        by {
            assert(sb@.chunks[i].1 == sb.chunks@[i].bitmap@.num_bits);
        }

        // (4) overflow (from strengthened internal_inv)
        assert forall|i: int| #![auto] 0 <= i < sb@.chunks.len()
            implies sb@.chunks[i].0 + sb@.chunks[i].1 <= usize::MAX as int
        by {
            assert(sb@.chunks[i].0 == sb.chunks@[i].offset as int);
            assert(sb@.chunks[i].1 == sb.chunks@[i].bitmap@.num_bits);
        }

        // (5) sorted non-overlapping (from strengthened internal_inv)
        assert forall|i: int, j: int| #![auto] 0 <= i < j < sb@.chunks.len()
            implies sb@.chunks[i].0 + sb@.chunks[i].1 <= sb@.chunks[j].0
        by {
            assert(sb@.chunks[i].0 == sb.chunks@[i].offset as int);
            assert(sb@.chunks[i].1 == sb.chunks@[i].bitmap@.num_bits);
            assert(sb@.chunks[j].0 == sb.chunks@[j].offset as int);
        }

        // (6) set_bits ⊆ covered
        lemma_lifted_set_bits_subset(sb.chunks@, 0);
        assert forall|g: int| #![auto] sb@.set_bits.contains(g)
            implies sb@.is_covered(g)
        by {
            let k = choose|k: int|
                #![trigger sb.chunks@[k]]
                0 <= k < sb.chunks@.len()
                && sb.chunks@[k].spec_covers(g);
            assert(sb@.chunks[k].0 == sb.chunks@[k].offset as int);
            assert(sb@.chunks[k].1 == sb.chunks@[k].bitmap@.num_bits);
        }

        // (7) set_bits finite
        lemma_lifted_set_bits_finite(sb.chunks@, 0);
    }

    /// Bidirectional lemma: for a chunk at index `ci`, the global set_bits
    /// contains `g = offset + local` iff the chunk's bitmap set_bits contains `local`.
    proof fn lemma_test_chunk(&self, ci: int, local: int)
        requires
            self.inv(),
            0 <= ci < self.chunks@.len() as int,
            0 <= local < self.chunks@[ci].bitmap@.num_bits,
        ensures
            self@.set_bits.contains(self.chunks@[ci].offset as int + local)
                <==> self.chunks@[ci].bitmap@.set_bits.contains(local),
    {
        if self.chunks@[ci].bitmap@.set_bits.contains(local) {
            lemma_lifted_set_bits_contains_chunk(self.chunks@, 0, ci, local);
        } else {
            lemma_lifted_set_bits_not_contains_chunk(self.chunks@, 0, ci, local);
        }
    }
}

/// If no contiguous free range of size 1 exists, the bitmap is full.
/// Extracted from `SparseBitmap::alloc` inline proof block.
proof fn lemma_no_free_single_implies_full(view: SparseBitmapView)
    requires
        view.wf(),
        !view.exists_contiguous_free_range(1),
    ensures
        view.is_full(),
{
    assert forall|idx: int| #![auto] view.is_covered(idx) implies view.set_bits.contains(idx) by {
        if view.is_covered(idx) && !view.set_bits.contains(idx) {
            assert(view.has_contiguous_free_range(idx, 1));
            assert(view.exists_contiguous_free_range(1));
        }
    };
}

//==================================================================================================
// Proof lemmas for set/clear (remove + insert pattern)
//==================================================================================================

/// Key identity: seq.remove(i).insert(i, x) =~= seq.update(i, x)
proof fn lemma_seq_remove_insert_is_update<A>(s: Seq<A>, i: int, x: A)
    requires
        0 <= i < s.len(),
    ensures
        s.remove(i).insert(i, x) =~= s.update(i, x),
{
    let removed = s.remove(i);
    let result = removed.insert(i, x);
    let updated = s.update(i, x);

    s.remove_ensures(i);
    removed.insert_ensures(i, x);

    // Both have the same length
    assert(result.len() == s.len());
    assert(updated.len() == s.len());

    // Prove element-wise equality
    assert forall|k: int| 0 <= k < s.len() implies result[k] == updated[k] by {
        if k < i {
            assert(result[k] == removed[k]);
            assert(removed[k] == s[k]);
        } else if k == i {
            assert(result[k] == x);
        } else {
            // k > i
            assert(result[k] == removed[k - 1]);
            assert(removed[k - 1] == s[k]);
        }
    }
}

/// When two chunk sequences agree everywhere except at index `ci`
/// (where they have the same offset but possibly different bitmaps),
/// lifted_set_bits reflects the change at chunk ci.
///
/// Specifically, for the suffix starting at `idx`:
///   lifted_set_bits(new_chunks, idx) is the union of:
///     - the shifted set_bits of new_chunks[ci] (for the changed chunk)
///     - the shifted set_bits of all other chunks at positions >= idx
///
/// We prove that lifted_set_bits(new_chunks, idx) agrees with
/// lifted_set_bits(old_chunks, idx) for all elements except those
/// whose membership depends on chunk ci.
proof fn lemma_lifted_set_bits_update_chunk(
    old_chunks: Seq<Chunk>, new_chunks: Seq<Chunk>, ci: int, idx: int,
)
    requires
        0 <= idx,
        0 <= ci < old_chunks.len(),
        new_chunks.len() == old_chunks.len(),
        // All chunks except ci are identical
        forall|k: int| 0 <= k < old_chunks.len() && k != ci
            ==> new_chunks[k] == old_chunks[k],
        // Chunk ci has same offset
        new_chunks[ci].offset == old_chunks[ci].offset,
    ensures
        // For any global index g:
        // g is in lifted_set_bits(new_chunks, idx) iff either:
        //   (a) g comes from chunk ci's new bitmap, or
        //   (b) g comes from some other chunk and was already present
        forall|g: int| lifted_set_bits(new_chunks, idx).contains(g) <==> ({
            ||| (idx <= ci && {
                let offset = new_chunks[ci].offset as int;
                let new_bv = new_chunks[ci].bitmap@;
                new_bv.set_bits.contains(g - offset)
            })
            ||| (exists|k: int|
                #![trigger old_chunks[k]]
                idx <= k < old_chunks.len() && k != ci
                && old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int))
        }),
    decreases old_chunks.len() - idx
{
    if idx >= old_chunks.len() {
    } else {
        lemma_lifted_set_bits_update_chunk(old_chunks, new_chunks, ci, idx + 1);

        assert forall|g: int| lifted_set_bits(new_chunks, idx).contains(g) <==> ({
            ||| (idx <= ci && {
                let offset = new_chunks[ci].offset as int;
                let new_bv = new_chunks[ci].bitmap@;
                new_bv.set_bits.contains(g - offset)
            })
            ||| (exists|k: int|
                #![trigger old_chunks[k]]
                idx <= k < old_chunks.len() && k != ci
                && old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int))
        }) by {
            let offset = new_chunks[idx].offset as int;
            let new_bv = new_chunks[idx].bitmap@;
            let shifted = Set::<int>::new(|g: int| new_bv.set_bits.contains(g - offset));

            if idx == ci {
                // This is the changed chunk
                if shifted.contains(g) {
                    // g comes from the new bitmap at ci
                } else if lifted_set_bits(new_chunks, idx + 1).contains(g) {
                    // g comes from some chunk > ci in new_chunks,
                    // which equals the same chunk in old_chunks
                    let k = choose|k: int|
                        #![trigger old_chunks[k]]
                        idx + 1 <= k < old_chunks.len() && k != ci
                        && old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int);
                    assert(old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int));
                }
            } else {
                // idx != ci: this chunk is unchanged
                assert(new_chunks[idx] == old_chunks[idx]);
                if shifted.contains(g) {
                    // g comes from old_chunks[idx] which is not ci
                    assert(old_chunks[idx].bitmap@.set_bits.contains(g - old_chunks[idx].offset as int));
                } else if lifted_set_bits(new_chunks, idx + 1).contains(g) {
                    // from inductive hypothesis
                }
            }
        }
    }
}

/// After updating chunk ci's bitmap (same offset), lifted_set_bits of the
/// new sequence equals: (old lifted_set_bits minus old chunk ci's contribution)
/// union (new chunk ci's contribution).
///
/// More usefully for set: if old bitmap gains exactly one element `local`,
/// then lifted_set_bits gains exactly `offset + local`.
proof fn lemma_lifted_set_bits_set(
    old_chunks: Seq<Chunk>, new_chunks: Seq<Chunk>, ci: int, local: int,
)
    requires
        0 <= ci < old_chunks.len(),
        new_chunks.len() == old_chunks.len(),
        forall|k: int| 0 <= k < old_chunks.len() && k != ci
            ==> new_chunks[k] == old_chunks[k],
        new_chunks[ci].offset == old_chunks[ci].offset,
        // The new bitmap's set_bits = old bitmap's set_bits + {local}
        new_chunks[ci].bitmap@.set_bits =~=
            old_chunks[ci].bitmap@.set_bits.insert(local),
        // All bitmaps valid
        forall|i: int| 0 <= i < old_chunks.len() ==> (#[trigger] old_chunks[i]).bitmap.inv(),
        forall|i: int| 0 <= i < new_chunks.len() ==> (#[trigger] new_chunks[i]).bitmap.inv(),
        // Sorted non-overlapping
        forall|i: int, j: int| #![auto] 0 <= i < j < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= old_chunks[j].offset as int,
        // Chunk ranges don't overflow
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits <= usize::MAX as int,
        // local is in range
        0 <= local < old_chunks[ci].bitmap@.num_bits,
        // The bit was not previously set
        !old_chunks[ci].bitmap@.set_bits.contains(local),
    ensures
        lifted_set_bits(new_chunks, 0) =~=
            lifted_set_bits(old_chunks, 0).insert(old_chunks[ci].offset as int + local),
{
    let g_target = old_chunks[ci].offset as int + local;
    let old_set = old_chunks[ci].bitmap@.set_bits;
    let new_set = new_chunks[ci].bitmap@.set_bits;
    let offset = old_chunks[ci].offset as int;

    // Decompose lifted_set_bits for new_chunks and old_chunks
    lemma_lifted_set_bits_update_chunk(old_chunks, new_chunks, ci, 0);
    lemma_lifted_set_bits_update_chunk(new_chunks, old_chunks, ci, 0);
    lemma_lifted_set_bits_not_contains_chunk(old_chunks, 0, ci, local);

    assert forall|g: int| #![auto]
        lifted_set_bits(new_chunks, 0).contains(g) <==>
        lifted_set_bits(old_chunks, 0).insert(g_target).contains(g)
    by {
        if g == g_target {
            assert(new_set.contains(local));
            assert(new_set.contains(g - offset));
        } else {
            let gl = g - offset;
            assert(gl != local) by {
                assert(g != g_target);
            }

            if lifted_set_bits(new_chunks, 0).contains(g) {
                if new_set.contains(gl) {
                    assert(old_set.insert(local).contains(gl));
                    assert(old_set.contains(gl));
                    lemma_lifted_set_bits_contains_chunk(old_chunks, 0, ci, gl);
                } else {
                    let k = choose|k: int|
                        #![trigger old_chunks[k]]
                        0 <= k < old_chunks.len() && k != ci
                        && old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int);
                    lemma_lifted_set_bits_contains_chunk(old_chunks, 0, k, g - old_chunks[k].offset as int);
                }
            }
            if lifted_set_bits(old_chunks, 0).contains(g) {
                // Use the reverse decomposition: old_chunks decomposes into
                // ci's old bitmap OR some other chunk k != ci (which equals new_chunks[k])
                if old_set.contains(gl) {
                    // Was in old bitmap at ci, so it's in new bitmap
                    assert(new_set.contains(gl));
                } else {
                    // From some other chunk k != ci
                    let k = choose|k: int|
                        #![trigger new_chunks[k]]
                        0 <= k < new_chunks.len() && k != ci
                        && new_chunks[k].bitmap@.set_bits.contains(g - new_chunks[k].offset as int);
                    assert(old_chunks[k] == new_chunks[k]);
                }
            }
        }
    }
}

/// Dual of lemma_lifted_set_bits_set for clearing a bit.
proof fn lemma_lifted_set_bits_clear(
    old_chunks: Seq<Chunk>, new_chunks: Seq<Chunk>, ci: int, local: int,
)
    requires
        0 <= ci < old_chunks.len(),
        new_chunks.len() == old_chunks.len(),
        forall|k: int| 0 <= k < old_chunks.len() && k != ci
            ==> new_chunks[k] == old_chunks[k],
        new_chunks[ci].offset == old_chunks[ci].offset,
        // The new bitmap's set_bits = old bitmap's set_bits - {local}
        new_chunks[ci].bitmap@.set_bits =~=
            old_chunks[ci].bitmap@.set_bits.remove(local),
        // All bitmaps valid
        forall|i: int| 0 <= i < old_chunks.len() ==> (#[trigger] old_chunks[i]).bitmap.inv(),
        forall|i: int| 0 <= i < new_chunks.len() ==> (#[trigger] new_chunks[i]).bitmap.inv(),
        // Sorted non-overlapping
        forall|i: int, j: int| #![auto] 0 <= i < j < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= old_chunks[j].offset as int,
        // Chunk ranges don't overflow
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits <= usize::MAX as int,
        // local is in range
        0 <= local < old_chunks[ci].bitmap@.num_bits,
        // The bit was previously set
        old_chunks[ci].bitmap@.set_bits.contains(local),
    ensures
        lifted_set_bits(new_chunks, 0) =~=
            lifted_set_bits(old_chunks, 0).remove(old_chunks[ci].offset as int + local),
{
    let g_target = old_chunks[ci].offset as int + local;
    let old_set = old_chunks[ci].bitmap@.set_bits;
    let new_set = new_chunks[ci].bitmap@.set_bits;
    let offset = old_chunks[ci].offset as int;

    lemma_lifted_set_bits_update_chunk(old_chunks, new_chunks, ci, 0);
    lemma_lifted_set_bits_update_chunk(new_chunks, old_chunks, ci, 0);
    lemma_lifted_set_bits_contains_chunk(old_chunks, 0, ci, local);

    assert forall|g: int| #![auto]
        lifted_set_bits(new_chunks, 0).contains(g) <==>
        lifted_set_bits(old_chunks, 0).remove(g_target).contains(g)
    by {
        if g == g_target {
            assert(!new_set.contains(local));
            if lifted_set_bits(new_chunks, 0).contains(g) {
                if new_set.contains(g - offset) {
                    assert(false);
                } else {
                    let k = choose|k: int|
                        #![trigger old_chunks[k]]
                        0 <= k < old_chunks.len() && k != ci
                        && old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int);
                    assert(old_chunks[k].spec_covers(g));
                    assert(old_chunks[ci].spec_covers(g));
                    if k < ci {
                        assert(old_chunks[k].offset as int + old_chunks[k].bitmap@.num_bits
                            <= old_chunks[ci].offset as int);
                    } else {
                        assert(old_chunks[ci].offset as int + old_chunks[ci].bitmap@.num_bits
                            <= old_chunks[k].offset as int);
                    }
                }
            }
        } else {
            let gl = g - offset;
            assert(gl != local) by {
                assert(g != g_target);
            }

            if lifted_set_bits(new_chunks, 0).contains(g) {
                if new_set.contains(gl) {
                    assert(old_set.remove(local).contains(gl));
                    assert(old_set.contains(gl));
                    lemma_lifted_set_bits_contains_chunk(old_chunks, 0, ci, gl);
                } else {
                    let k = choose|k: int|
                        #![trigger old_chunks[k]]
                        0 <= k < old_chunks.len() && k != ci
                        && old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int);
                    lemma_lifted_set_bits_contains_chunk(old_chunks, 0, k, g - old_chunks[k].offset as int);
                }
            }
            if lifted_set_bits(old_chunks, 0).contains(g) {
                // Use reverse decomposition
                if old_set.contains(gl) {
                    assert(new_set.contains(gl));
                } else {
                    let k = choose|k: int|
                        #![trigger new_chunks[k]]
                        0 <= k < new_chunks.len() && k != ci
                        && new_chunks[k].bitmap@.set_bits.contains(g - new_chunks[k].offset as int);
                    assert(old_chunks[k] == new_chunks[k]);
                }
            }
        }
    }
}

/// Lemma: when a chunk's bitmap is unchanged, remove+insert restores the original sequence.
proof fn lemma_seq_remove_insert_identity<A>(s: Seq<A>, i: int)
    requires
        0 <= i < s.len(),
    ensures
        s.remove(i).insert(i, s[i]) =~= s,
{
    lemma_seq_remove_insert_is_update(s, i, s[i]);
    // s.update(i, s[i]) =~= s (by extensional equality)
    assert forall|k: int| #![auto] 0 <= k < s.len() implies s.update(i, s[i])[k] == s[k] by {
        if k == i {
        } else {
        }
    }
}

/// Two SparseBitmapViews with extensionally equal chunks have the same capacity.
proof fn lemma_capacity_from_depends_only_on_chunks(
    v1: SparseBitmapView, v2: SparseBitmapView, from: int,
)
    requires
        v1.chunks =~= v2.chunks,
        from >= 0,
    ensures
        v1.capacity_from(from) == v2.capacity_from(from),
    decreases v1.chunks.len() - from
{
    if from >= v1.chunks.len() {
    } else {
        lemma_capacity_from_depends_only_on_chunks(v1, v2, from + 1);
    }
}

//==================================================================================================
// Connecting lemmas for new() verification
//==================================================================================================

/// chunk_seq_capacity on built chunks equals input_capacity on matching input.
proof fn lemma_chunk_seq_capacity_matches_input(
    chunks: Seq<Chunk>, input: Seq<(usize, Bitmap)>, idx: int,
)
    requires
        idx >= 0,
        chunks.len() == input.len(),
        forall|k: int| idx <= k < chunks.len() ==>
            (#[trigger] chunks[k]).bitmap == input[k].1,
    ensures
        chunk_seq_capacity(chunks, idx) == input_capacity(input, idx),
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
    } else {
        lemma_chunk_seq_capacity_matches_input(chunks, input, idx + 1);
    }
}

/// lifted_set_bits on built chunks equals input_lifted_set_bits on matching input.
proof fn lemma_lifted_set_bits_matches_input(
    chunks: Seq<Chunk>, input: Seq<(usize, Bitmap)>, idx: int,
)
    requires
        idx >= 0,
        chunks.len() == input.len(),
        forall|k: int| idx <= k < chunks.len() ==> {
            &&& (#[trigger] chunks[k]).offset == input[k].0
            &&& chunks[k].bitmap == input[k].1
        },
    ensures
        lifted_set_bits(chunks, idx) =~= input_lifted_set_bits(input, idx),
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
    } else {
        lemma_lifted_set_bits_matches_input(chunks, input, idx + 1);
        // At idx: offset and bitmap match, so shifted sets match
        assert(chunks[idx].offset as int == input[idx].0 as int);
        assert(chunks[idx].bitmap@ == input[idx].1@);
    }
}

/// chunk_seq_capacity on chunks equals capacity_from on the corresponding view.
proof fn lemma_chunk_seq_capacity_eq_capacity_from(
    chunks: Seq<Chunk>, view: SparseBitmapView, idx: int,
)
    requires
        idx >= 0,
        chunks.len() == view.chunks.len(),
        forall|k: int| idx <= k < chunks.len() ==>
            view.chunks[k].1 == (#[trigger] chunks[k]).bitmap@.num_bits,
    ensures
        chunk_seq_capacity(chunks, idx) == view.capacity_from(idx),
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
    } else {
        lemma_chunk_seq_capacity_eq_capacity_from(chunks, view, idx + 1);
    }
}

/// Appending a chunk extends chunk_seq_capacity by the new chunk's num_bits.
proof fn lemma_chunk_seq_capacity_push(chunks: Seq<Chunk>, c: Chunk, idx: int)
    requires
        idx >= 0,
        idx <= chunks.len(),
    ensures
        chunk_seq_capacity(chunks.push(c), idx) ==
            chunk_seq_capacity(chunks, idx) + c.bitmap@.num_bits,
    decreases chunks.len() - idx
{
    reveal_with_fuel(chunk_seq_capacity, 2);
    let pushed = chunks.push(c);
    if idx >= chunks.len() {
        assert(pushed[idx as int] == c);
    } else {
        assert(pushed[idx as int] == chunks[idx as int]);
        lemma_chunk_seq_capacity_push(chunks, c, idx + 1);
    }
}

//==================================================================================================
// Connecting lemma for alloc_range() verification
//==================================================================================================

/// After updating chunk ci's bitmap by unioning a range of local bits
/// [local_start, local_start + count), lifted_set_bits gains exactly
/// BitmapView::range_set(global_start, global_start + count) where
/// global_start = offset + local_start.
///
/// Generalises `lemma_lifted_set_bits_set` from a single bit to a range.
proof fn lemma_lifted_set_bits_alloc_range(
    old_chunks: Seq<Chunk>, new_chunks: Seq<Chunk>, ci: int, local_start: int, count: int,
)
    requires
        0 <= ci < old_chunks.len(),
        new_chunks.len() == old_chunks.len(),
        forall|k: int| 0 <= k < old_chunks.len() && k != ci
            ==> new_chunks[k] == old_chunks[k],
        new_chunks[ci].offset == old_chunks[ci].offset,
        new_chunks[ci].bitmap@.set_bits =~=
            old_chunks[ci].bitmap@.set_bits.union(
                BitmapView::range_set(local_start, local_start + count)),
        forall|i: int| 0 <= i < old_chunks.len() ==> (#[trigger] old_chunks[i]).bitmap.inv(),
        forall|i: int| 0 <= i < new_chunks.len() ==> (#[trigger] new_chunks[i]).bitmap.inv(),
        forall|i: int, j: int| #![auto] 0 <= i < j < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= old_chunks[j].offset as int,
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits <= usize::MAX as int,
        0 <= local_start,
        local_start + count <= old_chunks[ci].bitmap@.num_bits,
        count > 0,
        // All bits in the range were unset before
        forall|j: int| local_start <= j < local_start + count
            ==> !old_chunks[ci].bitmap@.set_bits.contains(j),
    ensures
        lifted_set_bits(new_chunks, 0) =~=
            lifted_set_bits(old_chunks, 0).union(
                BitmapView::range_set(
                    old_chunks[ci].offset as int + local_start,
                    old_chunks[ci].offset as int + local_start + count)),
{
    let offset = old_chunks[ci].offset as int;
    let gs = offset + local_start;
    let ge = offset + local_start + count;

    lemma_lifted_set_bits_update_chunk(old_chunks, new_chunks, ci, 0);
    lemma_lifted_set_bits_update_chunk(new_chunks, old_chunks, ci, 0);

    assert forall|g: int| #![auto]
        lifted_set_bits(new_chunks, 0).contains(g) <==>
        lifted_set_bits(old_chunks, 0).union(BitmapView::range_set(gs, ge)).contains(g)
    by {
        let gl = g - offset;

        if gs <= g && g < ge {
            // g is in the global range [gs, ge)
            // gl = g - offset is in [local_start, local_start + count)
            assert(BitmapView::range_set(local_start, local_start + count).contains(gl));
            assert(new_chunks[ci].bitmap@.set_bits.contains(gl));
            // → direction: g is in new lifted (via update_chunk, chunk ci contribution)
            // ← direction: g is in range_set(gs, ge) trivially, so in RHS
        } else {
            // g is NOT in the global range [gs, ge)
            // So gl is NOT in [local_start, local_start + count)
            assert(!BitmapView::range_set(local_start, local_start + count).contains(gl));

            // → direction: new lifted contains g implies old lifted contains g
            if lifted_set_bits(new_chunks, 0).contains(g) {
                if new_chunks[ci].bitmap@.set_bits.contains(gl) {
                    // gl not in range_set, but in new_bits = old_bits ∪ range_set
                    // So gl must be in old_bits
                    assert(old_chunks[ci].bitmap@.set_bits.contains(gl));
                    lemma_lifted_set_bits_contains_chunk(old_chunks, 0, ci, gl);
                } else {
                    // g comes from some other chunk k != ci
                    let k = choose|k: int|
                        #![trigger old_chunks[k]]
                        0 <= k < old_chunks.len() && k != ci
                        && old_chunks[k].bitmap@.set_bits.contains(g - old_chunks[k].offset as int);
                    lemma_lifted_set_bits_contains_chunk(
                        old_chunks, 0, k, g - old_chunks[k].offset as int,
                    );
                }
            }

            // ← direction: old lifted contains g implies new lifted contains g
            if lifted_set_bits(old_chunks, 0).contains(g) {
                if old_chunks[ci].bitmap@.set_bits.contains(gl) {
                    // old_bits ⊆ new_bits (new = old ∪ range)
                    assert(new_chunks[ci].bitmap@.set_bits.contains(gl));
                } else {
                    // exists k != ci in old decomposition
                    let k = choose|k: int|
                        #![trigger new_chunks[k]]
                        0 <= k < new_chunks.len() && k != ci
                        && new_chunks[k].bitmap@.set_bits.contains(g - new_chunks[k].offset as int);
                    assert(old_chunks[k] == new_chunks[k]);
                }
            }
        }
    }
}

//==================================================================================================
// Proof lemmas for sort_chunks_by_offset (move-left = remove(j).insert(i, s[j]))
//==================================================================================================

/// Index characterization of move-left: s.remove(j).insert(i, s[j])[k] equals
/// a known element of s depending on where k falls relative to i and j.
proof fn lemma_move_left_elem<A>(s: Seq<A>, i: int, j: int, k: int)
    requires
        0 <= i <= j < s.len(),
        0 <= k < s.len(),
    ensures
        s.remove(j).insert(i, s[j]).len() == s.len(),
        k < i ==> s.remove(j).insert(i, s[j])[k] == s[k],
        k == i ==> s.remove(j).insert(i, s[j])[k] == s[j],
        i < k && k <= j ==> s.remove(j).insert(i, s[j])[k] == s[k - 1],
        k > j ==> s.remove(j).insert(i, s[j])[k] == s[k],
{
    s.remove_ensures(j);
    let removed = s.remove(j);
    removed.insert_ensures(i, s[j]);
    let moved = removed.insert(i, s[j]);

    if k < i {
        assert(moved[k] == removed[k]);
        assert(removed[k] == s[k]);
    } else if k == i {
        assert(moved[k] == s[j]);
    } else if k <= j {
        assert(moved[k] == removed[k - 1]);
        // k - 1 >= i >= 0 and k - 1 < j, so removed[k-1] = s[k-1]
        assert(removed[k - 1] == s[k - 1]);
    } else {
        assert(moved[k] == removed[k - 1]);
        // k - 1 >= j, so removed[k-1] = s[(k-1)+1] = s[k]
        assert(removed[k - 1] == s[k]);
    }
}

/// After a selection-sort step (remove element at `hi`, insert at `lo`),
/// proves that all chunk element properties and sort invariants are maintained.
proof fn lemma_sort_step(
    pre: Seq<Chunk>, post: Seq<Chunk>, lo: int, hi: int,
)
    requires
        0 <= lo <= hi < pre.len(),
        post =~= pre.remove(hi).insert(lo, pre[hi]),
        // Pre-sort prefix [0, lo) is sorted
        forall|k: int, l: int| 0 <= k < l < lo
            ==> (#[trigger] pre[k]).offset <= (#[trigger] pre[l]).offset,
        // Pre-sort prefix [0, lo) <= suffix [lo, n)
        forall|k: int, l: int| 0 <= k < lo && lo <= l < pre.len()
            ==> (#[trigger] pre[k]).offset <= (#[trigger] pre[l]).offset,
        // hi holds the minimum offset in [lo, n)
        forall|k: int| lo <= k < pre.len()
            ==> pre[hi].offset <= (#[trigger] pre[k]).offset,
        // Element properties on pre
        forall|k: int| 0 <= k < pre.len() ==> (#[trigger] pre[k]).bitmap.inv(),
        forall|k: int| #![auto] 0 <= k < pre.len() ==> pre[k].bitmap@.num_bits > 0,
        forall|k: int| #![auto] 0 <= k < pre.len() ==> pre[k].bitmap@.set_bits.finite(),
        forall|k: int| #![auto] 0 <= k < pre.len()
            ==> pre[k].offset as int + pre[k].bitmap@.num_bits <= usize::MAX as int,
    ensures
        // Post-sort prefix [0, lo+1) is sorted
        forall|k: int, l: int| 0 <= k < l < lo + 1
            ==> (#[trigger] post[k]).offset <= (#[trigger] post[l]).offset,
        // Post-sort prefix [0, lo+1) <= suffix [lo+1, n)
        forall|k: int, l: int| 0 <= k < lo + 1 && lo + 1 <= l < post.len()
            ==> (#[trigger] post[k]).offset <= (#[trigger] post[l]).offset,
        // Element properties preserved
        forall|k: int| 0 <= k < post.len() ==> (#[trigger] post[k]).bitmap.inv(),
        forall|k: int| #![auto] 0 <= k < post.len() ==> post[k].bitmap@.num_bits > 0,
        forall|k: int| #![auto] 0 <= k < post.len() ==> post[k].bitmap@.set_bits.finite(),
        forall|k: int| #![auto] 0 <= k < post.len()
            ==> post[k].offset as int + post[k].bitmap@.num_bits <= usize::MAX as int,
{
    assert forall|k: int| 0 <= k < post.len()
        implies (#[trigger] post[k]).bitmap.inv()
    by { lemma_move_left_elem::<Chunk>(pre, lo, hi, k); }

    assert forall|k: int| #![auto] 0 <= k < post.len()
        implies post[k].bitmap@.num_bits > 0
    by { lemma_move_left_elem::<Chunk>(pre, lo, hi, k); }

    assert forall|k: int| #![auto] 0 <= k < post.len()
        implies post[k].bitmap@.set_bits.finite()
    by { lemma_move_left_elem::<Chunk>(pre, lo, hi, k); }

    assert forall|k: int| #![auto] 0 <= k < post.len()
        implies post[k].offset as int + post[k].bitmap@.num_bits <= usize::MAX as int
    by { lemma_move_left_elem::<Chunk>(pre, lo, hi, k); }

    assert forall|k: int, l: int| 0 <= k < l < lo + 1
        implies (#[trigger] post[k]).offset <= (#[trigger] post[l]).offset
    by {
        lemma_move_left_elem::<Chunk>(pre, lo, hi, k);
        lemma_move_left_elem::<Chunk>(pre, lo, hi, l);
    }

    assert forall|k: int, l: int| 0 <= k < lo + 1 && lo + 1 <= l < post.len()
        implies (#[trigger] post[k]).offset <= (#[trigger] post[l]).offset
    by {
        lemma_move_left_elem::<Chunk>(pre, lo, hi, k);
        lemma_move_left_elem::<Chunk>(pre, lo, hi, l);
    }
}

/// chunk_seq_capacity after Seq::remove: subtracts the removed element's contribution.
/// Generalized over `from` for inductive proof.
proof fn lemma_chunk_seq_capacity_remove_from(s: Seq<Chunk>, j: int, from: int)
    requires
        0 <= j < s.len(),
        0 <= from,
    ensures
        from <= j ==> chunk_seq_capacity(s.remove(j), from) ==
            chunk_seq_capacity(s, from) - s[j].bitmap@.num_bits,
        from > j ==> chunk_seq_capacity(s.remove(j), from) ==
            chunk_seq_capacity(s, from + 1),
    decreases s.len() - from
{
    s.remove_ensures(j);
    let removed = s.remove(j);

    reveal_with_fuel(chunk_seq_capacity, 2);

    if from >= removed.len() {
        if from <= j {
            assert(from == s.len() - 1);
            assert(j == s.len() - 1);
        }
    } else {
        lemma_chunk_seq_capacity_remove_from(s, j, from + 1);

        if from < j {
            assert(removed[from] == s[from]);
        } else if from == j {
            assert(removed[from] == s[from + 1]);
        } else {
            assert(removed[from] == s[from + 1]);
        }
    }
}

/// chunk_seq_capacity after Seq::insert: adds the inserted element's contribution.
/// Generalized over `from` for inductive proof.
proof fn lemma_chunk_seq_capacity_insert_from(s: Seq<Chunk>, i: int, c: Chunk, from: int)
    requires
        0 <= i <= s.len(),
        0 <= from,
    ensures
        from <= i ==> chunk_seq_capacity(s.insert(i, c), from) ==
            chunk_seq_capacity(s, from) + c.bitmap@.num_bits,
        from > i ==> chunk_seq_capacity(s.insert(i, c), from) ==
            chunk_seq_capacity(s, from - 1),
    decreases s.len() + 1 - from
{
    s.insert_ensures(i, c);
    let inserted = s.insert(i, c);

    if from >= inserted.len() {
        // chunk_seq_capacity(inserted, from) = 0
        if from <= i {
            // from >= s.len() + 1 and from <= i <= s.len(): impossible since from > s.len() >= i
            assert(from >= s.len() + 1);
            assert(from <= i);
            assert(i <= s.len());
            assert(false);
        }
        // from > i: from - 1 >= s.len(), so chunk_seq_capacity(s, from - 1) = 0
    } else {
        lemma_chunk_seq_capacity_insert_from(s, i, c, from + 1);

        reveal_with_fuel(chunk_seq_capacity, 2);

        if from < i {
            assert(inserted[from] == s[from]);
        } else if from == i {
            assert(inserted[from] == c);
        } else {
            // from > i
            assert(inserted[from] == s[from - 1]);
        }
    }
}

/// Membership characterization of lifted_set_bits: g is in lifted_set_bits(chunks, idx)
/// iff some chunk k >= idx has g in its shifted set_bits.
proof fn lemma_lifted_set_bits_membership(chunks: Seq<Chunk>, idx: int, g: int)
    requires
        idx >= 0,
    ensures
        lifted_set_bits(chunks, idx).contains(g) <==>
            exists|k: int| #![trigger chunks[k]]
                idx <= k < chunks.len()
                && chunks[k].bitmap@.set_bits.contains(g - chunks[k].offset as int),
    decreases chunks.len() - idx
{
    if idx >= chunks.len() {
        // Empty set, no k exists
    } else {
        lemma_lifted_set_bits_membership(chunks, idx + 1, g);
        // By IH: lifted_set_bits(chunks, idx+1).contains(g) <==>
        //   exists k in [idx+1, len): chunks[k].bitmap@.set_bits.contains(...)

        // lifted_set_bits(chunks, idx) = shifted ∪ lifted_set_bits(chunks, idx+1)
        // where shifted = Set::new(|g| chunks[idx].bitmap@.set_bits.contains(g - chunks[idx].offset))

        let offset = chunks[idx].offset as int;
        let bv = chunks[idx].bitmap@;
        let shifted = Set::<int>::new(|g: int| bv.set_bits.contains(g - offset));

        if shifted.contains(g) {
            // Witness: k = idx
            assert(chunks[idx].bitmap@.set_bits.contains(g - chunks[idx].offset as int));
        }
        // If g in rest (idx+1..), the IH existential witness serves directly.
        // Backward: if exists k in [idx, len), either k == idx (shifted) or k > idx (rest via IH).
    }
}

/// lifted_set_bits is preserved under move-left: s.remove(j).insert(i, s[j]).
proof fn lemma_lifted_set_bits_move_left(s: Seq<Chunk>, i: int, j: int)
    requires
        0 <= i <= j < s.len(),
    ensures
        lifted_set_bits(s.remove(j).insert(i, s[j]), 0) =~= lifted_set_bits(s, 0),
{
    s.remove_ensures(j);
    let removed = s.remove(j);
    removed.insert_ensures(i, s[j]);
    let moved = removed.insert(i, s[j]);

    assert forall|g: int|
        lifted_set_bits(moved, 0).contains(g) <==> lifted_set_bits(s, 0).contains(g)
    by {
        lemma_lifted_set_bits_membership(moved, 0, g);
        lemma_lifted_set_bits_membership(s, 0, g);

        // Forward: if g in lifted_set_bits(moved, 0)
        if lifted_set_bits(moved, 0).contains(g) {
            let km: int = choose|k: int| #![trigger moved[k]]
                0 <= k < moved.len()
                && moved[k].bitmap@.set_bits.contains(g - moved[k].offset as int);
            // Map km to the corresponding index in s
            lemma_move_left_elem::<Chunk>(s, i, j, km);
            if km < i {
                assert(s[km].bitmap@.set_bits.contains(g - s[km].offset as int));
            } else if km == i {
                assert(s[j].bitmap@.set_bits.contains(g - s[j].offset as int));
            } else if km <= j {
                assert(s[km - 1].bitmap@.set_bits.contains(g - s[km - 1].offset as int));
            } else {
                assert(s[km].bitmap@.set_bits.contains(g - s[km].offset as int));
            }
        }

        // Backward: if g in lifted_set_bits(s, 0)
        if lifted_set_bits(s, 0).contains(g) {
            let ks: int = choose|k: int| #![trigger s[k]]
                0 <= k < s.len()
                && s[k].bitmap@.set_bits.contains(g - s[k].offset as int);
            // Map ks to the corresponding index in moved
            if ks < i {
                lemma_move_left_elem::<Chunk>(s, i, j, ks);
                assert(moved[ks].bitmap@.set_bits.contains(g - moved[ks].offset as int));
            } else if ks == j {
                lemma_move_left_elem::<Chunk>(s, i, j, i);
                assert(moved[i].bitmap@.set_bits.contains(g - moved[i].offset as int));
            } else if ks >= i && ks < j {
                lemma_move_left_elem::<Chunk>(s, i, j, ks + 1);
                assert(moved[ks + 1].bitmap@.set_bits.contains(g - moved[ks + 1].offset as int));
            } else {
                lemma_move_left_elem::<Chunk>(s, i, j, ks);
                assert(moved[ks].bitmap@.set_bits.contains(g - moved[ks].offset as int));
            }
        }
    }
}

//==================================================================================================
// seq_sum_from Lemmas
//==================================================================================================

/// Unfolding lemma: seq_sum_from(s, from) == s[from] + seq_sum_from(s, from + 1) when from < s.len()
proof fn lemma_seq_sum_from_unfold(s: Seq<int>, from: int)
    requires
        0 <= from < s.len(),
    ensures
        seq_sum_from(s, from) == s[from] + seq_sum_from(s, from + 1),
{
    // Follows directly from the definition
}

/// When all entries in s[from..] are > 0 and from < s.len(), seq_sum_from >= s[from] > 0.
proof fn lemma_seq_sum_from_positive(s: Seq<int>, from: int)
    requires
        0 <= from < s.len(),
        forall|i: int| from <= i < s.len() ==> s[i] > 0,
    ensures
        seq_sum_from(s, from) >= s[from],
        seq_sum_from(s, from) > 0,
    decreases s.len() - from,
{
    lemma_seq_sum_from_unfold(s, from);
    // seq_sum_from(s, from) == s[from] + seq_sum_from(s, from+1)
    if from + 1 >= s.len() {
        // seq_sum_from(s, from+1) == 0
    } else {
        lemma_seq_sum_from_positive(s, from + 1);
        // seq_sum_from(s, from+1) > 0
    }
}

/// seq_sum_from is non-negative when all entries from `from` onwards are > 0.
proof fn lemma_seq_sum_from_nonneg(s: Seq<int>, from: int)
    requires
        from >= 0,
        forall|i: int| from <= i < s.len() ==> s[i] > 0,
    ensures
        seq_sum_from(s, from) >= 0,
    decreases (if from < s.len() { s.len() - from } else { 0 }),
{
    if from >= s.len() {
        // seq_sum_from(s, from) == 0
    } else {
        lemma_seq_sum_from_unfold(s, from);
        lemma_seq_sum_from_nonneg(s, from + 1);
        // s[from] > 0 and seq_sum_from(s, from+1) >= 0
    }
}

/// If seq_sum_from(s, from) > 0 and all entries are >= 0, then from < s.len().
proof fn lemma_seq_sum_from_positive_implies_in_range(s: Seq<int>, from: int)
    requires
        seq_sum_from(s, from) > 0,
        forall|i: int| 0 <= i < s.len() ==> s[i] > 0,
        from >= 0,
    ensures
        from < s.len(),
    decreases (if from < s.len() { s.len() - from } else { 0int }),
{
    if from >= s.len() {
        // seq_sum_from(s, from) == 0, contradicts > 0
    }
}

/// seq_sum_from after subtracting the first element.
proof fn lemma_seq_sum_from_subtract(s: Seq<int>, from: int)
    requires
        0 <= from < s.len(),
    ensures
        seq_sum_from(s, from) - s[from] == seq_sum_from(s, from + 1),
{
    lemma_seq_sum_from_unfold(s, from);
}

/// Pushing an element onto the end: sum over the whole extended seq
/// equals sum over the original seq plus the new element.
proof fn lemma_seq_sum_from_push(s: Seq<int>, val: int, from: int)
    requires
        0 <= from,
    ensures
        seq_sum_from(s.push(val), from)
            == seq_sum_from(s, from) + (if from <= s.len() { val } else { 0 }),
    decreases (if from < s.push(val).len() { s.push(val).len() - from } else { 0 }),
{
    let ext = s.push(val);
    if from > s.len() {
        // Both are 0
        assert(seq_sum_from(ext, from) == 0);
        assert(seq_sum_from(s, from) == 0);
    } else if from == s.len() {
        // ext[from] == val, ext has one more entry at from
        assert(ext[from] == val);
        assert(seq_sum_from(ext, from) == val + seq_sum_from(ext, from + 1));
        assert(seq_sum_from(ext, from + 1) == 0);
        assert(seq_sum_from(s, from) == 0);
    } else {
        // from < s.len()
        assert(ext[from] == s[from]);
        lemma_seq_sum_from_push(s, val, from + 1);
        // seq_sum_from(ext, from) == ext[from] + seq_sum_from(ext, from+1)
        //                         == s[from] + (seq_sum_from(s, from+1) + val)
        //                         == (s[from] + seq_sum_from(s, from+1)) + val
        //                         == seq_sum_from(s, from) + val
    }
}

//==================================================================================================
// Structural lemma: chunk update preserves invariant-relevant properties
//==================================================================================================

/// Proves that pairwise non-overlapping for consecutive chunks implies
/// non-overlapping for ALL pairs, using sorted-order transitivity.
proof fn lemma_nonoverlap_from_consecutive(chunks: Seq<Chunk>)
    requires
        chunks.len() > 0,
        forall|i: int, j: int| 0 <= i < j < chunks.len()
            ==> (#[trigger] chunks[i]).offset <= (#[trigger] chunks[j]).offset,
        forall|i: int| 0 <= i < chunks.len() - 1 ==>
            chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= (#[trigger] chunks[i + 1]).offset as int,
    ensures
        forall|k: int, l: int| #![auto] 0 <= k < l < chunks.len() ==>
            chunks[k].offset as int + chunks[k].bitmap@.num_bits
                <= chunks[l].offset as int,
{
    assert forall|k: int, l: int| #![auto] 0 <= k < l < chunks.len() implies
        chunks[k].offset as int + chunks[k].bitmap@.num_bits
            <= chunks[l].offset as int
    by {
        if l == k + 1 {
        } else {
            assert(chunks[k].offset as int + chunks[k].bitmap@.num_bits
                <= chunks[k + 1].offset as int);
            assert(chunks[k + 1].offset <= chunks[l].offset);
        }
    };
}

/// After replacing a single chunk (preserving its offset and num_bits),
/// the structural properties required by `internal_inv` are preserved.
///
/// This factors out the repeated "all bitmaps inv / num_bits > 0 /
/// set_bits finite / no overflow / sorted non-overlapping / view-chunks
/// match" proof that appears after every remove+mutate+insert cycle.
proof fn lemma_chunk_update_preserves_structure(
    old_chunks: Seq<Chunk>, new_chunks: Seq<Chunk>, ci: int,
)
    requires
        0 <= ci < old_chunks.len(),
        new_chunks.len() == old_chunks.len(),
        // Non-ci chunks identical
        forall|k: int| 0 <= k < old_chunks.len() && k != ci
            ==> new_chunks[k] == old_chunks[k],
        // Updated chunk preserves offset and num_bits
        new_chunks[ci].offset == old_chunks[ci].offset,
        new_chunks[ci].bitmap@.num_bits == old_chunks[ci].bitmap@.num_bits,
        // Updated chunk is individually valid
        new_chunks[ci].bitmap.inv(),
        new_chunks[ci].bitmap@.set_bits.finite(),
        // Old chunks satisfied structural properties
        forall|i: int| 0 <= i < old_chunks.len()
            ==> (#[trigger] old_chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].bitmap@.set_bits.finite(),
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= usize::MAX as int,
        forall|i: int, j: int| #![auto] 0 <= i < j < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= old_chunks[j].offset as int,
    ensures
        // All bitmaps valid
        forall|k: int| 0 <= k < new_chunks.len()
            ==> (#[trigger] new_chunks[k]).bitmap.inv(),
        // Positive sizes
        forall|k: int| #![auto] 0 <= k < new_chunks.len()
            ==> new_chunks[k].bitmap@.num_bits > 0,
        // Finite set_bits
        forall|k: int| #![auto] 0 <= k < new_chunks.len()
            ==> new_chunks[k].bitmap@.set_bits.finite(),
        // No overflow
        forall|k: int| #![auto] 0 <= k < new_chunks.len()
            ==> new_chunks[k].offset as int + new_chunks[k].bitmap@.num_bits
                <= usize::MAX as int,
        // Sorted non-overlapping
        forall|i: int, j: int| #![auto] 0 <= i < j < new_chunks.len()
            ==> new_chunks[i].offset as int + new_chunks[i].bitmap@.num_bits
                <= new_chunks[j].offset as int,
{
    assert forall|k: int| 0 <= k < new_chunks.len()
        implies (#[trigger] new_chunks[k]).bitmap.inv()
    by { if k == ci {} else {} }

    assert forall|k: int| #![auto] 0 <= k < new_chunks.len()
        implies new_chunks[k].bitmap@.num_bits > 0
    by {
        if k == ci {
            assert(new_chunks[k].bitmap@.num_bits == old_chunks[ci].bitmap@.num_bits);
        } else {}
    }

    assert forall|k: int| #![auto] 0 <= k < new_chunks.len()
        implies new_chunks[k].bitmap@.set_bits.finite()
    by { if k == ci {} else {} }

    assert forall|k: int| #![auto] 0 <= k < new_chunks.len()
        implies new_chunks[k].offset as int + new_chunks[k].bitmap@.num_bits
            <= usize::MAX as int
    by {
        if k == ci {
            assert(new_chunks[k].offset == old_chunks[ci].offset);
            assert(new_chunks[k].bitmap@.num_bits == old_chunks[ci].bitmap@.num_bits);
        } else {}
    }

    assert forall|i: int, j: int| #![auto] 0 <= i < j < new_chunks.len()
        implies new_chunks[i].offset as int + new_chunks[i].bitmap@.num_bits
            <= new_chunks[j].offset as int
    by {
        if i == ci {
            assert(new_chunks[i].offset == old_chunks[ci].offset);
            assert(new_chunks[i].bitmap@.num_bits == old_chunks[ci].bitmap@.num_bits);
        } else if j == ci {
            assert(new_chunks[j].offset == old_chunks[ci].offset);
        } else {}
    }
}

/// After updating a single chunk (via remove/mutate/insert), re-establish
/// `sb.inv()` and prove the abstract chunk sequence is unchanged.
/// Combines `lemma_chunk_update_preserves_structure`, view-chunk consistency,
/// capacity preservation, and `lemma_new_establishes_inv`.
proof fn lemma_inv_after_chunk_update(
    sb: &SparseBitmap,
    old_chunks: Seq<Chunk>,
    old_view: SparseBitmapView,
    ci: int,
)
    requires
        0 <= ci < old_chunks.len(),
        sb.chunks@.len() == old_chunks.len(),
        forall|k: int| #![auto] 0 <= k < old_chunks.len() && k != ci
            ==> sb.chunks@[k] == old_chunks[k],
        sb.chunks@[ci].offset == old_chunks[ci].offset,
        sb.chunks@[ci].bitmap@.num_bits == old_chunks[ci].bitmap@.num_bits,
        sb.chunks@[ci].bitmap.inv(),
        sb.chunks@[ci].bitmap@.set_bits.finite(),
        forall|i: int| 0 <= i < old_chunks.len()
            ==> (#[trigger] old_chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].bitmap@.set_bits.finite(),
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= usize::MAX as int,
        forall|i: int, j: int| #![auto] 0 <= i < j < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= old_chunks[j].offset as int,
        sb.capacity_bits as int == old_view.capacity(),
        (sb.next_chunk_hint as int) < sb.chunks@.len(),
        old_view.chunks.len() == old_chunks.len(),
        forall|k: int| #![auto] 0 <= k < old_chunks.len() ==> {
            &&& old_view.chunks[k].0 == old_chunks[k].offset as int
            &&& old_view.chunks[k].1 == old_chunks[k].bitmap@.num_bits
        },
    ensures
        sb.inv(),
        sb@.chunks =~= old_view.chunks,
{
    lemma_chunk_update_preserves_structure(old_chunks, sb.chunks@, ci);

    assert(sb@.chunks.len() == sb.chunks@.len());
    assert forall|k: int| #![auto] 0 <= k < sb.chunks@.len() implies {
        &&& sb@.chunks[k].0 == sb.chunks@[k].offset as int
        &&& sb@.chunks[k].1 == sb.chunks@[k].bitmap@.num_bits
    } by {}

    // Extensional equality: sb@.chunks =~= old_view.chunks
    assert forall|k: int| #![auto] 0 <= k < sb@.chunks.len() implies
        sb@.chunks[k] == old_view.chunks[k]
    by {
        assert(sb@.chunks[k].0 == sb.chunks@[k].offset as int);
        assert(sb@.chunks[k].1 == sb.chunks@[k].bitmap@.num_bits);
        if k == ci {
            assert(sb.chunks@[ci].offset == old_chunks[ci].offset);
            assert(sb.chunks@[ci].bitmap@.num_bits == old_chunks[ci].bitmap@.num_bits);
        } else {
            assert(sb.chunks@[k] == old_chunks[k]);
        }
        assert(old_view.chunks[k].0 == old_chunks[k].offset as int);
        assert(old_view.chunks[k].1 == old_chunks[k].bitmap@.num_bits);
    }
    assert(sb@.chunks =~= old_view.chunks);

    lemma_capacity_from_depends_only_on_chunks(sb@, old_view, 0);
    assert(sb.internal_inv());
    SparseBitmap::lemma_new_establishes_inv(sb);
}

//==================================================================================================
// find_chunk_index helper lemmas
//==================================================================================================

/// After binary search yields lo == 0 (all chunk offsets > index),
/// no chunk covers the index.
proof fn lemma_no_chunk_covers_all_above(
    chunks: Seq<Chunk>, view_chunks: Seq<(int, int)>, index: int,
)
    requires
        chunks.len() == view_chunks.len(),
        // All chunk offsets are above index
        forall|k: int| 0 <= k < chunks.len()
            ==> (#[trigger] chunks[k]).offset as int > index,
        // View matches concrete
        forall|k: int| #![auto] 0 <= k < chunks.len() ==> {
            &&& view_chunks[k].0 == chunks[k].offset as int
            &&& view_chunks[k].1 == chunks[k].bitmap@.num_bits
        },
    ensures
        forall|i: int|
            #![trigger view_chunks[i]]
            0 <= i < view_chunks.len()
            ==> !(view_chunks[i].0 <= index
                  && index < view_chunks[i].0 + view_chunks[i].1),
{
    assert forall|i: int|
        #![trigger view_chunks[i]]
        0 <= i < view_chunks.len()
        implies !(view_chunks[i].0 <= index
                  && index < view_chunks[i].0 + view_chunks[i].1)
    by {
        assert(view_chunks[i].0 == chunks[i].offset as int);
        assert(chunks[i].offset as int > index);
    }
}

/// After binary search, if the candidate chunk (lo-1) doesn't cover
/// the index, no chunk does.
proof fn lemma_no_chunk_covers_with_gap(
    chunks: Seq<Chunk>, view_chunks: Seq<(int, int)>,
    candidate: int, index: int,
)
    requires
        chunks.len() == view_chunks.len(),
        0 <= candidate < chunks.len(),
        // All chunks after candidate have offset > index
        forall|k: int| candidate as int + 1 <= k < chunks.len()
            ==> (#[trigger] chunks[k]).offset as int > index,
        // Candidate's offset ≤ index (from binary search)
        chunks[candidate].offset as int <= index,
        // Candidate doesn't cover index
        !(chunks[candidate].offset as int <= index
          && index < chunks[candidate].offset as int + chunks[candidate].bitmap@.num_bits),
        // Sorted non-overlapping
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        // View matches concrete
        forall|k: int| #![auto] 0 <= k < chunks.len() ==> {
            &&& view_chunks[k].0 == chunks[k].offset as int
            &&& view_chunks[k].1 == chunks[k].bitmap@.num_bits
        },
    ensures
        forall|i: int|
            #![trigger view_chunks[i]]
            0 <= i < view_chunks.len()
            ==> !(view_chunks[i].0 <= index
                  && index < view_chunks[i].0 + view_chunks[i].1),
{
    assert forall|i: int|
        #![trigger view_chunks[i]]
        0 <= i < view_chunks.len()
        implies !(view_chunks[i].0 <= index
                  && index < view_chunks[i].0 + view_chunks[i].1)
    by {
        assert(view_chunks[i].0 == chunks[i].offset as int);
        assert(view_chunks[i].1 == chunks[i].bitmap@.num_bits);
        if i < candidate {
            assert(chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[candidate].offset as int);
        }
    }
}

/// If chunks[first..=last] form a touching chain, every index in the range
/// [chunks[first].0, chunks[last].0 + chunks[last].1) is covered by some chunk.
proof fn lemma_touching_chain_coverage(
    view: SparseBitmapView,
    first: int,
    last: int,
)
    requires
        view.wf(),
        0 <= first,
        first <= last,
        last < view.chunks.len(),
        forall|i: int| first <= i < last ==>
            view.chunks[i].0 + view.chunks[i].1
                == (#[trigger] view.chunks[(i + 1) as int]).0,
    ensures
        forall|k: int|
            view.chunks[first].0 <= k < view.chunks[last].0 + view.chunks[last].1
            ==> view.is_covered(k),
    decreases last - first,
{
    if first == last {
        assert forall|k: int|
            view.chunks[first].0 <= k < view.chunks[first].0 + view.chunks[first].1
            implies view.is_covered(k)
        by {};
    } else {
        lemma_touching_chain_coverage(view, first + 1, last);
        assert forall|k: int|
            view.chunks[first].0 <= k < view.chunks[last].0 + view.chunks[last].1
            implies view.is_covered(k)
        by {
            if k < view.chunks[first].0 + view.chunks[first].1 {
                // Covered by chunk first
            }
            // Otherwise: k >= first_end == chunks[first+1].0 (touching)
            // By IH, covered by some chunk in [first+1, last]
        };
    }
}

/// Recursive proof: bits in the follow-chunk free prefixes are not in
/// lifted_set_bits. Covers range [base, base + seq_sum_from(pfp, pfp_idx)).
///
/// base must equal chunks[entry + 1 + pfp_idx].offset for the first
/// non-vacuous call (pfp_idx < pfp.len()).
proof fn lemma_follow_chunks_not_in_set_bits(
    chunks: Seq<Chunk>,
    pfp: Seq<int>,
    entry: int,
    pfp_idx: int,
    base: int,
)
    requires
        0 <= entry,
        entry + 1 + pfp.len() <= chunks.len(),
        0 <= pfp_idx,
        pfp_idx <= pfp.len(),
        // base matches the current chunk's offset (when not past end)
        pfp_idx < pfp.len() ==>
            base == chunks[(entry + 1 + pfp_idx) as int].offset as int,
        // Touching chain
        forall|i: int| entry as int <= i < entry + pfp.len() ==>
            chunks[i].offset as int + chunks[i].bitmap@.num_bits
                == (#[trigger] chunks[(i + 1) as int]).offset as int,
        // pfp bounds and free prefix bits
        forall|i: int| 0 <= i < pfp.len() ==> {
            let ci = entry + 1 + i;
            &&& 0 < (#[trigger] pfp[i]) <= chunks[ci].bitmap@.num_bits
            &&& forall|b: int| 0 <= b < pfp[i]
                ==> !chunks[ci].bitmap@.set_bits.contains(b)
        },
        // Full capacity or tail is zero
        forall|i: int| 0 <= i < pfp.len() ==> (
            (#[trigger] pfp[i]) == chunks[(entry + 1 + i) as int].bitmap@.num_bits
            || seq_sum_from(pfp, i + 1) == 0
        ),
        // Structural properties (for lemma_lifted_set_bits_not_contains_chunk)
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= usize::MAX as int,
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
    ensures
        forall|k: int| base <= k < base + seq_sum_from(pfp, pfp_idx)
            ==> !lifted_set_bits(chunks, 0).contains(k),
    decreases pfp.len() - pfp_idx,
{
    lemma_seq_sum_from_nonneg(pfp, pfp_idx);
    if pfp_idx >= pfp.len() {
        // seq_sum_from == 0, range is empty, vacuously true
    } else {
        let ci = entry + 1 + pfp_idx;
        let take = pfp[pfp_idx];
        lemma_seq_sum_from_unfold(pfp, pfp_idx);

        // This chunk's bits [0, take) are free → global [base, base+take) not in set_bits
        assert forall|k: int| base <= k < base + take
            implies !lifted_set_bits(chunks, 0).contains(k)
        by {
            let local = k - chunks[ci].offset as int;
            assert(0 <= local && local < take);
            assert(local < chunks[ci].bitmap@.num_bits);
            assert(!chunks[ci].bitmap@.set_bits.contains(local));
            lemma_lifted_set_bits_not_contains_chunk(chunks, 0, ci, local);
        };

        // Recurse for remaining chunks
        if pfp_idx + 1 < pfp.len() {
            // take == full capacity (from full-or-tail invariant, since there's a next entry)
            // touching: base + take == chunks[ci].offset + num_bits == chunks[ci+1].offset
            if pfp[pfp_idx] == chunks[ci].bitmap@.num_bits {
                // Full capacity, touching gives next offset
                assert(base + take == chunks[(ci + 1) as int].offset as int);
                lemma_follow_chunks_not_in_set_bits(chunks, pfp, entry, pfp_idx + 1, base + take);
            } else {
                // Not full: must be last meaningful entry, seq_sum_from(pfp, pfp_idx+1) == 0
                lemma_seq_sum_from_nonneg(pfp, pfp_idx + 1);
                // sum == take + 0, so range is just [base, base+take), already proved
            }
        }

        // Combine: range [base, base + sum(pfp, pfp_idx)) = [base, base+take) ∪ [base+take, ...)
        assert forall|k: int| base <= k < base + seq_sum_from(pfp, pfp_idx)
            implies !lifted_set_bits(chunks, 0).contains(k)
        by {
            if k < base + take {
            } else {
                // From recursive call or vacuous (sum_from(pfp, pfp_idx+1) == 0)
            }
        };
    }
}

/// Transfer `is_covered` across views whose `chunks` are extensionally equal.
/// The SMT solver cannot automatically transfer existentials; this lemma
/// provides the concrete witness from `v1` and asserts it for `v2`.
proof fn lemma_is_covered_transfer(v1: SparseBitmapView, v2: SparseBitmapView, k: int)
    requires
        v1.chunks =~= v2.chunks,
        v1.is_covered(k),
    ensures
        v2.is_covered(k),
{
    let i = choose|i: int|
        #![trigger v1.chunks[i]]
        0 <= i < v1.chunks.len()
        && v1.chunks[i].0 <= k
        && k < v1.chunks[i].0 + v1.chunks[i].1;
    assert(v2.chunks[i] == v1.chunks[i]);
}

/// Inserting the endpoint of a range_set extends the range by one.
/// range_set(a, b).insert(b) =~= range_set(a, b + 1)  when a <= b.
proof fn lemma_range_set_insert_end(a: int, b: int)
    requires a <= b,
    ensures BitmapView::range_set(a, b).insert(b) =~= BitmapView::range_set(a, b + 1),
{
    assert forall|x: int|
        BitmapView::range_set(a, b).insert(b).contains(x)
            == BitmapView::range_set(a, b + 1).contains(x)
    by {}
}

/// Union of two contiguous range_sets is a single range_set.
/// range_set(a, b) ∪ range_set(b, c) =~= range_set(a, c)  when a <= b <= c.
proof fn lemma_range_set_union_contiguous(a: int, b: int, c: int)
    requires a <= b, b <= c,
    ensures
        BitmapView::range_set(a, b).union(BitmapView::range_set(b, c))
            =~= BitmapView::range_set(a, c),
{
    assert forall|x: int| #![auto]
        BitmapView::range_set(a, b).union(BitmapView::range_set(b, c)).contains(x)
            == BitmapView::range_set(a, c).contains(x)
    by {}
}

//==================================================================================================
// Decomposition lemma for exists_contiguous_free_range
//==================================================================================================

/// If a contiguous free range of `count` bits exists, then there is some chunk ci
/// such that either the range fits entirely within ci (single-chunk) or starts in ci
/// and extends past it (cross-chunk).
proof fn lemma_exists_range_decomposes(view: SparseBitmapView, count: int)
    requires
        view.wf(),
        count > 0,
        view.exists_contiguous_free_range(count),
    ensures
        exists|ci: int|
            #![trigger view.chunks[ci]]
            0 <= ci < view.chunks.len() && (
                view.has_single_chunk_free_range(ci, count)
                || view.has_cross_chunk_free_range_from(ci, count)
            ),
{
    // Witness: start such that has_contiguous_free_range(start, count)
    let start = choose|start: int|
        #![trigger view.has_contiguous_free_range(start, count)]
        view.has_contiguous_free_range(start, count);

    // start is covered, so some chunk ci contains it
    assert(view.is_covered(start));
    let ci = choose|i: int|
        #![trigger view.chunks[i]]
        0 <= i < view.chunks.len()
        && view.chunks[i].0 <= start
        && start < view.chunks[i].0 + view.chunks[i].1;

    let chunk_end = view.chunks[ci].0 + view.chunks[ci].1;

    if start + count <= chunk_end {
        // Single-chunk case
        assert(view.has_single_chunk_free_range(ci, count));
    } else {
        // Cross-chunk case
        assert(view.has_cross_chunk_free_range_from(ci, count));
    }
}

/// Bridge: if Bitmap chunk ci has no contiguous free range of size `count`,
/// then the SparseBitmapView has no single-chunk free range in chunk ci.
///
/// The Bitmap's `!exists_contiguous_free_range(count)` says no `count`-length
/// run of unset bits exists anywhere in [0, num_bits). This translates directly:
/// any single-chunk range [start, start+count) ⊆ chunk ci would require
/// the local range [start - offset, start - offset + count) to be all-unset in the bitmap,
/// contradicting the Bitmap's negative fact.
proof fn lemma_bitmap_no_range_implies_no_single_chunk(
    view: SparseBitmapView,
    chunks: Seq<Chunk>,
    ci: int,
    count: int,
)
    requires
        view.wf(),
        count > 0,
        0 <= ci < view.chunks.len(),
        chunks.len() == view.chunks.len(),
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.set_bits.finite(),
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        forall|i: int| #![auto] 0 <= i < chunks.len() ==> {
            &&& chunks[i].offset as int == view.chunks[i].0
            &&& chunks[i].bitmap@.num_bits == view.chunks[i].1
        },
        count <= chunks[ci].bitmap@.num_bits,
        !chunks[ci].bitmap@.exists_contiguous_free_range(count),
        view.set_bits =~= lifted_set_bits(chunks, 0),
    ensures
        !view.has_single_chunk_free_range(ci, count),
{
    if view.has_single_chunk_free_range(ci, count) {
        // Get the witness start
        let start = choose|start: int|
            #![trigger view.has_contiguous_free_range(start, count)]
            view.chunks[ci].0 <= start
            && start + count <= view.chunks[ci].0 + view.chunks[ci].1
            && view.has_contiguous_free_range(start, count);

        let offset = view.chunks[ci].0;
        let local_start = start - offset;

        // All bits in [start, start+count) are free in view
        // → all local bits [local_start, local_start+count) are free in bitmap
        assert(0 <= local_start);
        assert(local_start + count <= chunks[ci].bitmap@.num_bits);

        // Show: the bitmap has all bits unset in this local range
        assert forall|j: int| local_start <= j < local_start + count
            implies !chunks[ci].bitmap@.set_bits.contains(j)
        by {
            let global = j + offset;
            assert(start <= global && global < start + count);
            assert(!view.set_bits.contains(global));
            // global not in lifted_set_bits → local j not in bitmap's set_bits
            lemma_lifted_set_bits_not_contains_implies_chunk(
                chunks, 0, ci, j);
        };

        // So bitmap has a contiguous free range — contradiction
        assert(chunks[ci].bitmap@.has_free_range_at(local_start, count));
        assert(chunks[ci].bitmap@.exists_contiguous_free_range(count));
    }
}

/// If `global` is NOT in `lifted_set_bits(chunks, 0)` and `global = chunks[ci].offset + local`
/// with `0 <= local < chunks[ci].bitmap@.num_bits`, then `local` is NOT in
/// `chunks[ci].bitmap@.set_bits`.
///
/// Contrapositive of `lemma_lifted_set_bits_contains_chunk`.
proof fn lemma_lifted_set_bits_not_contains_implies_chunk(
    chunks: Seq<Chunk>,
    idx: int,
    ci: int,
    local: int,
)
    requires
        idx >= 0,
        idx <= ci,
        ci < chunks.len(),
        0 <= local < chunks[ci].bitmap@.num_bits,
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.set_bits.finite(),
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        !lifted_set_bits(chunks, idx).contains(local + chunks[ci].offset as int),
    ensures
        !chunks[ci].bitmap@.set_bits.contains(local),
{
    // Contrapositive: if chunks[ci].set_bits contains local,
    // then lifted_set_bits contains local + offset
    if chunks[ci].bitmap@.set_bits.contains(local) {
        lemma_lifted_set_bits_contains_chunk(chunks, idx, ci, local);
        assert(false);
    }
}

/// If `count` exceeds chunk ci's capacity, no single-chunk range of that size exists there.
proof fn lemma_too_large_implies_no_single_chunk(
    view: SparseBitmapView,
    ci: int,
    count: int,
)
    requires
        view.wf(),
        count > 0,
        0 <= ci < view.chunks.len(),
        count > view.chunks[ci].1,
    ensures
        !view.has_single_chunk_free_range(ci, count),
{
    if view.has_single_chunk_free_range(ci, count) {
        let start = choose|start: int| {
            &&& view.chunks[ci].0 <= start
            &&& start + count <= view.chunks[ci].0 + view.chunks[ci].1
            &&& view.has_contiguous_free_range(start, count)
        };
        // start + count <= chunks[ci].0 + chunks[ci].1
        // start >= chunks[ci].0
        // → count <= chunks[ci].1
        // But count > chunks[ci].1. Contradiction.
        assert(false);
    }
}

/// If the walk from entry found no viable extension (uncovered or set index
/// blocks the range), no cross-chunk free range from entry exists.
///
/// `problem_idx` is the global index that blocks the range:
///   - If `set_bits.contains(problem_idx)`: a set bit blocks the range
///
/// The lemma proves that for ANY start in entry that extends past entry,
/// `problem_idx` falls within [start, start+count), yielding a contradiction.
proof fn lemma_walk_fail_no_cross_range(
    view: SparseBitmapView,
    chunks: Seq<Chunk>,
    entry: int,
    count: int,
    trailing_free: int,
    entry_cap: int,
    need: int,
    sum_pfp: int,
    last_chunk_end: int,
    problem_idx: int,
)
    requires
        view.wf(),
        count > 0,
        count > trailing_free,
        0 <= trailing_free <= entry_cap,
        0 <= entry < view.chunks.len(),
        chunks.len() == view.chunks.len(),
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.set_bits.finite(),
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        forall|i: int| #![auto] 0 <= i < chunks.len() ==> {
            &&& chunks[i].offset as int == view.chunks[i].0
            &&& chunks[i].bitmap@.num_bits == view.chunks[i].1
        },
        view.set_bits =~= lifted_set_bits(chunks, 0),
        entry_cap == chunks[entry].bitmap@.num_bits,
        // Trailing-free loop fact: if trailing_free < entry_cap, bit below is set
        trailing_free < entry_cap ==>
            chunks[entry].bitmap@.set_bits.contains(entry_cap - trailing_free - 1),
        // Sum invariant: need + sum(pfp) == count - trailing_free
        need > 0,
        sum_pfp >= 0,
        need + sum_pfp == count - trailing_free,
        // Tight sum-bounds: chunk_end(entry) + sum(pfp) == last_chunk_end
        last_chunk_end == chunks[entry].offset as int + entry_cap + sum_pfp,
        // problem_idx is at or beyond last_chunk_end
        problem_idx >= last_chunk_end,
        // problem_idx is close enough: within need of last_chunk_end
        problem_idx < last_chunk_end + need,
        // problem_idx is either uncovered or in set_bits (or both)
        !view.is_covered(problem_idx) || view.set_bits.contains(problem_idx),
    ensures
        !view.has_cross_chunk_free_range_from(entry, count),
{
    if view.has_cross_chunk_free_range_from(entry, count) {
        let start = choose|start: int| {
            &&& view.chunks[entry].0 <= start
            &&& start < view.chunks[entry].0 + view.chunks[entry].1
            &&& start + count > view.chunks[entry].0 + view.chunks[entry].1
            &&& view.has_contiguous_free_range(start, count)
        };

        // start >= chunk_end(entry) - trailing_free
        // Inline argument from lemma_cross_chunk_range_needs_follow_chunks:
        if trailing_free < entry_cap {
            let set_local = entry_cap - trailing_free - 1;
            let set_global = chunks[entry].offset as int + set_local;
            assert(chunks[entry].bitmap@.set_bits.contains(set_local));
            lemma_lifted_set_bits_contains_chunk(chunks, 0, entry, set_local);
            assert(view.set_bits.contains(set_global));
            if start <= set_global {
                assert(start <= set_global);
                assert(set_global < start + count);
                assert(!view.set_bits.contains(set_global));
                assert(false);
            }
        }
        assert(start >= chunks[entry].offset as int + entry_cap - trailing_free);

        // Show problem_idx is in [start, start+count):
        // Lower bound: problem_idx >= last_chunk_end >= chunk_end(entry) > start
        assert(start < chunks[entry].offset as int + entry_cap);
        // sum_pfp >= 0 from precondition
        assert(last_chunk_end >= chunks[entry].offset as int + entry_cap);
        assert(problem_idx >= last_chunk_end);
        assert(problem_idx >= start);

        // Upper bound: start + count >= chunk_end(entry) - trailing_free + count
        //   = chunk_end(entry) + (count - trailing_free)
        //   = chunk_end(entry) + need + sum_pfp
        //   = last_chunk_end + need
        //   > problem_idx
        assert(start + count >= chunks[entry].offset as int + entry_cap
            - trailing_free + count);
        assert(chunks[entry].offset as int + entry_cap - trailing_free + count
            == chunks[entry].offset as int + entry_cap + (count - trailing_free));
        assert(count - trailing_free == need + sum_pfp);
        assert(chunks[entry].offset as int + entry_cap + need + sum_pfp
            == last_chunk_end + need);
        assert(start + count >= last_chunk_end + need);
        assert(problem_idx < last_chunk_end + need);
        assert(problem_idx < start + count);

        // problem_idx is in [start, start+count)
        // has_contiguous_free_range gives: is_covered(problem_idx) AND !set_bits.contains(problem_idx)
        assert(view.has_contiguous_free_range(start, count));
        assert(view.is_covered(problem_idx));
        assert(!view.set_bits.contains(problem_idx));
        // But precondition: !is_covered(problem_idx) || set_bits.contains(problem_idx)
        // Either way: contradiction
        assert(false);
    }
}

//==================================================================================================
// Proof Extraction: alloc_range helpers
//==================================================================================================

/// After exhaustive search over all chunks, neither single-chunk nor cross-chunk
/// free ranges exist, so no contiguous free range exists at all.
proof fn lemma_exhaustive_search_no_free_range(view: SparseBitmapView, count: int)
    requires
        view.wf(),
        count > 0,
        view.chunks.len() > 0,
        forall|k: int| 0 <= k < view.chunks.len()
            ==> !view.has_single_chunk_free_range(k, count),
        forall|k: int| 0 <= k < view.chunks.len()
            ==> !view.has_cross_chunk_free_range_from(k, count),
    ensures
        !view.exists_contiguous_free_range(count),
{
    if view.exists_contiguous_free_range(count) {
        lemma_exists_range_decomposes(view, count);
        let ci = choose|ci: int|
            #![trigger view.chunks[ci]]
            0 <= ci < view.chunks.len() && (
                view.has_single_chunk_free_range(ci, count)
                || view.has_cross_chunk_free_range_from(ci, count)
            );
        assert(!view.has_single_chunk_free_range(ci, count));
        assert(!view.has_cross_chunk_free_range_from(ci, count));
        assert(false);
    }
}

/// When trailing_free == 0, the last bit in the chunk is set, so no cross-chunk
/// free range can start from this chunk (any such range would include that set bit).
proof fn lemma_trailing_zero_no_cross_range(
    view: SparseBitmapView,
    chunks: Seq<Chunk>,
    entry: int,
    count: int,
)
    requires
        view.wf(),
        count > 0,
        0 <= entry < chunks.len(),
        entry < view.chunks.len(),
        chunks[entry].bitmap@.num_bits > 0,
        chunks[entry].bitmap@.set_bits.contains(chunks[entry].bitmap@.num_bits - 1),
        view.chunks[entry].0 == chunks[entry].offset as int,
        view.chunks[entry].1 == chunks[entry].bitmap@.num_bits,
        view.set_bits =~= lifted_set_bits(chunks, 0),
    ensures
        !view.has_cross_chunk_free_range_from(entry, count),
{
    let entry_cap = chunks[entry].bitmap@.num_bits;
    let last_local = entry_cap - 1;
    let last_global = chunks[entry].offset as int + last_local;
    lemma_lifted_set_bits_contains_chunk(chunks, 0, entry, last_local);
    assert(view.set_bits.contains(last_global));
    if view.has_cross_chunk_free_range_from(entry, count) {
        let start = choose|start: int| {
            &&& view.chunks[entry].0 <= start
            &&& start < view.chunks[entry].0 + view.chunks[entry].1
            &&& start + count > view.chunks[entry].0 + view.chunks[entry].1
            &&& view.has_contiguous_free_range(start, count)
        };
        assert(start <= last_global);
        assert(last_global < start + count);
        assert(!view.set_bits.contains(last_global));
        assert(false);
    }
}

//==================================================================================================
// Proof Extraction: try_alloc_cross_chunk_from helpers
//==================================================================================================

/// Walk-fail gap case: when the walk reaches a gap after the last chunk
/// (either because last_chunk is the final chunk, or the next chunk doesn't
/// touch it), proves that no cross-chunk free range starts at entry.
/// Combines `lemma_no_chunk_covers_with_gap` and `lemma_walk_fail_no_cross_range`.
proof fn lemma_walk_fail_gap_case(
    view: SparseBitmapView,
    chunks: Seq<Chunk>,
    entry: int,
    count: int,
    trailing_free: int,
    entry_cap: int,
    need: int,
    sum_pfp: int,
    last_chunk: int,
)
    requires
        view.wf(),
        count > 0,
        count > trailing_free,
        0 <= trailing_free <= entry_cap,
        0 <= entry < chunks.len(),
        entry <= last_chunk,
        0 <= last_chunk < chunks.len(),
        chunks.len() == view.chunks.len(),
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.set_bits.finite(),
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        forall|i: int| #![auto] 0 <= i < chunks.len() ==> {
            &&& chunks[i].offset as int == view.chunks[i].0
            &&& chunks[i].bitmap@.num_bits == view.chunks[i].1
        },
        view.set_bits =~= lifted_set_bits(chunks, 0),
        entry_cap == chunks[entry].bitmap@.num_bits,
        trailing_free < entry_cap ==>
            chunks[entry].bitmap@.set_bits.contains(entry_cap - trailing_free - 1),
        need > 0,
        sum_pfp >= 0,
        need + sum_pfp == count - trailing_free,
        // Tight sum-bounds
        chunks[entry].offset as int + entry_cap + sum_pfp
            == chunks[last_chunk].offset as int + chunks[last_chunk].bitmap@.num_bits,
        // Gap: all chunks after last_chunk start beyond its end
        forall|k: int| last_chunk + 1 <= k < chunks.len()
            ==> (#[trigger] chunks[k]).offset as int
                > chunks[last_chunk].offset as int + chunks[last_chunk].bitmap@.num_bits,
    ensures
        !view.has_cross_chunk_free_range_from(entry, count),
{
    let lce = chunks[last_chunk].offset as int + chunks[last_chunk].bitmap@.num_bits;
    let last_chunk_end = chunks[entry].offset as int + entry_cap + sum_pfp;
    assert(last_chunk_end == lce);
    lemma_no_chunk_covers_with_gap(chunks, view.chunks, last_chunk, lce);
    lemma_walk_fail_no_cross_range(
        view, chunks, entry, count, trailing_free, entry_cap,
        need, sum_pfp, last_chunk_end, lce);
}

/// Walk-fail set-bit case: when the inner check-walk finds a set bit in
/// a follow chunk, proves that no cross-chunk free range starts at entry.
/// Combines `lemma_lifted_set_bits_contains_chunk` and
/// `lemma_walk_fail_no_cross_range`.
proof fn lemma_walk_fail_set_bit_case(
    view: SparseBitmapView,
    chunks: Seq<Chunk>,
    entry: int,
    count: int,
    trailing_free: int,
    entry_cap: int,
    need: int,
    sum_pfp: int,
    last_chunk: int,
    next: int,
    check_bit: int,
)
    requires
        view.wf(),
        count > 0,
        count > trailing_free,
        0 <= trailing_free <= entry_cap,
        0 <= entry < chunks.len(),
        entry <= last_chunk,
        0 <= last_chunk < chunks.len(),
        0 <= next < chunks.len(),
        chunks.len() == view.chunks.len(),
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.set_bits.finite(),
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        forall|i: int| #![auto] 0 <= i < chunks.len() ==> {
            &&& chunks[i].offset as int == view.chunks[i].0
            &&& chunks[i].bitmap@.num_bits == view.chunks[i].1
        },
        view.set_bits =~= lifted_set_bits(chunks, 0),
        entry_cap == chunks[entry].bitmap@.num_bits,
        trailing_free < entry_cap ==>
            chunks[entry].bitmap@.set_bits.contains(entry_cap - trailing_free - 1),
        need > 0,
        sum_pfp >= 0,
        need + sum_pfp == count - trailing_free,
        // Tight sum-bounds
        chunks[entry].offset as int + entry_cap + sum_pfp
            == chunks[last_chunk].offset as int + chunks[last_chunk].bitmap@.num_bits,
        // next touches last_chunk
        chunks[last_chunk].offset as int + chunks[last_chunk].bitmap@.num_bits
            == chunks[next].offset as int,
        // Set bit found in follow chunk
        0 <= check_bit < chunks[next].bitmap@.num_bits,
        chunks[next].bitmap@.set_bits.contains(check_bit),
        check_bit < need,
    ensures
        !view.has_cross_chunk_free_range_from(entry, count),
{
    let lce = chunks[last_chunk].offset as int + chunks[last_chunk].bitmap@.num_bits;
    let problem_idx = chunks[next].offset as int + check_bit;
    lemma_lifted_set_bits_contains_chunk(chunks, 0, next, check_bit);
    assert(view.set_bits.contains(problem_idx));
    let last_chunk_end = chunks[entry].offset as int + entry_cap + sum_pfp;
    assert(last_chunk_end == lce);
    assert(problem_idx >= last_chunk_end);
    assert(problem_idx < last_chunk_end + need);
    lemma_walk_fail_no_cross_range(
        view, chunks, entry, count, trailing_free, entry_cap,
        need, sum_pfp, last_chunk_end, problem_idx);
}

/// Proves that the cross-chunk range [gs, gs+count) is covered and free,
/// i.e., `view.has_contiguous_free_range(gs, count)`, where
/// `gs = chunks[entry].offset + entry_cap - trailing_free`.
/// Used in `try_alloc_cross_chunk_from` before calling commit.
proof fn lemma_cross_chunk_range_is_free(
    view: SparseBitmapView,
    chunks: Seq<Chunk>,
    pfp: Seq<int>,
    entry: int,
    last_chunk: int,
    entry_cap: int,
    trailing_free: int,
    count: int,
)
    requires
        view.wf(),
        chunks.len() == view.chunks.len(),
        forall|i: int| 0 <= i < chunks.len() ==> (#[trigger] chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].bitmap@.set_bits.finite(),
        forall|i: int, j: int| #![auto] 0 <= i < j < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= chunks[j].offset as int,
        forall|i: int| #![auto] 0 <= i < chunks.len() ==> {
            &&& chunks[i].offset as int == view.chunks[i].0
            &&& chunks[i].bitmap@.num_bits == view.chunks[i].1
        },
        view.set_bits =~= lifted_set_bits(chunks, 0),
        0 <= entry <= last_chunk,
        last_chunk < chunks.len(),
        entry_cap == chunks[entry].bitmap@.num_bits,
        count > 0,
        0 < trailing_free <= entry_cap,
        // Trailing free bits in entry chunk
        forall|b: int| (entry_cap - trailing_free) <= b < entry_cap
            ==> !chunks[entry].bitmap@.set_bits.contains(b),
        // Touching chain
        forall|i: int| entry <= i < last_chunk ==>
            chunks[i].offset as int + chunks[i].bitmap@.num_bits
                == (#[trigger] chunks[(i + 1) as int]).offset as int,
        // Phase 1b free prefixes
        pfp.len() == last_chunk - entry,
        entry + 1 + pfp.len() <= chunks.len(),
        forall|i: int| 0 <= i < pfp.len() ==> {
            let k = entry + 1 + i;
            &&& k < chunks.len()
            &&& 0 < (#[trigger] pfp[i]) <= chunks[k].bitmap@.num_bits
            &&& forall|b: int| 0 <= b < pfp[i]
                ==> !chunks[k].bitmap@.set_bits.contains(b)
        },
        // Each pfp entry: full capacity or remainder is zero
        forall|i: int| 0 <= i < pfp.len() ==> (
            (#[trigger] pfp[i]) == chunks[(entry + 1 + i) as int].bitmap@.num_bits
            || seq_sum_from(pfp, i + 1) == 0
        ),
        // Chunk ranges do not overflow usize
        forall|i: int| #![auto] 0 <= i < chunks.len()
            ==> chunks[i].offset as int + chunks[i].bitmap@.num_bits
                <= usize::MAX as int,
        // Sum conservation
        seq_sum_from(pfp, 0)
            == (if count > trailing_free { count - trailing_free } else { 0int }),
        // Sum-bounds: the range fits within the touching chain
        chunks[entry].offset as int + entry_cap
            + seq_sum_from(pfp, 0)
            <= chunks[last_chunk].offset as int + chunks[last_chunk].bitmap@.num_bits,
    ensures
        view.has_contiguous_free_range(
            chunks[entry].offset as int + entry_cap - trailing_free, count),
{
    let gs = chunks[entry].offset as int + entry_cap - trailing_free;
    let entry_end = chunks[entry].offset as int + entry_cap;

    // Establish touching chain on abstract view chunks
    assert forall|i: int| entry <= i < last_chunk implies
        view.chunks[i].0 + view.chunks[i].1
            == (#[trigger] view.chunks[(i + 1) as int]).0
    by {
        assert(chunks[i].offset as int == view.chunks[i].0);
        assert(chunks[i].bitmap@.num_bits == view.chunks[i].1);
        assert(chunks[(i + 1) as int].offset as int == view.chunks[(i + 1) as int].0);
    };

    if entry < last_chunk {
        lemma_touching_chain_coverage(view, entry, last_chunk);
    }

    assert(view.chunks[entry].0 <= gs);
    assert forall|k: int| gs <= k < gs + count
        implies view.is_covered(k)
    by {
        if entry < last_chunk {
            // k is in [gs, gs+count) ⊆ [view.chunks[entry].0,
            //   view.chunks[last_chunk].0 + view.chunks[last_chunk].1)
            // touching chain coverage provides the witness
        } else {
            // entry == last_chunk: range fits within entry chunk
            assert(view.chunks[entry].0 <= k);
            assert(k < view.chunks[entry].0 + view.chunks[entry].1);
        }
    };

    // Entry chunk trailing bits are not in lifted_set_bits
    assert forall|k: int| gs <= k < entry_end && k < gs + count
        implies !lifted_set_bits(chunks, 0).contains(k)
    by {
        let local = k - chunks[entry].offset as int;
        assert(!chunks[entry].bitmap@.set_bits.contains(local));
        lemma_lifted_set_bits_not_contains_chunk(
            chunks, 0, entry, local);
    };

    // Follow chunk free prefix bits are not in lifted_set_bits
    if entry < last_chunk {
        lemma_follow_chunks_not_in_set_bits(
            chunks, pfp, entry,
            0int, chunks[(entry + 1) as int].offset as int);
    }

    assert forall|k: int| gs <= k < gs + count
        implies !view.set_bits.contains(k)
    by {
        if k < entry_end {} else {}
    };
}

//==================================================================================================
// Single-chunk alloc Ok proof
//==================================================================================================

/// After a successful single-chunk `alloc_range`, re-establishes the sparse
/// bitmap invariant and proves coverage, free-before, and set_bits update.
proof fn lemma_single_chunk_alloc_ok(
    sb: &SparseBitmap,
    old_chunks: Seq<Chunk>,
    old_view: SparseBitmapView,
    pre_alloc_bv: BitmapView,
    idx: int,
    local: int,
    count: int,
)
    requires
        // Chunks sequence is an update at idx
        sb.chunks@ =~= old_chunks.update(idx, sb.chunks@[idx]),
        // Updated chunk preserves offset and num_bits
        sb.chunks@[idx].offset == old_chunks[idx].offset,
        sb.chunks@[idx].bitmap@.num_bits == old_chunks[idx].bitmap@.num_bits,
        sb.chunks@[idx].bitmap.inv(),
        // Updated chunk set_bits = old + range
        sb.chunks@[idx].bitmap@.set_bits =~= pre_alloc_bv.set_bits.union(
            BitmapView::range_set(local, local + count)),
        // pre_alloc matches old chunks[idx]
        pre_alloc_bv =~= old_chunks[idx].bitmap@,
        pre_alloc_bv.set_bits.finite(),
        // Bits were free before alloc
        forall|j: int| local <= j < local + count
            ==> !pre_alloc_bv.is_bit_set(j),
        // Old state structural properties
        old_view.wf(),
        old_view.chunks.len() == old_chunks.len(),
        forall|k: int| #![auto] 0 <= k < old_chunks.len() ==> {
            &&& old_view.chunks[k].0 == old_chunks[k].offset as int
            &&& old_view.chunks[k].1 == old_chunks[k].bitmap@.num_bits
        },
        old_view.set_bits =~= lifted_set_bits(old_chunks, 0),
        // Structural
        0 <= idx < old_chunks.len(),
        old_chunks.len() > 0,
        forall|i: int| 0 <= i < old_chunks.len()
            ==> (#[trigger] old_chunks[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].bitmap@.set_bits.finite(),
        forall|i: int| #![auto] 0 <= i < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= usize::MAX as int,
        forall|i: int, j: int| #![auto] 0 <= i < j < old_chunks.len()
            ==> old_chunks[i].offset as int + old_chunks[i].bitmap@.num_bits
                <= old_chunks[j].offset as int,
        sb.capacity_bits as int == old_view.capacity(),
        (sb.next_chunk_hint as int) < sb.chunks@.len(),
        // Alloc bounds
        0 <= local,
        local + count <= old_chunks[idx].bitmap@.num_bits,
        count > 0,
    ensures
        sb.inv(),
        sb@.chunks =~= old_view.chunks,
        sb@.set_bits =~= old_view.set_bits.union(
            BitmapView::range_set(
                old_chunks[idx].offset as int + local,
                old_chunks[idx].offset as int + local + count)),
        forall|k: int|
            old_chunks[idx].offset as int + local <= k
                < old_chunks[idx].offset as int + local + count
            ==> old_view.is_covered(k),
        forall|k: int|
            old_chunks[idx].offset as int + local <= k
                < old_chunks[idx].offset as int + local + count
            ==> !old_view.is_bit_set(k),
{
    // Prove set_bits.finite() for updated chunk
    assert(sb.chunks@[idx].bitmap@.set_bits.finite()) by {
        pre_alloc_bv.lemma_set_bits_finite();
        BitmapView::lemma_range_set_finite(local, local + count);
    }
    // Prove bits in range were free in old_chunks
    assert forall|j: int| local <= j < local + count
        implies !old_chunks[idx].bitmap@.set_bits.contains(j)
    by {
        assert(!pre_alloc_bv.is_bit_set(j));
    }
    // Update lifted_set_bits
    lemma_lifted_set_bits_alloc_range(
        old_chunks, sb.chunks@, idx, local, count);
    // Prove coverage
    let result_offset = old_chunks[idx].offset as int + local;
    assert forall|k: int|
        result_offset <= k < result_offset + count
        implies old_view.is_covered(k)
    by {
        assert(old_view.chunks[idx].0 <= k);
        assert(k < old_view.chunks[idx].0 + old_view.chunks[idx].1);
    }
    // Prove free-before
    assert forall|k: int|
        result_offset <= k < result_offset + count
        implies !old_view.is_bit_set(k)
    by {
        let i = k - old_chunks[idx].offset as int;
        assert(!old_chunks[idx].bitmap@.set_bits.contains(i));
        lemma_lifted_set_bits_not_contains_chunk(
            old_chunks, 0, idx, i);
    }
    // Re-establish invariant
    lemma_inv_after_chunk_update(sb, old_chunks, old_view, idx);
}

//==================================================================================================
// Constructor postcondition proof
//==================================================================================================

/// After constructing a SparseBitmap, proves the invariant holds and the
/// view matches the expected capacity, set_bits, and chunk_count.
proof fn lemma_constructor_postcondition(
    sb: &SparseBitmap,
    expected_capacity: int,
    expected_set_bits: Set<int>,
    expected_chunk_count: int,
)
    requires
        // Chunks are sorted and non-overlapping (consecutive)
        forall|i: int, j: int| #![auto] 0 <= i < j < sb.chunks@.len()
            ==> sb.chunks@[i].offset as int + sb.chunks@[i].bitmap@.num_bits
                <= sb.chunks@[j].offset as int,
        // Per-chunk properties (from constructor loop invariants)
        sb.chunks@.len() > 0,
        forall|i: int| 0 <= i < sb.chunks@.len()
            ==> (#[trigger] sb.chunks@[i]).bitmap.inv(),
        forall|i: int| #![auto] 0 <= i < sb.chunks@.len()
            ==> sb.chunks@[i].bitmap@.num_bits > 0,
        forall|i: int| #![auto] 0 <= i < sb.chunks@.len()
            ==> sb.chunks@[i].bitmap@.set_bits.finite(),
        forall|i: int| #![auto] 0 <= i < sb.chunks@.len()
            ==> sb.chunks@[i].offset as int + sb.chunks@[i].bitmap@.num_bits
                <= usize::MAX as int,
        (sb.next_chunk_hint as int) < sb.chunks@.len(),
        // Capacity matches expectation
        chunk_seq_capacity(sb.chunks@, 0) == expected_capacity,
        sb.capacity_bits as int == expected_capacity,
        // Set bits match expectation
        lifted_set_bits(sb.chunks@, 0) =~= expected_set_bits,
        // Chunk count matches expectation
        sb.chunks@.len() == expected_chunk_count,
    ensures
        sb.inv(),
        sb@.capacity() == expected_capacity,
        sb@.set_bits =~= expected_set_bits,
        sb@.chunk_count() == expected_chunk_count,
{
    lemma_nonoverlap_from_consecutive(sb.chunks@);
    assert forall|k: int| #![auto] 0 <= k < sb.chunks@.len() implies {
        &&& sb@.chunks[k].0 == sb.chunks@[k].offset as int
        &&& sb@.chunks[k].1 == sb.chunks@[k].bitmap@.num_bits
    } by { let _ = sb.chunks@[k]; }
    lemma_chunk_seq_capacity_eq_capacity_from(sb.chunks@, sb@, 0);
    SparseBitmap::lemma_new_establishes_inv(sb);
}

//==================================================================================================
// Circular scan exhaustion proof
//==================================================================================================

/// After a circular scan visits all `n` chunks, `has_wrapped` must be true.
proof fn lemma_circular_scan_exhausts(
    has_wrapped: bool,
    idx: int,
    start_hint: int,
    visited: int,
    n: int,
)
    requires
        visited == n,
        n > 0,
        0 <= idx < n,
        0 <= start_hint < n,
        !has_wrapped ==> idx == start_hint + visited,
        has_wrapped ==> idx == start_hint + visited - n,
    ensures
        has_wrapped,
        idx == start_hint,
{
    if !has_wrapped {
        assert(idx == start_hint + visited);
        assert(idx == start_hint + n);
        assert(false);
    }
}

} // verus!
