# GROUND_TRUTH — PM Verus frontend-limitation probe

Binding facts recorded before any mutation. Timestamps in local time.

## Nanvix working repository

- Path: `/home/ruize/nanvix-argus-pm`
- Branch: `verus/pm-limitations-20260824`
- HEAD: `42d84b51b799c6a613f16f70c3f71f4e2af210b8`
- Working tree at capture: only untracked `.autors/` and `research/` present
  (no tracked source modifications).

## Tracked PM Rust-file set (frozen input)

- Enumerated with `git ls-files 'src/kernel/src/pm/*.rs'`.
- Count: **66** files.
- Full list: `argus-pm-artifacts-20260824/pm_files.list`.
- Pre-run SHA-256 of every file: `argus-pm-artifacts-20260824/hashes/pm_pre.sha256`.
- Byte backup of every file: `argus-pm-artifacts-20260824/pm-backup/<relpath>`.
- This exact list is passed to BOTH `--inject-files` and `--scan-files`.
- No reachability manifest / scope manifest is created or used.

## Verus release identity

- Version: `0.2026.08.23.fbbbbcf` (Profile: release, linux_x86_64,
  toolchain 1.97.1).
- Binary: `/home/ruize/verus-exp/verus-main-20260824/source/target-verus/release/verus`
- Verus source HEAD: `fbbbbcf3085c4c14f5566690eced4ed3bd659fed`
  (branch `build/argus-pm-20260824`).

## Dummy-probe tool checkout

- Original (user-owned, READ-ONLY, uncommitted dirty changes preserved):
  `/home/ruize/verus-ai-exp/verus-ai`
  - HEAD: `48d0fc5e6d1a878f7fa034dd54b4c0652ff154fa`
  - Uncommitted status: `argus-pm-artifacts-20260824/hashes/tool_orig_status.txt`
  - Uncommitted diff: `argus-pm-artifacts-20260824/hashes/tool_orig_uncommitted.diff`
  - Script hashes: `argus-pm-artifacts-20260824/hashes/tool_orig_scripts.sha256`
  - Verified UNCHANGED after snapshot (HEAD still `48d0fc5e...`).
- Isolated byte-faithful snapshot (used for all runs):
  `/home/ruize/verus-ai-exp/verus-ai-argus-pm`
  - HEAD: `48d0fc5e6d1a878f7fa034dd54b4c0652ff154fa`
  - `scripts/dummy_probe.py`, `inject_verus_verify.py`, `dummy_probe_report.py`
    byte-identical to original (`SCRIPTS_BYTE_MATCH_OK`).
  - `argus-pm-artifacts-20260824/hashes/tool_snapshot_scripts.sha256`

## Build contract (from Makefile / rust-toolchain / target)

- Crate: `kernel` (`src/kernel`, `[package.metadata.verus] verify = true`).
- Module focus: `kernel::pm`.
- Toolchain channel: `nightly-2026-07-09` (`rust-toolchain`).
- Kernel RUSTFLAGS: `-C relocation-model=static -C prefer-dynamic=no`.
- Kernel features: `microvm trace` (`VERUS_KERNEL_FEATURES`).
- build-std args: `-Z build-std=core,alloc,compiler_builtins`
  `-Z build-std-features=compiler-builtins-mem -Z json-target-spec`.
- Target: `build/targets/x86-kernel.json`.
- `config` crate `build.rs` requires `MEMORY_SIZE_BYTES`; set to
  `134217728` (128 MiB) for the run.

## Artifact root

`/home/ruize/argus-pm-artifacts-20260824/` (outside the Nanvix repo).
Fresh external `CARGO_TARGET_DIR` = `.../target/cargo-target`.

## Round 1 blocker + tool fix (pre-fix run archived)

Pre-fix baseline run (`run-1`) failed immediately (round 0, 0.02s,
`stop_reason=unparsed_run_failure`, `layered_complete=false`). Concrete
first blocker — this Verus rejects `--verify-module` under `cargo verus
verify`:

```
partial verification selector: `--verify-module`
Error: Partial verification must use `cargo verus focus`
```

Archived evidence:
`argus-pm-artifacts-20260824/probe/pm_baseline_prefix_failed.json`,
`.../run-1-prefix-failed/`.

### Tool improvement (isolated snapshot only)

`scripts/dummy_probe.py :: run_verus_cargo` now emits `cargo verus focus`
(instead of `verify`) whenever a specific `--verify-module` is requested;
`verify` is kept for whole-crate / `--verify-root`. Committed in the
snapshot repo `/home/ruize/verus-ai-exp/verus-ai-argus-pm`:
- `dbebdc0dd` — provenance baseline (as-delivered dummy_probe.py + tests).
- `cee10ceef` — `fix(dummy_probe): drive --verify-module through cargo verus focus`
  (+ focus/verify/root regression tests, 3 passing).
The original user checkout `/home/ruize/verus-ai-exp/verus-ai` is untouched.

### Module-selector realization

Verus reports modules **crate-relative** (`mm::phys`, `hal::...` — no
`kernel::` prefix). `--verify-module MODULE` "Verify just one submodule and
its **descendants**", crate-relative (`'foo'` or `'foo::bar'`). Therefore the
faithful selector for the kernel `pm` module (covering all 66 files incl.
`pm::process`, `pm::kcall`, …) is `pm`, not `kernel::pm`. The probe wrapper
passes `--module pm`.

## Round 2 baseline (corrected tool) — runs, INCOMPLETE at unresolved_frontier

Command/env: `argus-pm-artifacts-20260824/probe/run-2/{command.txt,environment.txt,manifest.json}`.
Fresh `CARGO_TARGET_DIR` = `.../target/cargo-target` (built 467 MB incl.
libcore/libvstd artifacts — real build, focus mode). Verus selector:
`cargo verus focus -p kernel --no-default-features --features microvm,trace
... --verify-module pm` (+ `--multiple-errors 4 --expand-errors`, full-verify).

Result (`schema=dummy-probe/v2`, full `rounds_trace` present):
- `summary = {LIMITATION: 65, INCONCLUSIVE: 3}` (68 verdict rows).
- `layered.rounds = 3`, `stop_reason = unresolved_frontier`,
  `layered_complete = false` (max_rounds=100 not reached).
- Prepass: stripped 0 EB attrs; inserted 698 `#[verus_verify]` into 64 files.
- Restoration: **all 66 pm files byte-identical to pre-run** (independently
  re-hashed; `git status` shows 0 changed pm paths).

Artifacts: `pm_baseline_run-2.json`, `pm_baseline_run-2_report.md` (87 KB),
`pm_baseline_run-2_families.json`.

### Raw Verus frontend-diagnostic families surfaced (pre-triage)
| n | family |
|---|--------|
| 20 | `mut self` receiver unsupported |
| 10 | unsupported cast (int→pointer, e.g. `usize`→`*const T`) |
| 10 | "only variables are supported here, not general patterns" |
|  7 | dereferencing a pointer (implicit) |
|  6 | datatype constructor used as a function value |
|  3 | complex break expressions |
|  3 | `Path Def(Static { …, Mut … })` (static-mut path) |
|  2 | internal item statements |
|  2 | verus internal error: "expected array" (INCONCLUSIVE — candidate ICE) |
|  1 | casting a pointer (implicit) |
|  1 | array-fill expression with non-copy type |
|  1 | `panic!` not supported |
|  1 | `static mut` not supported |
|  1 | `unsafe impl` (for `Send`) not allowed (INCONCLUSIVE) |

Triage of these into genuine-Verus vs project vs architecture vs
false-positive is the NEXT node's work, not this baseline node.

### Concrete incompleteness blocker (first)
`stop_reason = unresolved_frontier` after round 2: round 2 discovered 20
limitations but produced `new_shield_targets = []`. The layered peel shields
a limitation fn with `#[verifier::external_body]`, which suppresses BODY-level
limitations but NOT signature/item-level ones (`mut self` receiver, `static
mut` paths, `unsafe impl`, the "expected array" internal error). Those cannot
be masked, so no new shieldable declaration is derivable and the peel cannot
reach a green fixed point. This is the first concrete blocker to a
`fixed_point` baseline; the 68 diagnostics are still fully enumerated in the
per-round trace. Reaching a true fixed point requires a peel strategy for
signature/item-level limitations (candidate dummy-probe improvement) or an
explicit decision to accept item-level limitations as terminal — deferred to
the next node.
