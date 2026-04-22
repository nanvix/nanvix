# Final Review — Verus Verification for `cache` crate

## Scope reviewed

- Source: `src/libs/cache/src/lib.rs`
- Spec: `src/libs/cache/src/lib.spec.rs`
- Proof: `src/libs/cache/src/lib.proof.rs`
- Analysis docs: caller/view/property/trust/bugs/integrity/polish logs under `verus-ai-logs/libs-cache/`

---

## 1) Spec Quality

### Overall assessment
- ✅ **Caller-oriented abstraction**: `CacheView<K, V>` uses `Map`, `Seq`, `nat` (`lib.spec.rs:53-57`), with clear invariant (`lib.spec.rs:66-75`).
- ✅ **Declarative transitions**: `spec_new/get/put/remove/clear` are implementation-independent (`lib.spec.rs:92-164`).
- ✅ **No tautological ensures in public API contracts**: `get/put/remove/clear/new` contracts constrain state and results (`lib.rs:142-145,169-185,209-215,255-261,272-278`).
- ✅ **Frame behavior is explicit enough via transition equality** (`self@ == old(self)@.spec_*`).

### Requested branch checks
- ✅ `get` hit vs miss modeled correctly in `spec_get` (`lib.spec.rs:102-110`) and reflected in method ensures (`lib.rs:173-184`).
- ✅ `put` handles all required branches:
  - zero capacity no-op (`lib.spec.rs:115-118`)
  - overwrite existing (`lib.spec.rs:118-124`)
  - below-capacity insert (`lib.spec.rs:133-139`)
  - at-capacity evict-then-insert (`lib.spec.rs:125-132`)
- ✅ `remove` present/absent key behavior modeled (`lib.spec.rs:144-154`).
- ✅ `clear` preserves capacity (`lib.spec.rs:158-163`).

### Notes
- ⚠️ `View` is uninterpreted for `Cache`/`CacheGuard` (`lib.spec.rs:171-189`), so correctness relies on external_body contracts rather than body proofs.

---

## 2) Caller Coverage

Caller expectations from `caller_analysis.md` were checked against implemented specs.

| Expectation | Status | Evidence |
|---|---|---|
| `Cache::new`: empty, capacity set, usable | ✅ | `lib.rs:142-145`, `lib.spec.rs:92-98` |
| `Cache::get`: hit returns Some/value, refreshes LRU | ✅ | `lib.rs:173-179`, `lib.spec.rs:102-107` |
| `Cache::get`: miss returns None, state unchanged | ✅ | `lib.rs:180-184`, `lib.spec.rs:108-110` |
| `Cache::put`: insert/overwrite/evict/zero-cap no-op | ✅ | `lib.rs:213-215`, `lib.spec.rs:114-141` |
| `Cache::remove`: remove or no-op | ✅ | `lib.rs:259-260`, `lib.spec.rs:144-154` |
| `Cache::clear`: clear entries, preserve capacity | ✅ | `lib.rs:276-277`, `lib.spec.rs:158-163` |
| `CacheGuard::deref`: yields `&V` view value | ✅ | `lib.rs:92-97` |
| `CacheGuard::deref_mut`: yields `&mut V` | ❌ (unverified) | unannotated due Verus limitation (`lib.rs:100-103`, `trust.md:113-124`) |

**Coverage count:** **7/8 caller expectations covered**, with only `deref_mut` excluded from verification.

---

## 3) Proof Completeness

- ✅ **`admit()` count = 0** across `lib.rs`, `lib.spec.rs`, `lib.proof.rs` (direct search found no matches).
- ✅ All 5 invariant-preservation lemmas are present and proven in `lib.proof.rs`:
  - `lemma_spec_new_inv` (`lib.proof.rs:161-171`)
  - `lemma_spec_get_inv` (`lib.proof.rs:174-204`)
  - `lemma_spec_put_inv` (`lib.proof.rs:207-277`)
  - `lemma_spec_remove_inv` (`lib.proof.rs:280-303`)
  - `lemma_spec_clear_inv` (`lib.proof.rs:306-318`)
- ✅ Matches trust/integrity/polish claims (`trust.md:137-149`, `review_r1.md:110`, `checklist.md:48`).

---

## 4) Trust Minimization

### external_body functions (7) — challenged
All 7 remain **justified** under current constraints; no clear eliminations found.

- `Cache::new` (`lib.rs:141`) — **VERUS_LIMITATION / STDLIB_WRAPPER(no_std btree gap)**
- `Cache::get` (`lib.rs:168`) — **VERUS_LIMITATION** (`get_mut` + `&mut` guard)
- `Cache::put` (`lib.rs:208`) — **VERUS_LIMITATION**
- `Cache::remove` (`lib.rs:254`) — **STDLIB_WRAPPER(no_std btree gap)**
- `Cache::clear` (`lib.rs:271`) — **STDLIB_WRAPPER(no_std btree gap)**
- `Cache::evict` (`lib.rs:289`) — **VERUS_LIMITATION + stdlib gap**
- `CacheGuard::deref` (`lib.rs:91`) — **VERUS_LIMITATION** (guard is opaque external type)

### external_type_specification items
- `ExBTreeMap` (`lib.spec.rs:21-24`) — ✅ genuine (`EXTERNAL_TYPE`, with `external_body` due private fields)
- `ExGlobal` (`lib.spec.rs:26-27`) — ✅ genuine (`EXTERNAL_TYPE`)
- `ExCacheEntry` (`lib.spec.rs:31-32`) — ✅ genuine (`EXTERNAL_TYPE`)
- `ExCacheGuard` (`lib.spec.rs:37-39`) — ✅ genuine (`VERUS_LIMITATION`, with `external_body`)

**Challengeable survivors:** ✅ **None identified** (consistent with `review_r1.md:47-54`).

---

## 5) AST Consistency

- ✅ Use precomputed result: **18/18 functions MATCH**, **3/3 structs MATCH**, consistent YES.
- ✅ No `VERUS REWRITE` comments in `lib.rs` (search found none).

---

## 6) Verification

- ✅ Use precomputed result: `make verify-cache` reports **0 errors** (Verus exit 0).
- ✅ Wrapper exit behavior (cheating gate) is explained and expected.

---

## 7) Guardrails Compliance

Precomputed cheating counts:

- `admit = 0` ✅
- `assume = 0` ✅
- `external_body = 9` ⚠️
  - 2 type-level (`lib.spec.rs:22,38`) — acceptable with justification
  - 7 function-level (`lib.rs:91,141,168,208,254,271,289`) — require explicit human sign-off by policy
- `trusted = 0` ✅
- `no_decreases = 0` ✅
- `cfg_gate = 0` ✅

**Assessment:** Technically clean (no admits/assumes/trusted), but governance requires acceptance of the 7 function-level `external_body` items.

---

## 8) Bug Reconciliation

### BUG-1 (counter overflow)
- Status in log: **UNCONFIRMED / LOW** (`bugs.md:5-15`).
- Still valid as a theoretical defect: `self.counter += 1` in `get` and `put` (`lib.rs:188,224,235`) can wrap after `2^64` increments.
- Classification: **Documented trust assumption**, not practical blocker.

### Additional bugs
- ✅ No new concrete functional bug found beyond BUG-1.
- ⚠️ External-body boundary can mask implementation defects in principle; current evidence does not reveal additional masked defects.

---

## Final Verdict

## **CONDITIONAL PASS**

- **Technical verification status:** PASS (spec/proof/integrity/AST/verification all consistent; no admits/assumes/trusted).
- **Policy status:** CONDITIONALLY BLOCKED pending formal human acceptance of the **7 function-level external_body trust items**.

If policy acceptance is granted for those 7 justified limitations, this verification effort is ready to close.
