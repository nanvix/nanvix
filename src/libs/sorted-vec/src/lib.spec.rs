verus! {

// ===========================================================================
// View
// ===========================================================================

impl<T: Ord> View for SortedVec<T> {
    type V = Seq<T>;

    closed spec fn view(&self) -> Seq<T> {
        self.inner@
    }
}

// ===========================================================================
// Spec Helpers — built on vstd's OrdSpec::cmp_spec
// ===========================================================================


/// Strict sorted: all pairs in ascending order via cmp_spec.
pub open spec fn spec_strictly_sorted<T: Ord>(s: Seq<T>) -> bool {
    forall|i: int, j: int| #![trigger s[i], s[j]]
        0 <= i < j < s.len() ==> s[i].cmp_spec(&s[j]) is Less
}

/// Membership via Ord-equality.
pub open spec fn spec_contains<T: Ord>(s: Seq<T>, v: T) -> bool {
    exists|i: int| #![auto] 0 <= i < s.len() && s[i].cmp_spec(&v) is Equal
}

// ===========================================================================
// Invariant
// ===========================================================================

impl<T: Ord> SortedVec<T> {
    pub open spec fn inv(&self) -> bool {
        &&& spec_strictly_sorted(self@)
        &&& self@.len() <= usize::MAX as int
    }
}

// ===========================================================================
// Assume Specifications
// ===========================================================================

// binary_search: requires obeys_cmp_spec (vstd ordering laws).
// No more axiom-bundling in ensures — axioms come from vstd.
pub assume_specification<T: Ord>[ <[T]>::binary_search ](
    slice: &[T],
    value: &T,
) -> (result: Result<usize, usize>)
    requires
        spec_strictly_sorted(slice@),
        obeys_cmp_spec::<T>(),
    ensures
        match result {
            Ok(idx) => {
                &&& (idx as int) < slice@.len()
                &&& slice@[idx as int].cmp_spec(value) is Equal
            },
            Err(idx) => {
                &&& (idx as int) <= slice@.len()
                &&& !spec_contains(slice@, *value)
                &&& forall|k: int| #![auto] 0 <= k < idx as int
                    ==> slice@[k].cmp_spec(value) is Less
                &&& forall|k: int| #![auto] idx as int <= k < slice@.len()
                    ==> value.cmp_spec(&slice@[k]) is Less
            },
        },
;

// binary_search_by_key
pub open spec fn maps_to_key<'a, T, B: Eq, F: FnMut(&'a T) -> B>(f: F, n: &'a T, key: &B) -> bool
{
    exists|x: B| #[trigger] f.ensures((n,), x) && x.eq_spec(key)
}

pub open spec fn map_key_cmp<'a, T, B: Ord, F: FnMut(&'a T) -> B>(
    f: F, n: &'a T, key: &B, order: core::cmp::Ordering,
) -> bool
{
    exists|x: B| #[trigger] f.ensures((n,), x) && x.cmp_spec(key) == order
}

pub assume_specification<'a, T, B: Ord, F: FnMut(&'a T) -> B>[ <[T]>::binary_search_by_key ](
    slice: &'a [T],
    key: &B,
    f: F,
) -> (result: Result<usize, usize>)
    requires
        obeys_cmp_spec::<B>(),
        forall|i: int, j: int, x: B, y: B| {
            &&& 0 <= i < j < slice@.len()
            &&& f.ensures((&slice[i], ), x)
            &&& f.ensures((&slice[j], ), y)
        } ==> !(x.cmp_spec(&y) is Greater),
    ensures
        match result {
            Ok(index) => {
                &&& 0 <= index < slice@.len()
                &&& maps_to_key(f, &slice[index as int], key)
            },
            Err(index) => {
                &&& 0 <= index <= slice@.len()
                &&& forall|i: int| #![trigger slice[i]]
                    0 <= i < index ==> map_key_cmp(f, &slice[i], key, core::cmp::Ordering::Less)
                &&& forall|i: int| #![trigger slice[i]]
                    index <= i < slice@.len() ==> map_key_cmp(f, &slice[i], key, core::cmp::Ordering::Greater)
            },
        },
;

// sort_unstable
pub assume_specification<T: Ord>[ <[T]>::sort_unstable ](slice: &mut [T])
    ensures
        slice@.len() == old(slice)@.len(),
        forall|v: T| slice@.contains(v) <==> old(slice)@.contains(v),
        forall|i: int, j: int| #![trigger slice@[i], slice@[j]]
            0 <= i < j < slice@.len() ==> !(slice@[j].cmp_spec(&slice@[i]) is Less),
;

// dedup: preserves order (subsequence property) + no consecutive eq_spec equal
pub assume_specification<T: PartialEq, A: core::alloc::Allocator>[ Vec::<T, A>::dedup ](vec: &mut Vec<T, A>)
    ensures
        vec@.len() <= old(vec)@.len(),
        forall|v: T| vec@.contains(v) ==> old(vec)@.contains(v),
        forall|i: int| 0 <= i < vec@.len() - 1
            ==> !(#[trigger] vec@[i]).eq_spec(&vec@[i + 1]),
        // Order preservation: pairwise
        forall|i: int, j: int| #![trigger vec@[i], vec@[j]]
            0 <= i < j < vec@.len() ==> (
            exists|ki: int, kj: int|
                0 <= ki < kj < old(vec)@.len()
                && vec@[i] == old(vec)@[ki]
                && vec@[j] == old(vec)@[kj]),
;

// Vec::capacity
pub assume_specification<T, A: core::alloc::Allocator>[ Vec::<T, A>::capacity ](vec: &Vec<T, A>) -> (result: usize)
    ensures
        result as int >= vec@.len(),
;

// FromSpec: vacuously satisfies vstd trait postcondition.
impl<T: Ord> FromSpecImpl<Vec<T>> for SortedVec<T> {
    open spec fn obeys_from_spec() -> bool { false }
    open spec fn from_spec(v: Vec<T>) -> Self { arbitrary() }
}

} // verus!

