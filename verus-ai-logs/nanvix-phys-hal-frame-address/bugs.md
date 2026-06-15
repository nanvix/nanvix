# Bugs — hal::mem::types::address::frame

## Summary

No code bugs found. All target functions (`FrameAddress::into_raw_value`,
`into_frame_number`, `from_raw_value`, `from_frame_number`, and the `inv()`
invariant) verify against their unchanged contracts.

## Trust-boundary note (not a bug) — `lemma_phys_view_is_spec_addr` (frame.proof.rs:28)

**What**: The single `admit()` in the module was the bridge proof
`spec_addr(&pa) == pa@` for a `PhysicalAddress`.

**Why it is not provable here**: `spec_addr<T: Address>` is declared `uninterp`
in `page.spec.rs` because it must apply to a bare `T: Address`, which carries no
`View<V = int>` bound. Nothing in scope constrains `spec_addr` for the concrete
`PhysicalAddress` type: `PhysicalAddress`'s own specs are stated purely over its
`View` (`pa@`), and the only library-edge fact relating `spec_addr` to a view —
the `<PageAligned<T> as Deref>::deref` boundary (`spec_addr(result) == addr@`) —
collapses to a tautology once `PageAligned<T>::view == spec_addr(&self.0)` is
unfolded (`deref` returns `&self.0`). The equality is true semantically (both
sides are the physical address) but only becomes derivable once the
`sys::mm::Address` trait `impl for PhysicalAddress` is verified, which is blocked
by its `usize as *const/*mut u8` sibling casts (see `verus-unsupported.md`).

**Resolution**: Classified as a **False Positive / external-bottom trust
boundary**, not a code bug. The specification phase already reviewed the claim as
semantically sound and deferred its discharge to the proving phase. Discharged
with the governed `axiom fn` mechanism (no `admit`, no `assume`, no
`external_body`), registered in `verus-ai-logs/tcb-allowed.md`. No target contract
was weakened. The axiom is removed when `sys::mm::Address` is verified.

**Severity**: n/a (trust boundary, not a defect).
