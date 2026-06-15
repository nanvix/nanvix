# Bugs — `mm::virt::identity_map`

None.

No code bug was found in `ensure_pt`, `ensure_pte`, or `identity_map_page`. The
exec logic is correct.

The three in-scope functions could not be verified in-body, but this is **not** a
code defect — it is a deferred proof-infrastructure issue (the `mm::virt`
identity-map ghost/permission token is never realized; dependency contracts such
as `Table::write` are deliberately contents-free, the `KERNEL_PD_PADDR` atomic
load is unspecified, and `bump_view(_).inv()` has no establishing fact). Under
the task's hard rules (fixed exec signatures, no new trust boundaries, no
`admit`/`assume`, no `external_body` on in-scope functions, no spec weakening)
these obligations cannot be discharged. Full technical analysis and the honest
hand-off are recorded in `verification-todo.md` (same directory).
