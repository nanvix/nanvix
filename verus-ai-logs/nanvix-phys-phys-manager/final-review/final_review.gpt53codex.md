# Spec Quality

Scope: `PhysMemoryManager::init`, `alloc_user_frame`, `check_user_watermark`, `alloc_many_user_frames`, `alloc_many_kernel_frames`, `alloc_kernel_frame` in `src/kernel/src/mm/phys/manager.rs`.

- `init` (`manager.rs:96-104`): **Weak / blocker context**. Spec is `external_body` and both arms assert only `phys_view().manager_ready` (`manager.rs:100-102`). Missing caller-relevant error contract (`InvalidArgument`/double-init) and missing state-preservation detail. Match arms are effectively redundant.
- `alloc_user_frame` (`manager.rs:280-296`): Good state transition (`alloc_one`) and watermark gate on `Ok`; `Err` encodes no-state-change + gate failure. Does **not** encode expected error code (`OutOfMemory`).
- `check_user_watermark` (`manager.rs:320-327`): Good bidirectional gate (`Ok` iff enough free frames, `Err` iff below threshold). Error-code distinction (overflow InvalidArgument vs breach OutOfMemory) not in contract.
- `alloc_many_user_frames` (`manager.rs:173-191`): Good all-or-nothing frame condition and booking semantics. Missing explicit `capacity >= count` precondition contract; `Err` arm does not encode watermark rejection condition (`!user_alloc_ok(count)`) nor error-code meaning.
- `alloc_many_kernel_frames` (`manager.rs:417-435`): Good contiguity + all-or-nothing semantics. Requires `count > 0` and empty vec; missing explicit `capacity >= count` precondition contract.
- `alloc_kernel_frame` (`manager.rs:363-374`): Good one-frame transition and no-leak `Err` frame condition. No liveness guarantee when free frames exist (caller analysis expects kernel path to bypass watermark, but no success condition beyond transition-on-Ok).

Overall: contracts are partially strong on state transitions, but external-top quality is incomplete on several caller-facing failure semantics.

# Caller Coverage (Covered N/Total, Missing list)

From `caller_analysis.md` explicit expectations, I counted **13** expectation items (Ok/Err/precondition bullets across six targets). **Covered: 8/13**.

## Missing / incomplete mappings
1. `init` Err semantics (double-init only, `InvalidArgument`) not specified (`manager.rs:97-103`; expected at `caller_analysis.md:64-65`).
2. `alloc_many_kernel_frames` caller storage precondition `capacity >= count` not in `requires` (`caller_analysis.md:96-97`; runtime check only at `manager.rs:456-461`).
3. `alloc_many_user_frames` caller storage precondition `capacity >= count` not in `requires` (`caller_analysis.md:110`; runtime check only at `manager.rs:211-216`).
4. `alloc_many_user_frames` Err expected watermark-gated rejection semantics (`caller_analysis.md:105-107`) not encoded as `!old(self)@.user_alloc_ok(count)` in Err arm (`manager.rs:187-190`).
5. `alloc_user_frame` Err expected `OutOfMemory` watermark rejection (`caller_analysis.md:119-121`) not encoded in ensures (`manager.rs:291-294`).

# Proof Completeness (admit count+locations, external_body count+locations)

Across `manager.rs` + `manager.spec.rs` + `manager.proof.rs`:

- `admit()` count: **4** (**BLOCKER**)
  - `manager.proof.rs:16` (`lemma_manager_attached`)
  - `manager.proof.rs:35` (`lemma_kernel_alloc_one`)
  - `manager.proof.rs:55` (`lemma_kernel_alloc_contiguous`)
  - `manager.proof.rs:216` (`lemma_user_bulk_err_restored`)

- `external_body` count: **2**
  - `manager.rs:96` (`PhysMemoryManager::init`, fn at `manager.rs:104`)
  - `manager.rs:524` (`kernel_watermark`, fn at `manager.rs:529`)

# TCB Compliance (each external_body: listed YES/NO + rationale assessment)

1. `PhysMemoryManager::init` (`manager.rs:96/104`)  
   - Listed in allow-list: **YES** (`tcb-allowed.md:86-87`)  
   - Rationale quality: **FAIL / BLOCKER**. Entry says “no specs yet; opaque callee,” but this function now has `#[verus_spec]` and is an explicit target. This rationale is stale and masks unverified in-scope code.

2. `kernel_watermark` (`manager.rs:524/529`)  
   - Listed in allow-list: **YES** (`tcb-allowed.md:132-140`)  
   - Rationale quality: **Acceptable** (build-time constant from non-Verus crate boundary).

# Guardrails Compliance (admit/assume/external_body/assume_specification/cfg-gated exec exact counts)

Counts in scoped files (`manager.rs`, `manager.spec.rs`, `manager.proof.rs`):

- `admit`: **4** (**BLOCKER**) — locations above.
- `assume(...)`: **0**.
- `external_body`: **2** — locations above.
- `assume_specification`: **3**
  - `manager.spec.rs:9` (`Result::and_then`)
  - `manager.spec.rs:23` (`Result::inspect_err`)
  - `manager.spec.rs:33` (`Vec::capacity`)
- `#[cfg(not(verus_keep_ghost))]` / `#[cfg(verus_keep_ghost)]`: **11**
  - `manager.rs:8,10` (include/import of spec/proof; allowed)
  - `manager.rs:207,213,339,345,382,385,452,458,500` (logging only `error!/warn!`; allowed)

No `assume(...)` found. `admit > 0` is a blocker per your guardrail.

# AST Consistency (PASS/FAIL + details)

**FAIL (BLOCKER)**

Command run: `python3 .../ast_consistency.py --base-ref origin/dev src/kernel/src/mm/phys/manager.rs ...`

- Summary: **4 mismatched functions, 1 extra function, 3 matched**.
- MISMATCH functions:
  - `PhysMemoryManager::alloc_kernel_frame`
  - `PhysMemoryManager::alloc_many_kernel_frames`
  - `PhysMemoryManager::alloc_many_user_frames`
  - `PhysMemoryManager::check_user_watermark`
- EXTRA_IN_VERUS:
  - `kernel_watermark`

`// VERUS REWRITE` comments: **0** (`verus_rewrite_count=0`).

Per your rule (“Any MISMATCH is a blocker”), this section is blocker-fail.

# Verification (PASS/FAIL + error count, from /tmp/verify_mmphys.log)

From `/tmp/verify_mmphys.log` (tail read at review time):

- Verus run exit code: **0**
- Reported verifier error count: **0** (no verification error lines; cached success)
- Cheating status in same log: **CHEATING_DETECTED**
  - Module `mm::phys`: `external_body=17`, `admit=12`, `cfg-gated exec code=12`
  - Global: `assume=0`, `external_body=18`, `admit=24`, `cfg_gate=15`

So: verification execution succeeded, but quality gate failed due cheating patterns.

# Bug Summary (total recorded, true bugs w/ severity, reconciliation)

From `verus-ai-logs/nanvix-phys-phys-manager/bugs.md`:

- Total recorded entries: **4** (`OBS-1`, `OBS-2`, `OBS-3`, `BUILD-1`).

Reconciliation:
- `OBS-1` (`count==0` kernel bulk path): **Still valid (Context-Dependent)**. Current contract keeps `requires count > 0` (`manager.rs:421`), no code fast-path.
- `OBS-2` (user bulk distinctness depends on allocator non-aliasing): **Still valid (Context-Dependent)**. Depends on trusted `Upool::alloc` contract chain.
- `OBS-3` (unsound err-empty lemma): **Fixed** (lemma removed; not present in current proof file).
- `BUILD-1` (unused loop variable): **Fixed** (`_idx` in `manager.rs:244`).

New issue not recorded in `bugs.md`:
- **TCB rationale drift for `init`** (allow-list text stale/outdated for in-scope target) — should be tracked explicitly.

# Issues (highest priority first)

1. **BLOCKER:** `admit()` remains (**4**) in `manager.proof.rs` (`16,35,55,216`).
2. **BLOCKER:** AST consistency fails (4 mismatches + 1 extra function).
3. **BLOCKER:** `PhysMemoryManager::init` remains `external_body` with stale allow-list rationale (`tcb-allowed.md:86-87`).
4. **MAJOR:** Caller expectation gaps in failure semantics/preconditions (init Err semantics; bulk capacity preconditions; user Err watermark/error-code mapping).

# Result: FAIL

Reason: blockers present (notably `admit_count=4`; AST mismatch fail; stale `init` trust rationale).
