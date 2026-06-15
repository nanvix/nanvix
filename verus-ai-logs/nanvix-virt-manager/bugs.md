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
