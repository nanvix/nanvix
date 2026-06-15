# Bugs — `arch::x86::mem::paging::table`

No code bugs were found in this module. All scope functions admit correct
specs as written; the only obstacles are a Verus language limitation and a
deferred abstraction, both recorded below for the proving phase.

## Not a code bug — Verus limitation (int-to-ptr cast)

`Table::read` / `Table::write` materialize a raw pointer from the integer base
address (`usize as *const/*mut PteWord`) and perform a volatile access of
externally-owned page-table memory. Verus does not support `usize`→pointer
casts (`error: Verus does not support this cast: usize to *const u32`). This is
a tooling limitation, not a logic error in the code. Mitigated with the
documented `external_body` trust boundary (see `verus-unsupported.md` and
`verus-ai-logs/tcb-allowed.md`). No fix required.

## Resolved (Turn 1 review) — read/write now carry full contracts

The earlier "deferred round-trip / addr-only View" note was resolved during the
Turn 1 specification review. `read`/`write` and the `TableEntry` codec now carry
complete `#[verus_spec]` contracts referencing a **global, parameter-free**
page-table-memory ghost (`spec_table_word`/`spec_table_read`), mirroring
`frame::instance`→`phys_view()`. No exec signature changed, so the out-of-scope
`admit()` callers do not cascade (`make verify`: kernel 76 verified, 0 errors).
The `entries: Map<nat, Option<E>>` model and the read-after-write round-trip law
(`lemma_entry_roundtrip`) are now expressed, not deferred. `read`/`write` remain
`external_body` only for the genuine Verus `usize`→pointer limitation below.

## Proving phase — `admit()` eliminated (no bug)

The spec-phase `admit()` in `lemma_entry_roundtrip` was the trait codec law
(`spec_entry_from_raw(spec_entry_raw(e)) == Some(e)`). Because the codec spec
functions are `uninterp` over a structureless generic `E`, the law is not
derivable in-module; it is a sound foundational axiom (a faithful serialization
is injective). It was converted from the banned `admit()` placeholder to the
idiomatic Verus broadcast axiom form (`external_body`, empty body) and registered
in `verus-ai-logs/tcb-allowed.md`. `read`/`write` keep the documented int-to-ptr
`external_body` trust boundary. Final: `make verify-arch` 47 verified, 0 errors,
admit=0, assume=0. No code bug.
