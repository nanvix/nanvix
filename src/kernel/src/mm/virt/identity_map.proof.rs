verus! {

//==================================================================================================
// Transition lemmas for the kernel identity map
//
// Signatures only during the specification phase; bodies are `admit()` placeholders that the
// proving phase discharges. They expose the monotonicity / idempotence / accessibility facts of
// the View transitions (`spec_install_page`, `spec_map_page`) that the in-scope exec functions
// (`ensure_pte`, `identity_map_page`) rely on.
//==================================================================================================

// Installing a page makes that page a member of `mapped` (the V==P realization performed by
// `ensure_pte`).
pub proof fn lemma_install_page_maps(v: IdentityMapView, page: int)
    ensures
        v.spec_install_page(page).mapped.contains(page),
{
    assert(v.spec_install_page(page).mapped =~= v.mapped.insert(page));
}

// Installing a page never removes an already-mapped page (monotonicity: once mapped, stays
// mapped).
pub proof fn lemma_install_page_monotone(v: IdentityMapView, page: int)
    ensures
        v.mapped.subset_of(v.spec_install_page(page).mapped),
{
    assert(v.spec_install_page(page).mapped =~= v.mapped.insert(page));
    assert forall|x: int| #[trigger] v.mapped.contains(x) implies
        v.spec_install_page(page).mapped.contains(x) by {
        assert(v.mapped.insert(page).contains(x));
    }
}

// Installing a page preserves well-formedness when the new page is page-aligned and the mapper is
// initialized (`mapped` stays empty only before init).
pub proof fn lemma_install_page_preserves_inv(v: IdentityMapView, page: int)
    requires
        v.inv(),
        v.initialized,
        spec_is_page_aligned(page),
    ensures
        v.spec_install_page(page).inv(),
{
    let v2 = v.spec_install_page(page);
    assert(v2.mapped =~= v.mapped.insert(page));
    assert forall|p: int| #[trigger] v2.mapped.contains(p) implies spec_is_page_aligned(p) by {
        assert(v.mapped.insert(page).contains(p));
        if p != page {
            assert(v.mapped.contains(p));
        }
    }
}

// After `identity_map_page`'s full transition the target page is accessible, regardless of whether
// the mapper was initialized (post-init: it is mapped; pre-init: boot tables cover it).
pub proof fn lemma_map_page_accessible(v: IdentityMapView, page: int)
    ensures
        v.spec_map_page(page).accessible(page),
{
    if v.initialized {
        assert(v.spec_map_page(page) == v.spec_install_page(page));
        assert(v.spec_install_page(page).mapped =~= v.mapped.insert(page));
        assert(v.spec_install_page(page).mapped.contains(page));
    }
}

// `spec_map_page` preserves well-formedness for a page-aligned target.
pub proof fn lemma_map_page_preserves_inv(v: IdentityMapView, page: int)
    requires
        v.inv(),
        spec_is_page_aligned(page),
    ensures
        v.spec_map_page(page).inv(),
{
    if v.initialized {
        lemma_install_page_preserves_inv(v, page);
    }
}

} // verus!
