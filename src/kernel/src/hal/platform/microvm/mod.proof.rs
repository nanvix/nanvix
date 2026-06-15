// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// hal::platform::microvm — Proofs (target: `gva_to_gpa`)
//
// `gva_to_gpa` is a pure query, not a state mutation, so there is no transition
// lemma. The single caller-relevant algebraic property is injectivity of the
// translation map (distinct page-aligned inputs map to distinct physical
// frames), which `book_mmio_regions` relies on to walk distinct frames. It is
// exposed as a View-level lemma here. Body is `admit()` during the
// specification phase; the proving phase discharges it (it follows directly
// from the identity definition of `spec_gva_to_gpa`).

verus! {

// Injectivity of the MicroVM address translation: distinct guest virtual
// addresses map to distinct guest physical addresses, so the MMIO frame walk in
// `book_mmio_regions` visits distinct frames.
pub proof fn lemma_translation_injective(v: MicrovmTranslationView)
    ensures
        v.injective(),
{
    admit();
}

} // verus!
