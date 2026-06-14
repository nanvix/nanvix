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

## Deferred (not a bug) — read/write round-trip + entries model

The full `Table` View (per-slot `entries: Map<nat, Option<E>>` with
`spec_read`/`spec_write` and the `TableEntry` round-trip law) needs a ghost
memory-permission token keyed by the table address. Threading that token adds a
`with`-clause ghost parameter to `read`/`write` that cascades into out-of-scope
callers (`identity_map::ensure_pt`/`ensure_pte`/`identity_map_page`), all of
which currently `admit()`. Deferred to the permission layer; the struct View is
`addr`-only for now. See `view_design.md` "As-Built Decision". No verified
caller loses guarantees today. Proving phase will revisit when the permission
layer is verified.
