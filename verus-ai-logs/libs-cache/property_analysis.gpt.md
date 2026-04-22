# Property Analysis (GPT-5.3-Codex Raw Output): `cache` (Bounded LRU Cache)

> Module: `src/libs/cache/src/lib.rs`
> Model: gpt-5.3-codex

---

## Modeling Anchor

Let `Vw(self)` be the abstract `CacheView<K,V>` of concrete state:
- `contents[k] == self.entries[k].value`
- `capacity == self.capacity`
- `lru_order` is keys sorted by increasing `last_used` (LRU → MRU)

Assume no `counter` overflow unless explicitly modeled modulo `u64`.

---

## TYPE-N: Type/Representation Invariants

| ID | Property | Applies | CacheView expression sketch |
|---|---|---|---|
| TYPE-1 | Representation relation: concrete state maps to `CacheView` via `Vw(self)` definition above. | all | `view == Vw(self)` |
| TYPE-2 | View invariant always holds. | all public ops, `evict` | `Vw(self).inv()` |
| TYPE-3 | Capacity bound in concrete and abstract agree. | all | `self.entries.len() == Vw(self).contents.dom().len() <= self.capacity` |
| TYPE-4 | Domain equality: concrete keys equal abstract domain. | all | `Vw(self).contents.dom() == set(self.entries.keys())` |
| TYPE-5 | LRU sequence is a permutation of domain, no duplicates. | all | `Vw(self).lru_order.no_duplicates() && to_set()==dom` |
| TYPE-6 | Ordering correspondence: earlier in `lru_order` means `last_used` is ≤ later key's `last_used`. | all | `i<j ==> last_used(lru[i]) <= last_used(lru[j])` |
| TYPE-7 | Timestamp bounded by counter. | all except overflow caveat | `forall k in dom: last_used(k) <= self.counter` |
| TYPE-8 | Timestamp uniqueness (strict recency order) if no overflow. | `get`,`put`,`clear`,`new` | `k1!=k2 ==> last_used(k1)!=last_used(k2)` |
| TYPE-9 | Empty-state coherence. | `new`,`clear` | `contents=empty && lru_order=empty` |

---

## FN-N: Function-Level Contracts

### `new(capacity)`
- **FN-1 (requires):** none (`capacity: usize`).
- **FN-2 (ensures):** `Vw(ret) == CacheView::spec_new(capacity as nat)`.
- **FN-3 (frame):** initializes empty map, `counter==0`, preserves given capacity.

### `get(&mut self, key)`
- **FN-4 (requires):** no overflow on hit path (`old.counter < u64::MAX`) or explicit modulo spec.
- **FN-5 (hit ensures):** if `old.contents` contains `key`, return `Some`, `Vw(new)==old.spec_get(key)`.
- **FN-6 (hit frame):** `contents` unchanged; only recency order changes (key moved to MRU), `capacity` unchanged.
- **FN-7 (hit counter):** `new.counter == old.counter + 1`.
- **FN-8 (miss ensures):** if absent, return `None` and `Vw(new)==old`.
- **FN-9 (miss frame/counter):** no state change, `counter` unchanged.
- **FN-10 (guard consistency):** returned guard deref equals value at `key` at return time.

### `put(&mut self, key, value)`
- **FN-11 (requires):** if `capacity>0`, increment path requires no overflow (`old.counter < u64::MAX`) unless modulo modeled.
- **FN-12 (capacity-0 ensures):** `capacity==0 ==> Vw(new)==old` and `counter` unchanged.
- **FN-13 (overwrite ensures):** key already present ⇒ `Vw(new)==old.spec_put(key,value)`; size unchanged; no eviction.
- **FN-14 (overwrite counter/frame):** counter +1; `capacity` unchanged; non-key entries/relative order preserved except key→MRU.
- **FN-15 (insert-not-full ensures):** key absent and size<capacity ⇒ key inserted, size +1, key is MRU.
- **FN-16 (insert-full ensures):** key absent and size>=capacity and capacity>0 ⇒ exactly LRU victim removed, key inserted, size unchanged.
- **FN-17 (insert-full victim correctness):** removed key == `old.lru_order[0]`.
- **FN-18 (all nonzero-capacity put counter):** exactly one counter increment.

### `remove(&mut self, key)`
- **FN-19 (requires):** none.
- **FN-20 (ensures):** `Vw(new)==old.spec_remove(key)`.
- **FN-21 (idempotence):** absent key ⇒ no-op.
- **FN-22 (frame/counter):** `capacity` and `counter` unchanged.

### `clear(&mut self)`
- **FN-23 (requires):** none.
- **FN-24 (ensures):** `Vw(new)==old.spec_clear()`.
- **FN-25 (counter reset):** `new.counter==0`.
- **FN-26 (frame):** `capacity` unchanged.

### `evict(&mut self)` (internal)
- **FN-27 (requires):** none (must handle empty safely).
- **FN-28 (non-empty ensures):** removes exactly one key with minimal `last_used`.
- **FN-29 (empty ensures):** empty entries ⇒ no-op.
- **FN-30 (frame):** `counter` and `capacity` unchanged.

### `deref(&self)` on `CacheGuard`
- **FN-31 (requires):** guard valid.
- **FN-32 (ensures):** returns shared ref to guarded value; no mutation/frame preserved.

### `deref_mut(&mut self)` on `CacheGuard`
- **FN-33 (requires):** unique mutable guard borrow valid.
- **FN-34 (ensures):** returns mutable ref to same guarded value location.

---

## MOD-N: Module-Level Safety Properties

| ID | Property | Applies |
|---|---|---|
| MOD-1 | Capacity safety: `|contents| <= capacity` always. | all ops |
| MOD-2 | Key uniqueness/map semantics preserved. | all ops |
| MOD-3 | Lookup consistency: if key present and not removed/evicted, `get` returns that value. | get/put/remove/evict/clear |
| MOD-4 | LRU consistency: `lru_order` matches recency induced by get/put(existing)/put(new). | get/put/evict |
| MOD-5 | Eviction soundness: only LRU key may be evicted. | put(full), evict |
| MOD-6 | Overwrite does not change cardinality and does not evict. | put(existing) |
| MOD-7 | Remove idempotence and non-interference on absent key. | remove |
| MOD-8 | Clear re-establishes fresh empty state (except capacity retained). | clear |
| MOD-9 | Counter monotone nondecreasing between clears; reset only by clear. | get/put/clear |

---

## LIVE-N: Liveness Properties

| ID | Property | Applies |
|---|---|---|
| LIVE-1 | All functions terminate (no unbounded loops/recursion). | all |
| LIVE-2 | `evict` iterator/min search terminates on finite map. | evict |
| LIVE-3 | If `capacity>0`, `put(k,v)` guarantees `k` present afterward. | put |
| LIVE-4 | After `remove(k)`, capacity slot is available for future insert. | remove/put |
| LIVE-5 | After `clear`, cache is immediately reusable (subsequent put behaves as from fresh). | clear/put |

---

## GLOBAL-N: Cross-Module/System Properties

| ID | Property | System connection |
|---|---|---|
| GLOBAL-1 | Bounded-memory contract: cache never exceeds configured capacity. | required for subsystem memory budgeting |
| GLOBAL-2 | Deterministic replacement policy (LRU) under non-overflow assumption. | required by callers relying on eviction predictability |
| GLOBAL-3 | Coherent key-value service abstraction (`put/get/remove/clear`). | required by higher-level memoization/indexing modules |
| GLOBAL-4 | Clear/reset semantics enable recovery/reinitialization workflows. | required for restart/error-recovery paths |

---

## Suspected Bugs / Edge Cases

1. **Counter overflow (`u64`)**: `counter += 1` can overflow (panic in checked configs, wrap in wrapping semantics). This can break strict recency ordering and wrong eviction choice.
2. **`evict()` on empty map**: concrete code is safe (no-op). Must be explicitly specified as safe no-op.
3. **Capacity 0**: `put` is no-op (correct); ensure proofs guarantee no insertion possible.
4. **Capacity 1**: repeated inserts of different keys should always evict sole prior key; verify this path explicitly.
5. **Tie behavior in `min_by_key`**: if overflow permits equal `last_used`, victim may become non-deterministic among ties; conflicts with strict LRU expectation.
