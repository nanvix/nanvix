# Verification TODOs: phys-kframe

In-scope functions (`KernelFrame::new`, `KernelFrame::drop`, `KernelFrame::base`)
are fully proven: `make verify-kernel MODULE=mm::phys` reports **22 verified, 0
errors**. There are **no `admit()` / `assume()` proof gaps** in any kframe file.

The items below are **external-library trust boundaries**, not proof gaps. They
do not trip the cheating gate as `assume`/`admit` (the gate reports `assume=0
admit=0`), are explicitly enumerated in `verus-ai-logs/tcb-allowed.md`, and can
only be discharged when their *not-yet-verified home modules* are verified. They
are recorded here for an honest hand-off.

## Remaining trust boundaries

- `<crate::hal::mem::PageAligned<T> as crate::hal::mem::Address>::from_raw_value`
  — `assume_specification` in `kframe.spec.rs:33`.
  - **Why it cannot be eliminated in this scope:** `from_raw_value` is a method
    of the *external* `sys::mm::Address` trait, whose home module `hal::mem` is
    not yet verified. A trait-impl method cannot be given a standalone
    `external_body` `#[verus_spec]` contract without marking the entire trait
    `impl` verified (which would pull every sibling method into scope), so
    `assume_specification` is the only available mechanism. Escalation ladder
    exhausted: vstd ships no spec for this project-defined trait; an isolated
    reproducer confirms Verus rejects a call to the unspecced external trait
    method without it; no equivalent rewrite avoids the call because
    `KernelFrame::new`'s exec body (which must stay byte-identical to base) calls
    `PageAligned::from_raw_value(base.into_raw_value())` to obtain the
    page-aligned physical address it identity-maps.
  - **Blocking dependency / removal condition:** removed when `hal::mem` / the
    `Address` trait are verified.

- `KernelFrame::clear` — `external_body` in `kframe.rs:141`.
  - **Why allowed / out of scope:** materializes a `*mut u8` from the frame's raw
    address (`usize as *mut u8`) and writes through the identity-map `memset`
    backend — a raw-memory operation Verus cannot model. Listed in
    `tcb-allowed.md`. It is also outside the in-scope function set
    (`new`/`drop`/`base`); per task rules unlisted functions must not be touched.
