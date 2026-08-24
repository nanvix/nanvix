# GROUND_TRUTH — virt-MM Verus limitation audit

Binding facts that constrain this mission. This file records what is *known and
fixed* and what is *still unknown* before any Nanvix executable code is changed.
It is evidence, not a plan. Update it as facts are established; never delete
recorded uncertainty.

## Immutable inputs (verified this round)

| Input | Value | How verified |
| --- | --- | --- |
| Nanvix repo | `/home/ruize/nanvix-argus-virt-mm` @ `verus/virt-mm-limitations-20260824`, HEAD `1568ca47e9151b1f5568a3ce390ea9d0811df382` | `git rev-parse HEAD` |
| Probe repo (isolated) | `/home/ruize/verus-ai-exp/verus-ai-argus-virt-mm`, HEAD `16aa766bc9a1eba02c47d768652fc4317b72cc67` | `git rev-parse HEAD` |
| Probe script | `scripts/dummy_probe.py` (isolated repo) | `--help` inspected |
| Verus binary | `/home/ruize/verus-exp/verus-main-20260824/source/target-verus/release/verus`, version `0.2026.08.23.fbbbbcf` | `verus --version` |
| x86 target | `build/targets/x86-kernel.json` (i686, softfloat, static reloc, panic=abort) | file read |
| Kernel Verus features | `microvm trace` | Makefile `VERUS_KERNEL_FEATURES` |
| Kernel rustflags | `-C relocation-model=static -C prefer-dynamic=no` | Makefile `KERNEL_RUST_FLAGS` |
| build-std | `-Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec` | Makefile `KERNEL_CARGO_FLAGS` |
| Canonical verify | `cargo verus verify --locked --no-default-features --features "microvm trace" -p kernel <build-std> --target x86-kernel.json` | Makefile `verify-kernel` |
| Artifacts root | `/home/ruize/argus-virt-mm-artifacts-20260824` | created this round |

## Seed scope (depth-0) — FIXED (AST-validated)

The seven tracked Rust files under `src/kernel/src/mm/virt/**`:
`mod.rs` (5 fns), `boot_init.rs` (1), `identity_map.rs` (14), `kpage.rs` (3),
`manager.rs` (16), `page_table_allocator.rs` (1), `vmem.rs` (43). These fn
counts are now **validated by a tree-sitter AST enumeration** of every item
(`scope/item_inventory.json` + `.md`, via `scope/gen_item_inventory_ts.py`).
Seed `static mut`: virt/manager.rs:92 `MEMORY_MANAGER`,
page_table_allocator.rs:77 `PAGE_TABLE_STORAGE`. Seed associated-const impl:
page_table_allocator.rs:87-88 (`impl BssStorage :: NUM_UNITS / STORAGE_SIZE`).

## Project-owned dependency closure — ESTABLISHED with edges

Every out-of-tree addition carries a concrete source edge (see manifest
`project_owned_closure_additions`). Key edges:

- MMU page-table / page-directory implementations live in
  `hal/arch/shared/mem/mmu/{page_table,page_directory}.rs`, imported through the
  `#[path]` re-export in `hal/arch/x86/mem/mmu/mod.rs`.
- `fast_memcpy` / `fast_memset` (asm) back `identity_map::{memcpy,memset}`.
- `hal/mem/**` defines the address/permission/region types the VM is written
  against.
- `mm/phys/{kframe,manager,upool,frame,mod}.rs`: KernelFrame/PhysMemoryManager/
  UserFrame plus the **back-edge** `kframe.rs -> crate::mm::virt::{identity_map_page,
  memset, sync_kernel_pdes}`.
- `mm/elf.rs`: ELF32 loader driven by `VirtMemoryManager::load_elf*`.
- `hal/platform/microvm/mod.rs` (**item-scoped, scan-only**): provides the 3
  items on the virt-MM edge — `virt_to_phys` (vmem.rs:1393,1566),
  `is_valid_physical_region` (vmem.rs:694), and the `NUM_PAGE_TABLES` const
  (identity_map.rs:34, page_table_allocator.rs:34). `hal::platform` resolves to
  this module under the `microvm` feature. It is **scanned but not injected**:
  injecting it would verus-process its module and surface 6 UNRELATED platform
  `static mut` items (GDT/IDT/IDTR/HEAP/KLOG/frame-allocator storage,
  mod.rs:121-155), contaminating the probe. **Coverage of these 3 items is
  item-level source inspection, not verifier coverage** — the zero-microvm-findings
  result is a contamination check (`FRONTEND_OK` enumeration needs a complete
  terminal pass, which did not occur), not proof that Verus verified them.

**x86_64-only exclusion (cfg-attributed).** `hal/arch/x86_64/mem/mmu/hwpt.rs`
and the six `vmem.rs` hwpt calls (L223,272,293,309,326,2054) are all under
`#[cfg(target_arch = "x86_64")]`. The probe target is i686
(`target_arch = "x86"`), whose `hal::arch` routing (`hal/arch/mod.rs:8-9`)
selects `hal/arch/x86/` — which has **no** hwpt module. So every hwpt edge is
dead under the probe target. Recorded in manifest `cfg_exclusions` because the
host-x86_64 rust-analyzer reachability run reached hwpt.rs via the x86_64 route.

30 audited kernel-crate files in the scanned scope (29 injected + item-scoped
microvm/mod.rs); pre-run SHA256 in `snapshots/pre_run_sha256.txt`, pristine
copies in `snapshots/pristine/`.

## Depth-0 reachability — EXECUTED and reconciled

`scripts/reachability.py` (find_deps_lsp.py / rust-analyzer LSP) was run at
`--depth 0` rooted at the seven seed files' 52 public entry points
(`scope/reachability_entry_points.txt`). Output: 60 reachable files / 1207 fns
(`reachability_run/`). Reconciled against the manual scope in
`scope/reachability_reconciliation.{json,md}`: 23 files in both, 10 LSP-only
kernel files (hwpt x86_64-only; 5 spec/proof companions; microvm + microvm.spec;
mm/mod.rs parent glue), 7 manual-only (mod.rs re-export shims that LSP resolves
through to leaves + the page_table_allocator SEED which has no public fn entry
point), and 27 workspace-crate (`src/libs`) boundary files. rust-analyzer indexes
the host x86_64 target, so arch paths were normalized x86_64/* -> x86/*.

## Boundaries (not in kernel-crate scan)

- **Workspace crates** split by Makefile:317 `VERUS_CRATES := bitmap sys
  nanvix-slab arch kernel`. Reached **Verus-verification targets** (`arch`, `sys`,
  `bitmap`) are verified in their own crate via `make verify-<crate>`. Reached
  **project-owned, non-target** crates (`config`, `bump_allocator`, `elf`,
  `error`, `raw-array`, plus host build-dep `build-utils`) are compiled/checked as
  ordinary dependencies but their exec bodies are **NOT** independently
  Verus-verified by the project pipeline. If a virt-MM limitation traces into a
  target crate's exec bodies it is probed under that crate, not by widening this
  run. **All 31 reachable dependency files carry a concrete edge and an explicit
  disposition** in manifest `workspace_files` (14 `OWN_CRATE_VERIFIED` + 6
  `PROJECT_OWNED_UNVERIFIED` + 8 `EXCLUDE:verus-spec-proof` + 1
  `GENERATED_BUILD_ARTIFACT` + 2 `BUILD_SCRIPT_HOST`); the 27 LSP-reached
  `src/libs/**` files plus the 4-node `config::kernel::MEMORY_SIZE` generation
  chain (manual, non-LSP). `nanvix-slab` is a VERUS_CRATES target but is **not**
  reached on this closure. Crate boundaries are in `workspace_crate_boundaries`.
- **External/hardware**: Rust `core`/`alloc`; x86 inline asm and CR3/MMU hardware.

## Resolved binding facts (this round's probe run)

1. **`MEMORY_SIZE_BYTES` is a required build-script env var.** `config`'s
   `build.rs` → `build-utils::memory_size()` (`src/libs/build-utils/src/lib.rs:68`)
   panics without it. Makefile exports `MEMORY_SIZE?=256` and
   `MEMORY_SIZE_BYTES=MEMORY_SIZE*1048576=268435456`. The probe bypasses Make, so
   the wrapper (`artifacts/run_probe.sh`) must export it. (Was unknown #? — now fixed.)
2. **`--baseline-filter` no longer exists** in the reviewed dummy_probe (removed as
   unsound: transformed-source line drift). Sanctioned replacement =
   **source-stable baseline preflight**: a no-mutation `--full-verify` run on the
   pristine tree (`--no-inject-verus-verify --no-strip-external-body --no-layered`)
   whose diagnostics are reconciled against the injected run by a source-stable
   identity `(rel_file, classification, code, normalized_message)` that excludes
   line/col. Implemented in `reconcile_baseline.py`; a fresh external
   `CARGO_TARGET_DIR` prevents cache contamination.
3. **Canonical FULL-VERIFY probe command shape (verified working):** `cargo verus
   verify -p kernel --no-default-features --locked --features "microvm trace" -Z
   build-std=... -Z build-std-features=compiler-builtins-mem -Z json-target-spec
   --target x86-kernel.json -- --multiple-errors 4 --expand-errors`
   (`front_end_only=false`; `--no-verify` dropped so SMT is enabled). dummy_probe
   hardcodes `-p <crate> --no-default-features`. Verus aborts in the rustc
   front-end on the `static mut`/E0407 items, so SMT is never reached for the
   virt-MM crate and full-verify yields the same limitation set — these are
   front-end translation limitations, not SMT/proof-obligation failures.
4. **Two genuine Verus frontend limitation categories found** (see
   `probe_run3/triage.{json,md}`, FULL-VERIFY canonical run):
   - **`static mut` unsupported** at 5 global-singleton sites (phys/frame.rs:78,933;
     phys/manager.rs:48; virt/manager.rs:92; virt/page_table_allocator.rs:77) —
     architecture-level, deferrable; **PRIMARY fixed-point blocker** (non-fn items
     can't be external_body-shielded → `unresolved_frontier`, `layered_complete=false`).
   - **Associated-const trait impls unsupported** (page_table_allocator BssStorage
     impl → E0407 on `VERUS_UNERASED_PROXY__*`); confirmed by Verus
     `builtin_macros/unerased_proxies.rs:79,146`. `BssStorage` is a
     **project-owned cross-crate** trait (`src/libs/bump_allocator/src/lib.rs:174`),
     NOT external/std; any later repair must preserve that trust boundary and must
     not `external_body`-shield the project-owned impl.
   - `&'static mut [...]` borrows are NOT static-mut items — excluded.
5. **Both categories confirmed on latest Verus via minimized legal-Rust
   reproducers** (`research/reproducers/`, Verus `0.2026.08.23.fbbbbcf`):
   - `repro_static_mut_verusverify.rs` / `repro_static_mut_verusblock.rs` →
     `error: Verus does not support 'static mut'` (byte-identical to the probe).
   - `repro_assoc_const_impl.rs` → `error[E0407]: method
     VERUS_UNERASED_PROXY__* is not a member of trait Bss` (structurally
     identical). Root cause: Verus `builtin_macros/src/unerased_proxies.rs`
     :79,:146. See `research/reproducers/RESULTS.md`.
6. **Residual inconclusive classified.** The terminal
   `residual_verdicts=[INCONCLUSIVE, LIMITATION]` +
   `attribution_gaps=[non_frontier_diag:RUSTC_ERROR]` are the two E0407
   unerased-proxy errors (CAT-assoc-const-trait-impl). Final class =
   **frontend_limitation**; the "inconclusive" was purely the fn-frontier
   peeler's inability to `external_body`-shield a non-fn associated-const impl
   item (a dummy_probe tool-observation, not a Nanvix/SMT unknown).
7. **Canonical FULL-VERIFY run reproducible + clean + baseline-attributed.**
   probe_run3 (`--full-verify --inject-verus-verify --no-auto-seal-stdlib-deps`,
   layered max 100, fresh external `CARGO_TARGET_DIR`, scan=30/inject=29) gives
   the same 4 findings, `stop_reason=unresolved_frontier`,
   `layered_complete=false`, 0 restoration mismatches, 0 residual markers, and
   **zero microvm contamination**. The **source-stable baseline preflight** found
   **0** diagnostics on the pristine tree; reconciliation attributes **all 6
   distinct canonical diagnostics as injection-induced, 0 pre-existing**
   (`probe_run3/baseline_reconcile.{json,md}`), so every finding is attributable
   to instrumenting the audited scope rather than to prior source state.
8. **microvm/mod.rs coverage is source-inspection, NOT verifier coverage.** Zero
   microvm findings is a *contamination check*, not empirical verifier coverage:
   `FRONTEND_OK` enumeration requires a complete terminal verifier pass, which did
   not occur (`layered_complete=false`). The three virt-MM-reachable microvm items
   (`virt_to_phys` identity fn, `is_valid_physical_region` checked_add bounds test,
   `NUM_PAGE_TABLES` const) are covered by item-level source inspection only.
9. **Non-function declaration peeling now crosses the original two-round
   blocker.** The repaired isolated probe assigns distinct AST identities to all
   five `static mut` declarations and to the `BssStorage` impl, removes their
   injected verification markers, and reaches a four-round terminal frontier.
   Round 3 contains 133 diagnostics in
   `ASSUME_SPECIFICATION_REQUIRED`, `OPAQUE_TYPE_FRONTIER`,
   `PROJECT_SPECIFICATION_REQUIRED`, and `TYPE_SPECIFICATION_REQUIRED`; this is
   deeper coverage, not a fixed point. The full command, per-round trace,
   findings, baseline reconciliation, and restoration evidence are under
   `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run5/`.
10. **Native `make verify-kernel` does not process the five `static mut`
    declarations.** On the pristine tree with latest Verus it reports
    `1882 verified, 0 errors` for the kernel (and zero errors for every
    subsequent crate result). The `static mut` diagnostics are therefore
    injection-induced probe findings, not failures of the current native
    verification surface. Exact command and output are retained in
    `probe_run5/make_verify_kernel.*`.
11. **Peeling tool fix committed; run-6 canonical rerun reproduces the frontier
    and is fully triaged.** The non-function declaration peeling category is now
    committed in the isolated probe repo as `0f9825f9` (`fix(dummy_probe): peel
    non-function declaration markers (static mut, assoc-const impl)`), file set
    exactly `scripts/dummy_probe.py` + `tests/test_dummy_probe.py` (15 focused
    peeling regression tests pass; 5 pre-existing `_bisect_panic_within_file`
    failures reproduce at HEAD and are unrelated to this diff). The canonical
    `run_probe_full.sh` rerun into fresh `probe_run6/` + fresh
    `cargo-target-20260824-150838/` (Verus `0.2026.08.23.fbbbbcf`,
    `--max-layer-rounds 100`, 30-scan/29-inject) reproduces run-5: same 6 peels
    (5 `static mut` + `BssStorage` impl), `failed_peels=[]`,
    `stop=unresolved_frontier`, `layered_complete=false`, **0 restoration
    mismatches**, **0 residual markers**, pristine-baseline **0** diagnostics,
    all **142** distinct diagnostics injection-induced. Full triage of every
    finding/diagnostic (semantic category, occurrence set, evidence, disposition)
    is in `probe_run6/triage.{json,md}`. Promoted (reproducer-backed) frontend
    limitations: CAT-static-mut (5) and CAT-assoc-const-trait-impl E0407 (2).
    Candidate frontend limitations behind the peeled frontier (36 diagnostics:
    ptr casts, inline-asm, const-block, panic, deref, complex-break,
    only-variables, closures-capturing-mut-ref, plus `Path Def(Static)` reads
    coupled to the static-mut architecture) are NOT promoted — no minimized
    reproducer built this node. The remaining 109 diagnostics are non-qualifying
    project/type/assume-specification + opaque-type obligations. No proof/SMT
    obligations were observed (front-end aborts before SMT).

## Still-unknown binding facts

1. **Whether the layered run can reach a real fixed point after middle-layer
   specification work.** The non-function blocker is resolved, but the current
   run still stops at `unresolved_frontier` after four rounds because missing
   project/type/function specifications and one opaque-type frontier are not
   peelable frontend limitations.
2. **What additional frontend limitations lie behind the newly exposed
   specification/type frontier** remains unknown until those project-owned
   dependencies and contracts are handled in later bounded missions.
3. ~~Whether real `make verify-kernel` rejects the `static mut` items~~ —
   **RESOLVED** (see Resolved fact #10). Native verification succeeds because
   those declarations are outside its current verified surface.
4. ~~Minimized legal-Rust reproducers~~ — **RESOLVED** (see Resolved fact #5).
   Both categories reproduced on latest Verus with root cause identified.
5. ~~arch-crate reachability~~ — **RESOLVED** (see Depth-0 reachability). The
   executed reachability run confirms virt-MM's only arch/library edges are
   out-of-crate boundaries (hwpt is x86_64-dead; `bump_allocator::BssStorage` is a
   **project-owned cross-crate** trait behind CAT-assoc-const; `src/libs/*` are
   workspace crates, each reachable file dispositioned in manifest
   `workspace_files`).

## Hard constraints (mission)

- No `.verus_agent` use and no Nanvix category repair in this baseline task.
- No `external_body`/`assume`/`admit`/`assume_specification`/new trust when repairs
  begin later.
- Byte-for-byte restoration of every instrumented file must be verified after each
  run (`snapshots/pre_run_sha256.txt` is the oracle).
