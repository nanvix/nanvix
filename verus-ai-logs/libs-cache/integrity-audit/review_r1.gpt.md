# Integrity Audit Review — GPT (gpt-5.3-codex)

## 1. Cheating Counts

| Pattern | Count | Locations |
|---|---:|---|
| `admit()` | 0 | none (checked `lib.rs`, `lib.spec.rs`, `lib.proof.rs`, `lib.vstd_btree.rs`) |
| `assume()` | 0 | none |
| `#[...external_body]` | 8 | `lib.rs:93` (`CacheGuard::deref`), `lib.rs:114` (`btreemap_remove`), `lib.rs:190` (`Cache::get`), `lib.rs:230` (`Cache::put`), `lib.rs:315` (`Cache::find_lru_victim`), `lib.spec.rs:24` (`ExCacheGuard`), `lib.proof.rs:401` (`axiom_cache_lru_of_remove`), `lib.vstd_btree.rs:32` (`ExBTreeMap`) |
| `trusted` | 0 | none |
| `exec_allows_no_decreases_clause` | 0 | none |
| cfg-gated exec code | 0 suspicious | only `lib.rs:50/52/54` (ghost includes) and `lib.rs:367` (test module) |
| `assume_specification` | 5 | `lib.vstd_btree.rs:69,88,98,108,130` |
| `broadcast axiom` | 2 | `lib.vstd_btree.rs:56`, `lib.vstd_btree.rs:80` |

Cross-check vs `fix_report.md`: counts match numerically (`external_body=8`, `assume_specification=5`, `broadcast axiom=2`, others 0).

## 2. Trust Item Challenges

a) **ExBTreeMap** (`lib.vstd_btree.rs:31-38`)  
- Classification: `EXTERNAL_TYPE`  
- Challenge: upstream vstd also declares `ExBTreeMap` with `external_type_specification + external_body` (`std_specs/btree.rs:432-437`).  
- Verdict: **KEEP** (not eliminable by ladder).

b) **ExCacheGuard** (`lib.spec.rs:23-25`)  
- Classification: Verus `&mut` struct-field limitation.  
- Challenge: struct has `value: &'a mut V` (`lib.rs:86`), which is unsupported in normal verified struct bodies.  
- Verdict: **KEEP**.

c) **btreemap_remove** (`lib.rs:114-123`)  
- Classification: stdlib wrapper.  
- Challenge: upstream vstd has generic `remove::<Q>` spec for `std::collections::BTreeMap` (`std_specs/btree.rs:776-791`). Local no_std path uses `alloc::collections::BTreeMap`; direct equivalent was not demonstrated here with a local reproducer.  
- Verdict: **KEEP (conditionally)**, but evidence is incomplete; claim “assume_specification cannot express this for alloc path” is not fully substantiated in current artifacts.

d) **CacheGuard::deref** (`lib.rs:93-99`)  
- Classification: depends on opaque external type.  
- Challenge: body reads opaque field from `ExCacheGuard`; impossible to body-verify without removing (b).  
- Verdict: **KEEP**.

e) **Cache::get** (`lib.rs:190-218`)  
- Classification: Verus limitation + API shape (`Option<CacheGuard<'_,V>>`).  
- Challenge: relies on `get_mut` and constructs `CacheGuard` with `&mut`; cannot eliminate without changing public API or proving unsupported `&mut` flow.  
- Verdict: **KEEP**.

f) **Cache::put** (`lib.rs:230-265`)  
- Classification: Verus limitation around `get_mut` path.  
- Challenge: rewrite to avoid `get_mut` is possible operationally, but current proof architecture (uninterpreted `cache_lru_of`) would require additional trust/axioms; no net trust reduction shown.  
- Verdict: **KEEP**.

g) **find_lru_victim** (`lib.rs:315-331`)  
- Classification: iterator/min combinator limitation in this no_std setup.  
- Challenge: vstd search found no `min_by_key` specs; no BTreeMap iterator-for-loop support available from local alloc adaptation.  
- Verdict: **KEEP**.

h) **axiom_cache_lru_of_remove** (`lib.proof.rs:401-411`)  
- Classification: axiomatic bridge for uninterpreted `cache_lru_of_nonempty`.  
- Challenge: cannot prove from current abstractions; proving requires exposing/defining concrete ordering relation over map entries.  
- Verdict: **KEEP**.

i) **5 assume_specifications** (`lib.vstd_btree.rs`)  
- Faithfulness to upstream: **not fully faithful**.  
  - `new`, `is_empty`, `clear`: faithful.  
  - `len` bridge axiom and `insert` dropped upstream `key_obeys_cmp_spec` / `obeys_cmp_spec` guards (upstream: `std_specs/btree.rs:586`, `630`).  
- Verdict: **KEEP but WEAKENING NEEDED** (guards should be restored).

j) **2 broadcast axioms** (`lib.vstd_btree.rs`)  
- `axiom_btree_map_view_finite_dom`: standard and aligned with vstd intent.  
- `axiom_spec_btree_map_len`: locally stronger than upstream due missing guard; risk for pathological `Ord` implementations.  
- Verdict: **1 sound, 1 over-strong (needs guard).**

k) **Counter overflow trust assumption**  
- Classification in `bugs.md` as low-risk physical-limit assumption is reasonable, but still real trust debt for long-lived systems.  
- Verdict: **KEEP (documented limitation).**

l) **deref_mut exclusion**  
- `deref_mut` returns `&mut V`; currently unverifiable in this setup.  
- Verdict: **KEEP**.

## 3. AST Consistency

Independent run (`python3 /home/ruize/verus-ai-exp/verus-ai-lru-0422/scripts/ast_consistency.py src/libs/cache/src/lib.rs summary`) confirms: 15 MATCH, 3 MISMATCH, 2 EXTRA.

- **MISMATCH `Cache::new`**: `Self{...}` rewritten to `let result = Self{...}; ...; result` plus proof call.  
  - Pre-approved? **Partially** (named-return style is pre-approved family).  
  - Semantics: preserved.  
  - `VERUS REWRITE` comment: **missing**.

- **MISMATCH `Cache::remove`**: `self.entries.remove(key)` -> `btreemap_remove(...)` + proof call.  
  - Pre-approved? **No** (not in pre-approved table; justified wrapper pattern).  
  - Semantics: preserved (wrapper body is `m.remove(k)`).  
  - `VERUS REWRITE` comment: **present**.

- **MISMATCH `Cache::evict`**: iterator chain extracted into helper + remove wrapper.  
  - Pre-approved? **No** (non-table rewrite).  
  - Semantics: preserved by direct extraction.  
  - `VERUS REWRITE` comment: **present**.

- **EXTRA `Cache::find_lru_victim`**: extracted original iterator chain; semantics preserved; comment present.
- **EXTRA `btreemap_remove`**: thin stdlib wrapper; semantics preserved; no rewrite comment in function body (but documented around callsites).

## 4. Bug vs Limitation

| external_body item | Classification |
|---|---|
| `ExBTreeMap` | Genuine tooling/type-spec limitation |
| `ExCacheGuard` | Genuine `&mut` struct-field limitation |
| `CacheGuard::deref` | Consequence of opaque `ExCacheGuard` |
| `btreemap_remove` | Wrapper trust boundary; no concrete bug evidence |
| `Cache::get` | Limitation; may hide overflow corner-case assumption, not proven bug |
| `Cache::put` | Limitation; may hide overflow corner-case assumption, not proven bug |
| `find_lru_victim` | Limitation (iterator/min spec gap) |
| `axiom_cache_lru_of_remove` | Limitation from uninterpreted abstraction |

No direct masked functional defect was proven. Overflow remains a documented trust assumption (already in `bugs.md`).

## 5. Spec Quality

- `get` and `put` specs are high-level and useful (`spec_get/spec_put`, invariant preservation).
- `btreemap_remove` spec is reasonably strong (state relation + result relation).
- `find_lru_victim` spec is strong enough for caller (`Some` iff non-empty, returns LRU head).
- Main weakness: local BTreeMap assumptions are stronger than upstream due dropped cmp-obedience guards; this is a spec-faithfulness/soundness concern.

## 6. Challengeable Items (items that SHOULD have been eliminated)

1. **Guard dropping in BTreeMap assumptions should have been eliminated**: restore upstream `key_obeys_cmp_spec` / `obeys_cmp_spec` guards for `len` bridge axiom and `insert` spec.
2. **`Cache::new` AST deviation lacks required rewrite documentation comment** (process/compliance issue).

## 7. Issues Found

1. **High**: `lib.vstd_btree.rs` has stronger-than-upstream assumptions (`len` axiom, `insert`) by dropping cmp-spec guards.  
2. **Medium**: Evidence gap for claim that direct `assume_specification` on `alloc::BTreeMap::remove::<Q>` is impossible (wrapper may still be justified, but reproducer is not included).  
3. **Low**: `Cache::new` mismatch lacks explicit `VERUS REWRITE` comment.

## Result: FAIL

Rationale: trust minimization is incomplete. While most external bodies appear justified, at least one trust boundary set (`len`/`insert` assumptions) is unnecessarily strengthened versus upstream and should be tightened. Under strict audit criteria, this is a failing integrity finding.
