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

## Run-5 — closure-parameter general-patterns family eliminated (10 → 0)

Node `triage-general-pattern-family`. Rewrote every non-variable closure
**parameter** in the six PM source files to a named variable plus equivalent
field access / dereference (16 closure params: the 10 run-4-reported sites plus
6 same-family sites masked by `--multiple-errors 4`). The `for (i, _)` loop
pattern is a loop binding, not a closure parameter, and Verus accepts it —
left unchanged.

- Reproducer (`argus-pm-artifacts-20260824/reproducer/`): `closure_pattern_bad.rs`
  → 6 `only variables are supported here, not general patterns` errors on
  Verus `0.2026.08.23.fbbbbcf`; `closure_pattern_good.rs` → `7 verified, 0 errors`
  (exit 0, passes frontend lowering); `legal_rust_check.rs` → ordinary `rustc`
  exit 0 (legal Rust). Full write-up: `reproducer/CLOSURE_PATTERN_LIMITATION.md`.
- Fresh full 66-file layered probe `run-5` (fresh isolated `CARGO_TARGET_DIR`
  `.../target/cargo-target-run5`, direct `--inject-files`/`--scan-files` 66-file
  lists, `--max-layer-rounds 100`, no reachability manifest):
  `summary={LIMITATION:35, INCONCLUSIVE:3}` (38 rows, down from run-4's 48);
  `layered.rounds=3`, `stop_reason=fixed_point`, `layered_complete=True`,
  `shielded=37`. Artifacts: `probe/pm_acceptance_run-5.json`,
  `probe/pm_acceptance_run-5_report.md`, `probe/run5_family_delta.json`.
- Per-family delta (`run5_family_delta.json`): the general-patterns family
  `10 → 0`; **every other family unchanged; no new family exposed**.
- Restoration: all 66 pm files byte-identical to the pre-run rewritten source
  (`hashes/pm_run5_pre.sha256` vs `hashes/pm_run5_restore_check.txt`, 66 OK /
  0 mismatch).
- Compilation: the probe's `cargo verus` full build of `kernel` succeeded
  (verification results were produced), confirming the rewritten source compiles
  under the x86-kernel target with `microvm,trace`.

## Run-6 — datatype-constructor-as-function-value family eliminated (6 → 0)

Node `triage-datatype-constructor-family`. Rewrote every site that passes a bare
datatype-constructor path as a first-class function value to a combinator
(`.map`/`.map_err`) into an equivalent explicit closure. Verus rejects
`.map(Ctor)` (a constructor is not a first-class function value in the frontend);
the behaviour-identical `.map(|x| Ctor(x))` lowers cleanly.

- **Exhaustive source inventory (not the 6 run-5-reported declarations):** a
  regex over all 66 pm files found **11 constructor-as-function-value sites across
  6 files** — one more file than run-5 surfaced. The extra site,
  `pm/kcall/join_thread.rs:75` (`.map_err(SleepError::Generic)`), was **masked**
  in run-5: `join_thread` was already shielded for its unrelated `u32 → *mut
  ExitStatus` cast limitation, which suppressed its body-level constructor
  diagnostic. Exhaustive SOURCE scanning (not trusting the 6 reported rows) was
  therefore essential. The six edited files / sites:
  - `pm/kcall/join_thread.rs` @75 — `SleepError::Generic` (masked site)
  - `pm/kcall/lock_mutex.rs` @96,98 — `SleepError::Generic`
  - `pm/kcall/wait_cond.rs` @109,127,130,132 — `SleepError::Generic`
  - `pm/process/manager/delivery/delivery_sequence.rs` @44 — `Self` (tuple struct)
  - `pm/process/manager/mod.rs` @2616 — `Err`
  - `pm/process/state/zombie.rs` @83,104 — `ThreadRef::Zombie`/`ThreadRefMut::Zombie`
- Reproducer (`argus-pm-artifacts-20260824/reproducer/`):
  `constructor_fn_value_bad.rs` → **5** `using a datatype constructor as a function
  value` errors on Verus `0.2026.08.23.fbbbbcf` (`aborting due to 5 previous
  errors`); `constructor_fn_value_good.rs` (explicit closures) → `6 verified, 0
  errors` (exit 0, passes frontend lowering); `constructor_legal_rust_check.rs` →
  ordinary `rustc` exit 0 (the rejected form is legal Rust). Full write-up:
  `reproducer/CONSTRUCTOR_LIMITATION.md`.
- **Clippy tension resolved:** Verus rejects `.map(Ctor)`, but clippy's
  `redundant_closure` (inside `clippy::all = deny`) rejects the fix `|x| Ctor(x)`.
  The pre-commit hook runs `lint-check` and the harness commits without
  `--no-verify`, so both must pass. Resolution: explicit closure plus a
  function-level `#[allow(clippy::redundant_closure)]` with an explanatory comment
  (an existing codebase idiom, robust across clippy versions). On `mod.rs` the
  existing `#[allow(clippy::type_complexity)]` was extended in place.
- Fresh full 66-file layered probe `run-6` (fresh isolated `CARGO_TARGET_DIR`
  `.../target/cargo-target-run6`, direct `--inject-files`/`--scan-files` 66-file
  lists, `--max-layer-rounds 100`, no reachability manifest):
  `summary={LIMITATION:29, INCONCLUSIVE:3}` (32 rows, down from run-5's 38);
  `layered.rounds=3`, `stop_reason=fixed_point`, `layered_complete=True`,
  `shield_events=31`. Artifacts: `probe/pm_acceptance_run-6.json`,
  `probe/pm_acceptance_run-6_report.md`, `probe/run6_family_delta.json`.
- Per-family delta (`run6_family_delta.json`): the constructor family `6 → 0` is
  the **only** changed family; the other 14 families are byte-identical
  (`deref 7→7`, `casts` all unchanged incl. `join_thread`'s `u32→*mut ExitStatus
  1→1`, `general-patterns` stayed `0`, `complex break 3→3`, `Path Def 3→3`,
  `internal 2→2`, `expected array 2→2`, `panic 1→1`, `unsafe impl Send/Sync 1/1`,
  `static mut 1→1`). Per-declaration diff: exactly **6 declarations removed** (the
  six constructor declarations — `lock_mutex`, `wait_cond`, `checked_next`,
  `try_join_thread`, `find_thread`, `find_thread_mut` — now verify clean),
  **0 declarations added** (no previously-masked limitation exposed by the fix),
  **0 shared declarations changed family**. `join_thread` remains a limitation
  under its unchanged cast family, as expected (its constructor site is fixed but
  the cast persists).
- Restoration: all 66 pm files byte-identical to the pre-run rewritten source
  (`hashes/pm_run6_pre.sha256` vs `hashes/pm_run6_restore_check.txt`, 66 OK /
  0 mismatch).
- Ordinary Nanvix check on the edited tree (not the probe): `cargo clippy --locked
  -p kernel ... -- -D warnings` (build-std, x86-kernel target, `microvm,trace`)
  exit 0 with the `kernel` crate compiled (not cached); `cargo fmt -p kernel
  --check` clean; codespell clean.

## Run-7 — complex-break (value-carrying `break EXPR`) family eliminated (3 → 0)

Node `triage-complex-break-family`. Rewrote every value-carrying `break EXPR` (a
`break` that carries a value out of a `loop`) into its behaviour-preserving
equivalent. Verus `0.2026.08.23.fbbbbcf` rejects `break VALUE` during frontend
lowering with `The verifier does not yet support the following Rust feature:
complex break expressions`; a plain `break;` is accepted. The two accepted forms
are `return VALUE` (function-tail loops) and assign-then-plain-`break;` with a
block-tail read (value-position loops).

- **Exhaustive source inventory (independent of diagnostic caps):** a `\bbreak\b`
  scan over all 66 pm files found exactly **4 value-carrying break sites across 3
  files / 3 declarations**; every other `break` is plain (`break;` / `break,`) or
  a comment. No masked extra site. The three edited files / sites:
  - `pm/sync/mutex.rs` @179 — `Ok(guard) => break Ok(guard)` → `return Ok(guard)`
    (tail loop = whole function body).
  - `pm/process/manager/unsafe.rs` @773,780 — `break Ok(status)` /
    `break Err(SleepError::Generic(error))` → `return …` (tail loop; two sites in
    the one `join_thread` declaration).
  - `pm/process/manager/signal.rs` @257 — tuple `break (signum, entry, mask,
    flags)` → pre-declared `selected = (…)` then plain `break;`, block tail reads
    `selected` (value-position loop; definite-assignment confirmed by a standalone
    `rustc --edition 2021` compile).
- Reproducer (`argus-pm-artifacts-20260824/reproducer/`):
  `complex_break_bad.rs` → **3** `complex break expressions` errors on Verus
  `0.2026.08.23.fbbbbcf` (`aborting due to 3 previous errors`);
  `complex_break_good.rs` (return / assign-then-plain-break + `decreases`) → `7
  verified, 0 errors` (exit 0, passes frontend lowering AND verifies);
  `complex_break_legal_rust_check.rs` → ordinary `rustc --edition 2021` exit 0 (the
  rejected form is legal Rust). Full write-up: `reproducer/COMPLEX_BREAK_LIMITATION.md`.
- Fresh full 66-file layered probe `run-7` (fresh isolated `CARGO_TARGET_DIR`
  `.../target/cargo-target-run7`, direct `--inject-files`/`--scan-files` 66-file
  lists, `--layered --exhaustive --max-layer-rounds 100`, no reachability
  manifest): `summary={LIMITATION:26, INCONCLUSIVE:3}` (29 rows, down from run-6's
  32); `layered.rounds=3`, `stop_reason=fixed_point`, `layered_complete=True`,
  `shield_events=28`. Artifacts: `probe/pm_acceptance_run-7.json`,
  `probe/pm_acceptance_run-7_report.md`, `probe/run7_family_delta.json`,
  `probe/run-7/` (supervisor verdict, lint log, manifest). Supervised subagent
  `pm-complex-break-run7-1787546951` exit 0, 59 s.
- Per-family delta (`run7_family_delta.json`): the complex-break family `3 → 0` is
  the **only** changed family; every other family is byte-identical (`deref 7→7`,
  every `usize/u32 → *…` cast unchanged, `Path Def 3→3`, `internal 2→2`,
  `array-fill 1→1`, `panic 1→1`, `static mut 1→1`, `expected array 2→2`
  [INCONCLUSIVE], `unsafe impl Send/Sync 1/1` [INCONCLUSIVE]). Terminal frontier
  fully enumerated (`attribution_gaps=[]`); residual verdicts are the 26 remaining
  LIMITATION rows plus 3 INCONCLUSIVE.
- Restoration: all 66 pm files byte-identical to the pre-run rewritten source
  (`hashes/pm_run7_pre.sha256` vs `hashes/pm_run7_restore_check.txt`, 66 OK /
  0 mismatch).
- Ordinary Nanvix check on the edited tree (probe Step 1): `make
  rust-lint-check-kernel MACHINE=microvm LOG_LEVEL=trace RELEASE=no` (clippy
  `-D warnings`, build-std, x86-kernel) exit 0 — `probe/run-7/lint_check_kernel.log`.

## Run-8 — internal item statements (items nested in a fn body) family eliminated (2 → 0)

Node `triage-internal-item-statement-family`. Hoisted every item declared as a
statement inside a PM function body to module scope. Verus `0.2026.08.23.fbbbbcf`
rejects an inner item with `The verifier does not yet support the following Rust
feature: internal item statements` (a `StmtKind::Item` in a fn body); a module-scope
`static` / `extern` block has identical linkage/semantics and is accepted. Verus
also accepts inner `const` and nested `fn`, so those PM sites are left unchanged.

- **Exhaustive source inventory (independent of diagnostic caps):** a brace-tracked
  scan of all 66 pm files for item keywords (`static`, `extern`, `fn`, `struct`,
  `enum`, `union`, `trait`, `impl`, `mod`, `type`, `macro_rules`) inside a function
  body found exactly **2 `static`/`extern` inner-item sites in 2 declarations, both
  in `process/manager/mod.rs`**. The other inner items surfaced by the scan are all
  function-local `const` (accepted by Verus) and are left untouched. No masked
  extra `static`/`extern` site. The two edited sites:
  - `process/manager/mod.rs` `forge_user_context` — `unsafe extern "C" { pub fn
    __leave_kernel_to_user_mode(); }` → module-scope `unsafe extern "C" { fn
    __leave_kernel_to_user_mode(); }` (symbol defined once in the arch asm hooks).
  - `process/manager/mod.rs` `write_nul_terminated_to_user` — `static NUL: u8 = 0;`
    → module-scope `static NUL: u8 = 0;`.
- Reproducer (`argus-pm-artifacts-20260824/reproducer/`):
  `internal_item_bad.rs` (inner `static` + inner `extern` block) → **2** `internal
  item statements` errors on Verus `0.2026.08.23.fbbbbcf`; `internal_item_good.rs`
  (both hoisted to module scope, + inner-`const` / nested-`fn` positive contrasts)
  → `7 verified, 0 errors` (exit 0, passes frontend lowering AND verifies);
  `internal_item_legal_rust_check.rs` → ordinary `rustc --edition 2021` exit 0 (the
  rejected form is legal Rust). Full write-up: `reproducer/INTERNAL_ITEM_LIMITATION.md`.
- Fresh full 66-file layered probe `run-8` (fresh isolated `CARGO_TARGET_DIR`
  `.../target/cargo-target-run8`, direct `--inject-files`/`--scan-files` 66-file
  lists, `--layered --exhaustive --max-layer-rounds 100`, no reachability manifest):
  `summary={LIMITATION:26, INCONCLUSIVE:3}` (29 rows); `layered.rounds=3`,
  `stop_reason=fixed_point`, `layered_complete=True`, `shield_events=28`, terminal
  frontier `fully_enumerated=True` / `attribution_gaps=[]`. Artifacts:
  `probe/pm_acceptance_run-8.json`, `probe/pm_acceptance_run-8_report.md`,
  `probe/run8_family_delta.json`, `probe/run-8/`.
- Per-family delta (`run8_family_delta.json`): the internal-item-statement family
  `2 → 0` (eliminated everywhere in PM). **Masked-limitation exposure (documented,
  non-regression):** removing the item-statement error unshields each declaration's
  next frontend limitation, so `forge_user_context` now surfaces its `debug_assert!`
  **panic** (panic `1 → 2`) and `write_nul_terminated_to_user` its implicit pointer
  **dereference** (deref `7 → 8`). No other family changed. This mirrors run-6's
  `join_thread` (constructor fixed, underlying cast persists): the targeted category
  is genuinely closed; the surfaced panic/deref are the same fundamental
  non-actionable limitations already dominating the residual set. Net finding count
  stays 29 (2 internal findings replaced by 1 panic + 1 deref).
- Restoration: all 66 pm files byte-identical to the pre-run rewritten source
  (`hashes/pm_run8_pre.sha256` vs `hashes/pm_run8_restore_check.txt`, 66 OK /
  0 mismatch).
- Ordinary Nanvix check on the edited tree (probe Step 1): `make
  rust-lint-check-kernel MACHINE=microvm LOG_LEVEL=trace RELEASE=no` (clippy
  `-D warnings`, build-std, x86-kernel) exit 0 — `probe/run-8/lint_check_kernel.log`.

### Residual actionability after run-8

Actionable frontend-limitation categories closed so far: general-patterns (run-5),
datatype-constructor (run-6), complex-break (run-7), internal-item-statement
(run-8). The remaining 26 LIMITATION + 3 INCONCLUSIVE findings are fully located
and evidence-classified, but they are not yet a demonstrated
no-actionable-category fixed point:
- integer→pointer / pointer→integer **casts** (11) — the fundamental user-copy /
  MMIO address machinery (`… as *const/*mut T`, `… as usize`);
- raw-pointer **dereferences** (8) — the copy_from/to_user and context/fpu accessors;
- **Path Def** (3, `PROCESS_MANAGER` accessors) and **static mut** (1);
- **panic** (2) — `debug_assert!`/`assert!` lowerings;
- **array-fill** (1) — still actionable-candidate evidence, not a closed
  classification: `LifecycleQueueChunk::new` uses
  `[const { None }; LIFECYCLE_QUEUE_CHUNK_CAPACITY]`, while this PM tree already
  contains Verus-accepted `core::array::from_fn(|_idx| ...)` calls. A minimized
  latest-Verus comparison and a PM trial of `array::from_fn(|_idx| None)` have not
  yet been recorded;
- INCONCLUSIVE: **expected array** ICE (2), **unsafe impl Send/Sync** (1).
The panic family remains non-actionable absent a behavior-identical accepted
lowering: deleting or replacing `debug_assert!` changes debug-build panic
semantics, while moving it to a helper only moves the diagnostic. The cast,
deref, `Path Def`, `static mut`, and unsafe-impl families remain tied to PM's
runtime address/unsafe/concurrency semantics; the expected-array diagnostics
remain verifier ICEs. Each residual is evidence-classified as a genuine Verus
frontend limitation or ICE rather than a Nanvix defect, but the array-fill
candidate keeps campaign delivery open.
