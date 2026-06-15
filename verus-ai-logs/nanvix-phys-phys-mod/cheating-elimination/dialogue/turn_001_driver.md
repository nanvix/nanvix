## Turn 1: Full Checklist Walkthrough — phys-mod cheating elimination

### Progress
- Done (all PASS): Zero admit; Zero assume; Zero trusted; Zero no_decreases;
  Zero cfg-gated exec (non-allowed); external_body all TCB-listed; AST consistency;
  exec-rewrite comments (N/A); each external_body TCB-listed; specs not weakened;
  cross-module regression; verification + build.
- Current: none — verdict reached.
- Remaining: none.

### Verification

Scope = `mod.rs` + `mod.spec.rs` + `mod.proof.rs` (functions `init`,
`book_physical_memory_regions`, `book_mmio_regions`).

**Tools run:**
- `make verify-kernel` → Exit code 0. Cheating gate:
  `assume=0 external_body=14 admit=3 trusted=0 no_decreases=0 cfg_gate=9` (crate-global).
- `./z build -- all-kernel` → `Finished dev profile`, `[OK] Build complete`, exit 0,
  no warnings emitted.
- `git diff verus-ai-prove-bottom-up -- mod.rs mod.spec.rs mod.proof.rs` → 0 lines
  (in-scope files byte-identical to base; proving phase left phys-mod fully verified).
- `cheating-detail.txt` per-item locations confirmed.

**Per-item findings:**

1. **Zero admit()** — PASS. The 3 admits are `mm/virt/identity_map.rs:533/627/718`
   (`ensure_pt`, `ensure_pte`, `identity_map_page`) — module `mm::virt`, out of scope.
   None in mod.rs/spec/proof. `grep` of scope files for `admit(` = empty.
2. **Zero assume()** — PASS. The 4 assumes are `manager.proof.rs:31/47/61/175` —
   `mm::phys::manager` proof, out of phys-mod scope, gate-approved (L60–L63, hence
   `assume=0`). None in mod.rs/spec/proof.
3. **Zero trusted functions** — PASS. Gate `trusted=0`.
4. **Zero exec_allows_no_decreases_clause** — PASS. Gate `no_decreases=0`.
5. **Zero cfg-gated exec code (non-allowed)** — PASS. In-scope cfg gates in `mod.rs`:
   L36 `#[cfg(verus_keep_ghost)] use vstd` (import), L40/L42 `#[cfg(verus_keep_ghost)]
   include!` of spec/proof (harness includes), L15/L195 `#[cfg(feature="test")]` on the
   test module/`test()` fn (standard test harness, not verus-gated, hides nothing from
   the verifier). All present in base, byte-identical. No `#[cfg(not(verus_keep_ghost))]`
   exec divergence.
6. **external_body only if TCB-listed** — PASS. In-scope external_body items individually
   checked against `verus-ai-logs/tcb-allowed.md`:
   - `mod.rs:73 book_physical_memory_regions` → listed (tcb-allowed.md L82).
   - `mod.rs:103 book_mmio_regions` → listed (tcb-allowed.md L87).
   - `mod.spec.rs:69 ExLinkedList` (external_type_specification) → listed (tcb-allowed.md L74).
   No unlisted external_body in scope.
7. **AST consistency** — PASS. In-scope files byte-identical to base (`git diff` = 0 lines);
   no exec-code rewrites introduced, so zero possible mismatch.
8. **All exec rewrites have VERUS REWRITE comment + reproducer** — PASS (N/A). No exec
   rewrites exist (empty diff).
9. **Each surviving external_body confirmed TCB-listed** — PASS. See item 6; all three
   individually matched to tcb-allowed.md lines.
10. **No specs weakened** — PASS. `mod.spec.rs` and the `#[verus_spec]` contracts on `init`,
    `book_*` are byte-identical to base (empty diff); original `ensures` guarantees intact
    (booked frames reserved; covered MMIO frames reserved; `live()` on success).
11. **Cross-module regression** — PASS. `make verify-kernel` exit 0; all verified modules pass.
12. **Verification + build** — PASS. `make verify-kernel` exit 0; `./z build -- all-kernel`
    exit 0, 0 warnings.

### Fix Request
None. All 12 checklist items PASS with concrete tool evidence. No code changes required.

### Verdict: RESOLVED
