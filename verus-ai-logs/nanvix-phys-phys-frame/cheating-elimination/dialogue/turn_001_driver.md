## Turn 1: Full cheating-elimination checklist sweep (phys::frame)

### Progress
- Done (PASS): all 12 checklist items (see below).
- Current: final review — every item verified against actual code + tool output.
- Remaining: none.

Scope: the phys-frame module = `src/kernel/src/mm/phys/frame.rs`,
`frame.spec.rs`, `frame.proof.rs`. Kernel-wide counters in the cheating
report (`admit=16`, `external_body=19`, `cfg_gate=19`) include OTHER modules
(hal address layer, `manager`, `mm::virt`) that are out of scope here and
tracked on their own TCB lines.

### Verification

Command run: `make verify-kernel` → **Exit code 0**, summary
`verification: cached (no recompilation), — (exit 0)`,
`cheating: assume=0 ... trusted=0 ... no_decreases=0`. Git working tree for
`src/kernel/src/mm/phys/` is clean. Only output diagnostics were informational
"automatically chosen trigger" **notes** (count: 1) — `0` lines matching
`^warning:`.

Per-file grep of the three target files (`frame.rs`, `frame.spec.rs`,
`frame.proof.rs`):

1. **Zero admit() — PASS.** No `admit` in any of the three target files. The
   admits in `cheating-detail.txt` are all in out-of-scope files
   (`hal/mem/types/address/*.proof.rs`, `manager.proof.rs`,
   `mm/virt/identity_map.*`).
2. **Zero assume() — PASS.** `assume=0` kernel-wide; none in target files
   (only the words "assume_specification"/"assume" appear inside comments in
   `frame.spec.rs`).
3. **Zero trusted functions — PASS.** `trusted=0`; no `trusted` markers in
   target files.
4. **Zero exec_allows_no_decreases_clause — PASS.** `no_decreases=0`; no
   `no_decreases`/`external`/`external_fn` in target files.
5. **Zero cfg-gated exec code (only imports/derives/debug_assert/logging) —
   PASS.** Every `#[cfg(...)]` in `frame.rs` falls in an allowed class:
   - `#[cfg(verus_keep_ghost)]` (49, 52): `include!` of `frame.spec.rs` /
     `frame.proof.rs` — ghost imports.
   - `#[cfg_attr(verus_keep_ghost, verus_spec(...))]` (337, 1134, 1261):
     Verus contract attachment — not exec code.
   - `#[cfg(not(verus_keep_ghost))]` blocks (167,186,203,236,303,315,358,408,
     491,580,594,646,749,764,796,862,886,899,974,995,1179,1182,1219,1221,1223,
     1226,1290,1294): each is exactly one of `error!(...)` (logging),
     `debug_assert_eq!(...)` (debug_assert), or a diagnostic-only `let`
     binding. The four diagnostic bindings (`uncovered_addr`,
     `conflicting_addr`, `region_start`, `region_end`) were proven consumed
     **only** inside `error!` macros (grep showed no other use); they have no
     control-flow / state / return-value effect, so they are logging-support.
     The actual `return Err(...)`, `reason` bindings, and state mutations
     (`self.refcount[i] = 1;`) are NOT gated → Verus sees identical behavior.
6. **Zero external_body unless TCB-listed — PASS.** The 10 `external_body`
   in `frame.rs` are each listed in `verus-ai-logs/tcb-allowed.md`:
   `instance` (1408), `init` (1446, skip/exclude), `alloc` (1502),
   `alloc_contiguous` (1532), `free_count` (1553), `free` (1571),
   `book` (1613), `alloc_range` (1634), `share` (1654), `refcount` (1675).
   None unlisted.
7. **AST consistency: zero mismatches — PASS.** No `ast-consistency` script or
   skill exists in this repo, so done manually: the only verus/non-verus
   divergences are the allowed logging/debug_assert/diagnostic cfg blocks of
   item 5. The 8 `// VERUS BUG FIX:` rewrites are **non-divergent** — the
   rewritten expression (`X.into_raw_value() / mem::FRAME_SIZE`, range guard,
   unaligned-reject) is identical in both `verus_keep_ghost` and production
   configs, so they introduce no AST mismatch.
8. **All exec rewrites have comment + minimal reproducer — PASS.** The 8
   exec rewrites (297, 547, 716, 851, 856, 951, 1023, 1085) each carry an
   inline `// VERUS BUG FIX:` comment AND are fully documented in
   `verus-ai-logs/nanvix-phys-phys-frame/bugs.md` (Where / What / Why /
   "Verification Failure" with reproduction command
   `make verify-kernel MODULE=mm::phys`). These rewrites replace
   `into_frame_number()` (a checked, panicking unwrap that Verus proved
   reachable on the top-of-space aligned address) with the total computation
   — a genuine safety fix applied uniformly, not a verification dodge.
9. **Each surviving external_body confirmed TCB-listed — PASS.** All 10
   enumerated in item 6 cross-checked against `tcb-allowed.md`; every one has
   an explicit entry with rationale. No eliminations required.
10. **No specs weakened (spec drift) — PASS.** Git tree clean; the only
    `frame.spec.rs` change (commit 43d1db8e6) **removed** redundant
    placeholder `assume_specification`s (`FRAME_SIZE`, `PageAligned` →
    `Address::into_raw_value`/`deref`) that are now superseded by real
    verified contracts — strengthening, not weakening. `Inner::inv` intact
    (`self@.wf() && self.internal_inv()`). All in-body `Inner` methods retain
    full `requires`/`ensures` contracts (`alloc` 115, `alloc_contiguous` 257,
    `free` 511, `share` 687, `refcount` 834, `book` 929, `is_covered` 1011,
    `alloc_range` 1052).
11. **Cross-module regression — PASS.** `make verify-kernel` verifies the
    whole kernel crate; exit 0, all modules pass (cached, no errors).
12. **Verification + build — PASS.** `make verify-kernel` exit 0, 0 errors,
    0 warnings (only informational trigger notes), `cargo` "Finished dev
    profile" included in the same invocation.

### Fix Request

None. All 12 checklist items PASS with concrete tool evidence. No code changes
required. Creating STOP = RESOLVED.
