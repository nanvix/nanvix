// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// hal::platform::microvm — Specifications (target: `gva_to_gpa`)
//
// The in-scope facet of this module is the MicroVM platform's guest-virtual →
// guest-physical address translation (`gva_to_gpa`). It is a free function
// (`#[inline(always)] pub fn gva_to_gpa(gva: usize) -> usize`) — pure, total and
// deterministic — that reads/writes no global state. There is therefore no
// caller-observable *state* to model, only a caller-observable *mathematical
// map*. The honest View for this scope is a stateless (unit) View whose content
// is a single pure translation function plus the algebraic properties the caller
// relies on. See `verus-ai-logs/nanvix-phys-hal-platform-microvm/view_design.md`.

verus! {

// Caller-visible abstraction of the MicroVM platform's address-translation
// facet. The translation `gva_to_gpa` is a pure, stateless function, so this
// View carries no fields: there is no mutable abstract state a caller observes
// across calls. The substance of the abstraction is the spec function
// `spec_gva_to_gpa` and its properties, defined below on this type.
pub struct MicrovmTranslationView {}

impl MicrovmTranslationView {
    // Well-formedness of the translation facet. The facet is stateless, so there
    // is no internal bookkeeping to constrain; well-formedness (totality,
    // determinism, injectivity) is structural — a property of `spec_gva_to_gpa`,
    // a total Verus spec function — not an invariant over mutable fields.
    pub open spec fn inv(self) -> bool {
        true
    }

    // Abstract guest-virtual → guest-physical translation as a mathematical map
    // over addresses. On the MicroVM platform the guest address space is flat
    // and identity-mapped, so this is the identity. Addresses are modeled as
    // `nat` (raw, non-negative machine addresses in spec world). `open` because
    // identity is the caller-relevant MicroVM contract: callers depend on the
    // result equaling the input to walk distinct frames.
    pub open spec fn spec_gva_to_gpa(self, gva: nat) -> nat {
        gva
    }

    // Injectivity of the translation over the address space: distinct guest
    // virtual addresses yield distinct guest physical addresses. This is what
    // lets `book_mmio_regions` advance `start` by `FRAME_SIZE` and be sure it
    // walks distinct physical frames (no aliasing / double-booking).
    pub open spec fn injective(self) -> bool {
        forall|a: nat, b: nat|
            #![trigger self.spec_gva_to_gpa(a), self.spec_gva_to_gpa(b)]
            self.spec_gva_to_gpa(a) == self.spec_gva_to_gpa(b) ==> a == b
    }
}

} // verus!
