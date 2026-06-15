# Bugs — `mm::virt::manager`

## Context-Dependent — `link_user_pages` / `rollback_linked_pages` (manager.rs:301, :465)

**What**: The documented contract and caller analysis state that on `Err`,
`link_user_pages` restores *both* `parent` and `child` to their entry state
("any copy-on-write marks installed on `parent` are cleared"). The
implementation's rollback (`rollback_linked_pages`) deliberately does **not**
clear the parent's CoW marks and is otherwise best-effort (it logs and continues
on every unmap/unmark failure).

**Why**: `rollback_linked_pages`'s own docstring (manager.rs:453-464) explains the
choice: distinguishing a CoW mark this call installed from one that pre-existed
(a re-fork of a frame still shared with an earlier child) cannot be done reliably
from PTE state or refcounts, so wrongly unmarking could break CoW for another
sharer. Leaving a page CoW that could have been writable only costs one extra CoW
fault, which is harmless. Hence the parent is **not** bit-for-bit restored on the
error path; some logically-writable parent pages may be left read-only with the
CoW bit set.

**Verification Failure**: Not a crash or overflow. The mismatch is between the
narrative contract ("`parent`/`child` restored to entry state") and the actual
best-effort, CoW-preserving rollback. A spec asserting
`final(parent)@ == old(parent)@ && final(child)@ == old(child)@` on `Err` would be
**unsound** (it claims behavior the code does not provide).

**How Verus Helped**: Forcing an explicit `Err` postcondition surfaced that the
"full rollback" wording is an overstatement: the only guarantee the code can
actually uphold on `Err` is that both address spaces remain well-formed
(`inv()`), not that they are byte-identical to entry.

**Severity**: cosmetic / correctness-preserving. The deliberate design choice is
safe (an abandoned fork's parent keeps working; stray CoW marks only cause a
redundant copy on the next write). No memory-safety or refcount-leak issue.

**Suggested Fix**: None to the code (the behavior is intentional and safe).
Instead the spec for `link_user_pages`'s `Err` arm captures only the provable
guarantee — `final(parent).inv() && final(child).inv()` — rather than full
restoration. The caller (`fork`) discards `child` and continues using `parent`,
which remains correct under this weaker guarantee. The caller-analysis/docstring
wording could be softened to "best-effort rollback; both spaces remain valid".

**Auto-Fixed**: No (intentional design; spec adjusted to match real behavior).

## Spec gap (fixed) — `ctrl_upage` missing precondition (manager.rs:865)

**What**: The draft `#[verus_spec]` on `ctrl_upage` omitted the
`vmem@.user_mapped(vaddr.addr_nat())` precondition. The body forwards to
`Vmem::uctrl`, whose own contract *requires* `self@.user_mapped(vaddr.addr_nat())`
(vmem.rs:~1985). With the precondition missing, `ctrl_upage`'s body could not
discharge `uctrl`'s requirement, so the function failed to verify.

**Why**: `ctrl_upage` only changes permissions on an existing mapping; it never
allocates. The "page already mapped" condition is a genuine caller obligation
(property FN-25 / LIVE-4), and the view design (view_design.md:302) lists it as a
precondition. The draft simply dropped it.

**How Verus Helped**: Removing `external_body` and translating the body produced a
precondition-not-satisfied error at the `uctrl` call site, pinpointing the missing
`requires`.

**Severity**: spec-only; no code defect. The code is correct given the (now
explicit) precondition.

**Auto-Fixed**: Yes — added
`vmem@.user_mapped(vaddr.addr_nat())` to `ctrl_upage`'s `requires`. The function
now verifies with a translated (non-`external_body`) body.

## Spec gap (fixed) — `new_vmem` missing `pgdir` distinctness (manager.rs:260, SB-2)

**What**: The draft `new_vmem` `Ok` arm omitted `new@.pgdir != vmem@.pgdir`
(property FN-2 / GLOBAL-4). Without it this module does not establish that a freshly
cloned address space gets its own page directory.

**How Verus Helped**: Cross-checking the spec against the property analysis
(FN-2/GLOBAL-4) and view design surfaced the omission.

**Severity**: spec-completeness; no code defect (the code allocates a fresh
`pgdir`). `new_vmem` is currently `external_body` (depends on the unverified
`phys`/`kpage`/`PageDirectory` modules), so the clause is a trusted guarantee until
those modules are verified; proving it then will require `Vmem::clone`'s contract
to be strengthened to expose `pgdir` freshness.

**Auto-Fixed**: Yes — added `new@.pgdir != vmem@.pgdir` to the `Ok` arm.
