---
id: verus-integer-to-pointer-cast-limitation
type: fact
status: active
title: Verus rejects integer-to-pointer construction in Nanvix PM
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
---

Verus `0.2026.08.23.fbbbbcf` rejects legal Rust integer-to-pointer `as`
conversions during HIR-to-VIR lowering. The same build also rejects
`core::ptr::with_exposed_provenance[_mut]`,
`core::ptr::without_provenance[_mut]`, and integer-to-pointer `transmute`
before verification; the diagnostics suggest `assume_specification`, which is
not an admissible project fix.

The vstd two-argument `with_exposed_provenance` operation is accepted, but its
tracked argument must identify a provenance that was exposed. The always
constructible `IsExposed::null()` token denotes null provenance, so it does not
model the ambient exposed provenance of a non-null syscall or MMIO address.

An absolute source scan of the 66 Nanvix PM Rust files identifies 14
Verus-visible integer-to-pointer sites. Ten use the pointer only as an address
carrier into `pm::copy_from_user` or `pm::copy_to_user`; those helpers immediately
convert it back to `VirtualAddress` and copy through page tables. Four synthesize
pointers that are genuinely dereferenced by an MMIO volatile read,
`from_raw_parts`, or stack forging. The scan also found a
`not(verus_keep_ghost)`-masked `from as *const u8` cast, but `from` is already a
`*mut ContextInformation`; that site is pointer-to-pointer and is excluded from
the integer-to-pointer family.

The direct-cast limitation and the in-place spelling refutation are established.
The ten address-carrier sites are now **resolved** by a landed, behavior-preserving
PM-local rewrite: new helpers `pm::copy_from_user_addr` and `pm::copy_to_user_addr`
accept the user address as a `VirtualAddress`, and the retained pointer-typed
`pm::copy_from_user`/`pm::copy_to_user` delegate to them, so `ipc` and `io` callers
keep the existing pointer API. The ten PM callers now pass the `VirtualAddress`
they already compute, removing every integer-to-pointer cast at those sites.
Because the kernel-buffer `&ref as *ptr as usize` lowering is written once in the
address-typed helper and reused by delegation, the pre-existing raw-pointer-deref
finding is relocated rather than multiplied (run-9 delta: deref family 8→8).

Run-9 (fresh 66-file layered probe, isolated target dir, 100-round cap) reached a
fixed point in 3 rounds with 0/66 restoration mismatches and `clippy -D warnings`
exit 0; every address-only cast dropped to 0 while the four genuinely-dereferenced
sites (MMIO volatile read, `from_raw_parts`, stack forging) remain a genuine
limitation. Isolated frontend evidence: `intptr/intptr_addr_carrier_ok.rs` is
accepted (`9 verified, 0 errors`) and `intptr/intptr_addr_helper_deref.rs` emits
only the pre-existing implicit-deref diagnostic with no cast error.

The minimized cases and logs are under
`/home/ruize/argus-pm-artifacts-20260824/reproducer/intptr/`; the inventory and
current classification are in `research/GROUND_TRUTH.md`.
