# Review: cache proving (Claude Opus 4.6)

## 1. Spec Preservation

**PASS — zero diff.**

`git diff 1cd84e654 -- src/libs/cache/src/lib.rs src/libs/cache/src/lib.spec.rs` produces
empty output. Both files are byte-identical to the specification-phase baseline. No specs
were weakened, removed, trivialized, or otherwise modified. The proving phase touched only
`lib.proof.rs`, as intended.

## 2. Cheating Audit

### Forbidden patterns

| Pattern | Count | Verdict |
|---|---|---|
| `admit()` | 0 | ✅ All 5 stubs replaced with real proofs |
| `assume(` | 0 | ✅ Clean |
| `trusted` | 0 | ✅ Clean |
| `verifier::external` (bare) | 0 | ✅ Clean |
| `cfg(not(verus_keep_ghost))` | 0 | ✅ Clean |
| `// VERUS REWRITE` | 0 | ✅ No exec rewrites |

### external_body inventory

**lib.rs — 7 occurrences** (all on exec functions):

| # | Function | Line | Classification | Justified? |
|---|---|---|---|---|
| 1 | `CacheGuard::deref` | 91 | `VERUS_LIMITATION` | ✅ CacheGuard is external_body (contains `&mut V`); body accesses opaque field |
| 2 | `Cache::new` | 141 | `VERUS_LIMITATION` | ✅ Calls `BTreeMap::new()` — no vstd spec |
| 3 | `Cache::get` | 168 | `VERUS_LIMITATION` | ✅ Calls `BTreeMap::get_mut()` — no vstd spec; also constructs CacheGuard |
| 4 | `Cache::put` | 208 | `VERUS_LIMITATION` | ✅ Calls `BTreeMap::get_mut/len/insert` + `self.evict()` — no vstd specs |
| 5 | `Cache::remove` | 254 | `VERUS_LIMITATION` | ✅ Calls `BTreeMap::remove()` — no vstd spec |
| 6 | `Cache::clear` | 271 | `VERUS_LIMITATION` | ✅ Calls `BTreeMap::clear()` — no vstd spec |
| 7 | `Cache::evict` | 289 | `VERUS_LIMITATION` | ✅ Calls `BTreeMap::iter/min_by_key/remove` — no vstd specs |

**lib.spec.rs — 2 occurrences** (both on external type specifications):

| # | Type | Line | Classification | Justified? |
|---|---|---|---|---|
| 1 | `ExBTreeMap` | 21 | `EXTERNAL_TYPE` | ✅ BTreeMap has private fields; `external_body` hides them |
| 2 | `ExCacheGuard` | 37 | `VERUS_LIMITATION` | ✅ CacheGuard has `&'a mut V` field; Verus cannot handle `&mut` in structs |

**Total: 9 external_body** (7 functions + 2 types). All match `trust.md` exactly.

**Escalation ladder assessment:** The root cause is clear — `BTreeMap` has zero vstd
coverage. No individual `external_body` can be eliminated without either (a) vstd adding
BTreeMap specs, or (b) replacing BTreeMap with a data structure that has vstd specs
(e.g., a custom Vec-based map). Option (b) would be a major rewrite of the exec code and
is not warranted for the proving phase. The trust boundary is legitimate and well-documented.

### Unverifiable function

`CacheGuard::deref_mut` (line 101) is excluded from verification entirely — no
`#[verus_verify]` annotation. Documented in `trust.md` as a Verus limitation (`&mut`
return type). This is appropriate and does not constitute cheating.

## 3. Proof Quality

### Helper lemmas (6 total)

All helpers are well-scoped, necessary, and follow standard proof patterns for
sequence/set reasoning in Verus.

**`lemma_push_preserves_no_dup`** (line 16–35):
Proves `push(elem)` on a no-dup sequence preserves no_duplicates when `!s.contains(elem)`.
Uses forall-by quantifier; the empty body relies on the solver connecting the two
preconditions to the conclusion. Clean and correct.

**`lemma_filter_preserves_no_dup`** (line 38–63):
Inductive proof over sequence length. Decomposes into `drop_last + last`, recurses on
rest, and uses `lemma_filter_contains_rev` to show the last element isn't in the filtered
rest. Well-structured structural induction with explicit `decreases` clause.

**`lemma_filter_neq_to_set`** (line 66–90):
Set-extensionality proof: filters `!=key` and shows the result's `to_set()` equals
`to_set().remove(key)`. Both directions are proven explicitly via `forall` blocks using
`lemma_filter_contains_rev`, `lemma_filter_pred`, and `lemma_filter_contains`. This is
correct and necessary — the solver cannot derive this automatically.

**`lemma_filter_neq_len`** (line 93–112):
For a no-dup sequence containing `key`, filtering out `key` reduces length by 1. Chains
together `lemma_filter_preserves_no_dup`, `lemma_filter_neq_to_set`, and
`unique_seq_to_set` (which links no-dup sequence length to set cardinality). Sound
reasoning — the key insight is that for no-dup sequences, `|s| == |s.to_set()|`.

**`lemma_subrange_no_dup`** (line 114–131):
Subrange of a no-dup sequence is no-dup. Direct forall-by proof: `sub[i] = s[start+i]`
and `start+i ≠ start+j` when `i ≠ j`. Minimal and correct.

**`lemma_drop_first_to_set`** (line 134–163):
Specialized case: `subrange(1, len).to_set() == to_set().remove(s[0])`. Both directions
proven with explicit witnesses and `lemma_seq_subrange_elements`. Used in the eviction
case of `spec_put`. Necessary because Verus cannot automatically connect subrange elements
to the original sequence's set.

### Main invariant lemmas (5 total)

Each lemma proves that the CacheView invariant (4 conjuncts: size ≤ capacity,
no_duplicates, to_set == dom, len match) is preserved by the corresponding spec
transition.

**`lemma_spec_new_inv`** (line 170–180):
Trivial: empty map and empty sequence satisfy all invariant conjuncts. Uses set
extensionality to equate `Map::empty().dom()` and `Seq::empty().to_set()` with
`Set::empty()`. ✅ Complete.

**`lemma_spec_get_inv`** (line 183–218):
Hit case: proves `move_to_mru(key)` preserves all four invariant conjuncts.
1. No-dup: filter preserves no-dup, key not in filtered (proof by contradiction
   using `lemma_filter_pred`), then push preserves no-dup.
2. to_set: filter removes key, push adds it back; set algebra shows equality.
3. len: `filter_neq_len` + push length.
Miss case: identity, inherits invariant trivially.
✅ Complete, well-structured with clear numbered steps.

**`lemma_spec_put_inv`** (line 221–315):
Most complex lemma — three branches:
1. **Zero-capacity**: identity, trivial.
2. **Overwrite existing key**: mirrors `spec_get` — same `move_to_mru` pattern.
   Additionally shows `contents.insert(key, value).dom() =~= contents.dom()` since
   key is already present. ✅ Correct.
3. **At capacity, new key (eviction)**: The hardest case. Proves:
   - victim is in contents (via to_set → dom);
   - key ≠ victim (since key is new);
   - subrange(1, len) has no-dup;
   - key not in subrange (proof by contradiction via `lemma_seq_subrange_elements`);
   - push preserves no-dup;
   - to_set algebra: `drop_first_to_set` + push = remove(victim).insert(key);
   - len via `unique_seq_to_set`.
   ✅ Sound and thorough. The `key ≠ victim` step (line 270) deserves attention — it
   relies on `victim` being in `contents.dom()` and `key` not being in `contents.dom()`,
   so the solver can derive `key ≠ victim`. This is correct.
4. **Below capacity, new key**: push(key) with key not already present (shown by
   contradiction via to_set). ✅ Clean.

**`lemma_spec_remove_inv`** (line 318–345):
Present case: filter removes key from lru_order; proves no-dup, to_set, and len.
Uses `filter_preserves_no_dup`, `filter_neq_to_set`, `filter_neq_len`, and
`unique_seq_to_set`. Absent case: identity. ✅ Complete.

**`lemma_spec_clear_inv`** (line 348–360):
Same structure as `spec_new` — empty map and empty sequence. ✅ Trivial and correct.

### Overall proof assessment

The proofs are **genuine mathematical reasoning**, not brute force. The helper lemma
decomposition is clean: each helper proves one reusable property about sequences/sets,
and the main lemmas compose them. The proof strategy follows a consistent pattern:
for each invariant conjunct, invoke the relevant helper and use Verus's extensionality
(`=~=`) to close the gap. There is no unnecessary complexity — every helper is called
at least once, and the hardest case (eviction in `spec_put`) is appropriately the most
detailed.

No missing cases were identified. All branches in the spec transitions are covered.

## 4. Rewrite Audit

**PASS — zero rewrites.**

No `// VERUS REWRITE` comments found in `lib.rs`. The exec code is unchanged from the
original. This is expected since the proving phase only modified `lib.proof.rs`.

## Summary

**No blockers.** The proving phase cleanly replaces all 5 `admit()` stubs with genuine
proofs while preserving the spec baseline exactly. Key findings:

- **Spec preservation**: Perfect — zero changes to `lib.rs` and `lib.spec.rs`.
- **Cheating**: Zero forbidden patterns. All 9 `external_body` items are legitimate
  and documented in `trust.md`. The root cause (BTreeMap lacks vstd specs) is structural
  and cannot be resolved without upstream vstd work.
- **Proof quality**: High. The 6 helper lemmas are well-factored, each main lemma covers
  all branches of its spec transition, and the reasoning is sound mathematical induction
  and set algebra — not brute-force assertion bombing.
- **Exec integrity**: No rewrites, no `cfg` guards on exec code.

The only residual trust gap is the `external_body` boundary around all Cache methods
(inevitable given BTreeMap's status) and the unmodeled `deref_mut` (Verus `&mut`
limitation). Both are well-documented and cannot be addressed in the current Verus
version.
