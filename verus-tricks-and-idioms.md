# Verus Tricks and Idioms

## Data-Structure Invariants

- Expose the primary invariant as `pub open spec fn inv(&self) -> bool`. An open invariant acts
  as a transparent abbreviation, so callers and downstream modules can unfold the representation
  facts needed to satisfy method preconditions and postconditions.
- Write invariant predicates as methods of the object whose state they describe. A
  `permissions_well_formed(&self)` predicate should read the instance's tracked permission map and
  ghost base directly instead of accepting them as unrelated parameters.
- Factor predicates over constructor inputs into separately named helpers. A constructor does not
  yet have a `self`, so `permissions_match_storage(permissions, base)` validates the resources
  used to assemble an instance, while `self.permissions_well_formed()` describes an existing
  instance.
- Keep `inv()` small and compositional. It should combine named representation predicates rather
  than duplicate their definitions.
- Split a data-structure invariant into an open `wf()` predicate and a closed `internal_inv()`
  predicate. Put public sanity and semantic validity properties in `wf()`, and put
  representation-dependent facts needed only by the implementation in `internal_inv()`.
- Define the main open invariant as `self.wf() && self.internal_inv()`. Clients can unfold and use
  the public well-formedness facts while carrying the closed internal invariant abstractly through
  verified operations.
- Give each helper predicate one coherent responsibility. For example, keep collection domain
  shape in a well-formedness predicate and keep element-to-storage correspondence in a separate
  matching predicate; do not bundle unrelated representation properties for convenience.
- Package persistent data-structure properties in `inv()` instead of repeating them in every
  method's preconditions and postconditions. Methods should require the invariant, establish any
  operation-specific input conditions, and ensure that the invariant is preserved.
- Account for all legitimate lifecycle states when adding a property to an invariant. For storage
  that may not yet be initialized, express the property as "uninitialized, or initialized with a
  valid value" rather than requiring a value accessor to be meaningful unconditionally.
- Use a closed helper only when its representation should be hidden across module boundaries and
  exported lemmas provide the intended reasoning interface. Open predicates are preferable when
  clients must routinely unfold concrete representation facts.
- State collection domains against the concrete object size. Prefer
  `0 <= i < CAPACITY` over `i < permissions.dom().len()` when the data structure has a fixed
  size. This avoids circular definitions and makes the intended shape clear.
- State both the lower and upper bound whenever an index is used for an array, slice, vector, map,
  or sequence access. Write `0 <= i < len` even when the index type makes the lower bound
  mathematically redundant; the explicit range documents the access obligation and generalizes
  safely when index types change.
- State both finiteness and exact domain shape when cardinality matters. A useful pattern is:
  `dom().finite()`, `dom().len() == N`, and `dom() == Set::new(|i| i < N)`.
- Preserve the relationship between ghost state and executable storage. Permission invariants
  should cover address offsets, pointer provenance, initialization, and the expected domain.

## Trusted Environment Interactions

- Isolate each terminal environment operation in a small `env_interaction_*` wrapper and mark only
  that operation as `#[verus_verify(external_body)]`. This keeps the trusted computing base narrow.
- Use `old(self)` for mutable-receiver pre-state and `final(self)` for post-state in attribute-mode
  contracts.
- Require the owner's invariant before an interaction and ensure it afterward.
- Add explicit frame conditions. For a single-entry write, specify the new selected value and prove
  that every other permission is unchanged.
- Specify protocol or representation validity separately from memory safety. A valid pointer and
  initialized `PointsTo` permission do not imply that the stored value is meaningful to an
  external environment.
- Do not fold publication, lifetime, ordering, cache invalidation, or synchronization effects into
  a memory-write contract. Model those obligations at their own interaction boundaries.

## Raw Memory and Proof Ownership

- Separate runtime identity from proof knowledge:
  - `*mut T` identifies the executable location.
  - `PointsTo<T>` represents tracked ownership and knowledge of its contents.
  - `MemContents<T>` distinguishes initialized from unknown contents.
- Do not use `&mut T` as the proof model for memory that hardware or another environment agent may
  modify. A mutable reference asserts exclusivity that the real system may not provide.
- Store per-entry permissions in a tracked map owned by the data structure. Do not mint permissions
  at each interaction site.
- Thread proof resources from their origin:
  `allocator -> owning structure -> environment interaction`.
- Preserve a ghost base pointer when a permission map alone cannot prove that its entries belong to
  the executable allocation.
- Check pointer provenance as well as numeric addresses. Equal addresses do not necessarily prove
  that pointers originate from the same allocation.
- Use a quantifier trigger on map lookup, such as
  `let permission = #[trigger] permissions[i]`, when the solver must instantiate per-entry
  properties from an indexed access.

## Zero-Runtime-Cost Attribute-Mode Specifications

- Keep executable functions, including `env_interaction_*` wrappers, outside `verus! {}`. These
  functions must remain in the runtime binary and be callable from ordinary Rust.
- Use `verus! {}` only for declarations that live purely in spec or proof mode. Attach contracts
  to executable functions with attributes such as `#[verus_spec(...)]` and
  `#[verus_verify(...)]`, and place local proof steps in `proof! {}` blocks.
- Do not call executable-mode functions from specification expressions. Prefer a specification-
  usable constant when the result is fixed. For computed behavior, define an equivalent spec
  function and prove that the executable function refines or returns the value described by that
  spec function.
- Gate ghost and tracked fields with `#[cfg(verus_keep_ghost_body)]` so ordinary Rust layout and
  APIs remain unchanged.
- Add proof-only inputs with `#[verus_spec(with ...)]`.
- Supply proof-only fields at verified construction sites with `proof_with!`.
- Keep `Tracked` and `Ghost` values out of ordinary executable signatures.
- Let executable owners retain their proof resources instead of repeatedly passing them through
  runtime methods.

## Pitfalls

- A `pub closed spec fn inv` hides its body from other modules. This is unsuitable when clients
  need direct access to representation facts and no abstraction lemmas exist.
- Putting representation-specific details directly in an open `wf()` or `inv()` leaks the
  implementation into client proofs and makes refactoring expensive. Conversely, hiding basic
  validity facts in `internal_inv()` prevents clients from performing ordinary sanity checks.
- A static predicate named like an object invariant obscures ownership. Use a `&self` method for
  steady-state validity and a distinctly named helper for unassembled constructor resources.
- Repeating a persistent representation property in individual method contracts makes the
  specification noisy and allows methods to disagree about the object's valid states. Put the
  property in the invariant and prove invariant preservation instead.
- A helper whose name describes one relationship should not silently enforce unrelated domain,
  lifecycle, or value-validity properties. Poor packaging makes contracts harder to audit and
  reuse.
- Defining a domain in terms of its own length can admit an unnecessarily indirect or ambiguous
  shape. Tie it directly to the data structure's fixed capacity.
- An unconstrained permission introduced at the interaction site makes the proof vacuous and can
  conceal aliasing or lifetime bugs.
- Checking pointer addresses without provenance can incorrectly relate different allocations.
- Updating executable memory without transitioning the corresponding `PointsTo` state leaves the
  proof model stale.
- Defining a runtime interaction wrapper inside `verus! {}` risks erasing the function during
  ordinary compilation and making the runtime call unavailable. A function is not proof-only just
  because it carries a Verus contract.
- Calling an executable function from spec mode crosses Verus's mode boundary and is generally
  rejected. Repeating the executable computation informally in contracts is also fragile; use a
  shared constant or a spec function connected to the implementation by a proof.
- Making a large method an `external_body` trusts unrelated policy and weakens verification.
- Introducing explicit environment-owned ghost state too early mixes a trusted boundary contract
  with a future environment implementation.
