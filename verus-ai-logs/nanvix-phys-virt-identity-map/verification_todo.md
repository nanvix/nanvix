# Verification TODOs — mm::virt::identity_map

Status: **no proof gaps**.

The module contains **zero** `admit()` and **zero** `assume()` in source, spec, or
proof files. The four abstract laws in `identity_map.proof.rs`
(`lemma_map_idempotent`, `lemma_map_on_success`, `lemma_map_monotone`,
`lemma_map_preserves_inv`) are fully proven (no admitted bodies); the header
comment referencing `admit()` is stale documentation, not live code.

The only retained trust surface is the three in-scope `#[verus_verify(external_body)]`
shims (`ensure_pt`, `ensure_pte`, `identity_map_page`), all explicitly listed in
`verus-ai-logs/tcb-allowed.md` ("Allowed `external_body` — `mm::virt::identity_map`
(proof target)"). These are not proof gaps: their bodies touch state Verus cannot
model (module-global `static`s, raw page-table memory via `arch::Table` volatile
pointer ops, the interior-mutable `PAGE_TABLE_ALLOCATOR`, `arch` newtype/enum-flag
constructors, and the `paging::invlpg` inline-asm). Their contracts are stated over
the uninterpreted `identity_map_view()` accessor, which by construction cannot be
derived from the body — exactly the `phys_view()` trusted-boundary pattern. They are
removable only when the `arch` paging types and a page-table memory model are
themselves verified, which is out of scope for this module.

No actionable, in-scope proof obligations remain.
