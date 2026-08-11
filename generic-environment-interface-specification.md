# Specifying a System-Environment Interface

## Purpose

A low-level environment specification describes the boundary between a system and an external
agent that the system cannot implement or inspect directly. Examples include hardware, a network,
other threads, a hypervisor, a device, or an operating-system service.

The specification should state:

- what the system assumes the environment will do;
- what the system guarantees before or during an interaction;
- what the environment may change;
- what the system may rely on after the interaction.

The result is a contract suitable for verifying the system without first verifying the
environment.

## Method

### 1. Define the environment model informally

Start with a simple operational description of the environment:

- what state it observes;
- what state it may modify;
- what requests or events cause it to act;
- what results, faults, or asynchronous changes it may produce;
- what state it may cache.

Keep this model general enough to be recognizable to a domain expert. Use the implementation to
discover details, but do not define the environment solely in terms of one codebase.

### 2. Trace the reachable interaction boundary

Starting from the relevant subsystem entry point, follow all reachable code and identify terminal
interactions. An interaction is a read or write of state shared with, interpreted by, or controlled
by the environment.

Prefer the last-level primitive:

- a raw or volatile memory access;
- a register or device operation;
- an atomic operation;
- a system or network call;
- a synchronization primitive.

Do not list every caller when they all converge on one primitive, but search for bypasses such as
standalone pointer dereferences, inline assembly, direct buffer access, or duplicate implementations.

Also inventory structures that own or describe shared state. Ownership, reachability, lifetime,
encoding, and cached state often create obligations even when the interaction is a single write.

### 3. Separate responsibilities

For each interaction, write four short lists:

1. system assumptions about the environment;
2. system guarantees to the environment;
3. environment assumptions about the system;
4. environment guarantees to the system.

Include only cross-boundary responsibilities. Do not misclassify an internal allocator,
initialization, alignment, or lifetime obligation as an environmental assumption merely because it
is necessary for correctness. The system must establish its own internal preconditions before the
boundary contract applies.

### 4. Isolate each interaction

Wrap every terminal interaction in a narrowly scoped function. The wrapper should:

- perform the same single operation;
- have a name identifying it as an environment interaction;
- contain no unrelated policy;
- preserve the original operation as a comment when auditability requires it;
- reside near the original implementation or in an included specification file.

This produces a stable verification boundary and prevents contracts from being scattered across
arbitrary raw accesses.

Prioritize audit simplicity and direct semantic equivalence when choosing the wrapper signature and
body. Prefer moving the original terminal statement unchanged into the wrapper over decomposing it
into derived pointers, offsets, or lower-level operations merely to simplify the proof. Keep
location derivation and consistency obligations in invariants, contracts, and proofs when possible,
so an implementation reviewer can compare the wrapper with the replaced operation without reading
the proof. Introduce a lower-level executable form only when it is required for correctness or
expressibility, and document why it is equivalent.

### 5. Specify only observable system state

When the environment is not yet modeled explicitly, the trusted contract should mention only:

- executable state available to the system;
- ghost or tracked state representing knowledge held by the system;
- input and output values of the interaction.

Do not introduce hypothetical environment-owned state merely to make the contract expressive.
Such state belongs in a later environment model. At the trusted-boundary stage, encode the
environment's promise as a postcondition over system-observable state.

### 6. Thread proof resources from their origin

A contract requiring ownership or knowledge must explain where that proof resource comes from.
Mint it at the operation that establishes ownership, then store and transfer it with the executable
owner:

```text
resource creator -> owning object -> interaction primitive
```

Avoid creating permissions locally with an unconstrained assumption at every interaction. That
would make the contract vacuous and hide aliasing or lifetime mistakes.

## Memory and Shared-State Abstractions

A raw pointer identifies a location but does not describe what is stored there or who may access
it. These concerns should be represented separately.

Verus `PointsTo<T>` is a useful generic abstraction:

- the raw pointer represents runtime identity;
- `PointsTo<T>` represents tracked permission and knowledge of the pointed-to contents;
- `MemContents<T>` distinguishes initialized and unknown contents.

This pattern applies to memory shared with hardware, devices, foreign code, or other execution
agents. It is also useful in concurrent verification, but `PointsTo` itself is exclusive. If
another agent is explicitly modeled as mutating the location concurrently, the permission must
normally be protected by an invariant, lock protocol, atomic abstraction, or another rely-guarantee
mechanism.

Do not use an ordinary mutable reference as the sole model of externally mutable memory. A mutable
reference asserts exclusive access, which may be stronger than the real system guarantees.

Conversely, do not add a concurrent invariant prematurely. If the current goal is only a trusted
interface contract and not an explicit model of the environment, store the system's permission and
describe the interaction as a trusted transition of that permission.

## Permissions as Environment Knowledge

A permission represents more than memory safety: it also represents what the verifier knows about
the current contents of a location. This distinction matters when an environment may modify shared
memory asynchronously.

An exclusive initialized permission asserts that:

- the permission owner has exclusive write authority;
- the location currently contains the recorded value; and
- no unmodeled agent changes that value while the permission remains valid.

Hiding such a permission behind a private wrapper does not weaken these semantic claims. Privacy
prevents clients from using the permission directly, but it does not make stale internal proof state
sound. If the environment may modify the location, the specification must change the ownership or
knowledge model rather than rely on encapsulation alone.

Two useful designs are:

1. **Conservative trusted-boundary design.** Retain the raw permission for identity and access, but
   forget exact contents between interactions. Store separate ghost knowledge for properties that
   the environment cannot change. Trusted interaction wrappers update this knowledge and return
   observations constrained by a compatibility relation.
2. **Shared-invariant design.** Place the authoritative initialized permission inside an invariant
   shared by the system and environment model. Neither side owns it continuously. Each interaction
   opens the invariant, observes or updates the exact value, and closes the invariant with a state
   satisfying the permitted-transition rules.

The first design does not explicitly implement the environment and is appropriate for a trusted
interface stage. The second supports direct reasoning about asynchronous environment actions, but
introduces an explicit shared-state protocol and is therefore closer to modeling the environment.

### Stable and Environment-Managed State

When the environment may change only part of a value, split knowledge into:

- **stable fields**, which must remain equal to the last system-established value; and
- **environment-managed fields**, which may change according to a stated transition relation.

Define a compatibility predicate between an expected value and an observed value. It should:

- require the observed value to remain valid;
- require stable fields to be equal;
- permit only documented changes to environment-managed fields; and
- encode directionality when changes are monotonic, such as a status bit that may be set but not
  cleared by the environment.

A read interaction should return the actual observed value, not necessarily the system's previous
exact value. A write interaction may establish a new expected value, but the specification must not
continue claiming exact knowledge after the environment is allowed to modify it.

This pattern generalizes beyond hardware registers and paging structures. It applies to device
status words, DMA descriptors, shared-memory protocols, foreign runtimes, hypervisors, and other
interfaces where ownership of a location and knowledge of all its fields are not identical.

### Specify the Token Semantics Before Its Machinery

When no existing proof primitive matches the environment's authority, first define a small abstract
token API in terms of the required semantics:

- identity of the shared location;
- initialized versus uninitialized state;
- the baseline most recently established by the system; and
- a predicate describing values the environment may currently produce.

Do not select an ownership primitive merely because its API is convenient. In particular, a private
exact-value token remains unsound if the environment may change the value. Decide later whether the
abstract token is implemented by conservative knowledge, an invariant, a state machine, or another
mechanism.

Derived contract facts need not be repeated. If an open observation predicate is defined entirely
from initialization, baseline, and compatibility, a write need only establish the new baseline.

### Validate Referenced Objects at Pointer Publication

Writing a value that contains a pointer or handle may expose a second object to the environment. If
the environment may immediately follow it, the write precondition should connect the encoded target
to a proof-only view of the actual object and require:

- the encoded address or identifier matches that object;
- the object has the required shape and capacity;
- every element the environment may inspect is initialized; and
- every such element satisfies the protocol's validity predicate.

Use a conditional proof argument when absent values are not followed. Require exact correspondence
between the encoding and the argument: a present pointer has `Some(target)`, while an absent pointer
has `None`. Accepting `Some(target)` for an absent encoding hides caller mistakes and weakens the
meaning of the witness.

Keep this immediate publication condition separate from continued lifetime. A valid target at the
write does not by itself prove that the target remains allocated for every later environment access.
Also distinguish virtual storage identity from the physical, device, or protocol address encoded in
the published value.

## Zero-Runtime-Cost Specifications

Proof state must not alter production APIs or binaries.

For Verus attribute-mode code:

- gate ghost or tracked structure fields with `#[cfg(verus_keep_ghost_body)]`;
- add proof-only parameters with `#[verus_spec(with ...)]`;
- supply them at verified call sites with `proof_with!`;
- use `proof_decl!` for proof variables that must span proof blocks;
- keep ordinary Rust signatures unchanged.

The owning structure may store a tracked permission map and allow interaction specifications to
access `self.permissions` directly. This is usually clearer than passing the same permission
explicitly through every executable method.

## Contract Design Rules

- State architectural or protocol validity explicitly; memory safety alone does not imply that a
  value is meaningful to the environment.
- Preserve unaffected state in postconditions.
- Distinguish writing shared state from publishing it. Ordering, invalidation, synchronization, or
  acknowledgement may be separate interactions.
- Account for environment-managed fields or asynchronous updates.
- Tie permission domains to the expected concrete object size, not merely to their own length.
- Preserve the link between executable storage and ghost permissions. A permission map is
  insufficient unless its pointers are related to the object's actual backing storage.
- Keep trusted wrappers small. The larger the external body, the larger the trusted computing base.

## Approaches to Avoid

- **Overfitting the environment model to implementation details.** It obscures the real interface
  and makes the specification less reusable.
- **Listing high-level callers instead of terminal accesses.** This duplicates work and can miss
  raw bypasses.
- **Refactoring an interaction solely for proof convenience.** Passing derived addresses or
  splitting an operation may create additional runtime states and audit obligations. Preserve the
  original executable statement and discharge representation relationships on the proof side when
  feasible.
- **Treating internal correctness as an environmental promise.** Allocation, initialization, and
  ownership remain system obligations.
- **Using `&mut` to model memory that an external agent may change.** It assumes exclusivity not
  present in the real system.
- **Keeping an exact exclusive permission while allowing asynchronous environment writes.** The
  permission becomes semantically stale even if it is private and never exposed to clients.
- **Treating a private wrapper as weaker knowledge.** Encapsulation restricts API use but does not
  weaken the semantic claims of the wrapped proof resource.
- **Allowing an optional target witness that does not exactly match presence in the encoding.** This
  admits irrelevant or missing witnesses and obscures which case the caller proved.
- **Conflating immediate target validity with lifetime.** Initialization and address agreement at
  publication do not establish that the referenced object remains alive.
- **Treating encapsulation as a concurrency model.** A custom API can restrict proof clients, but
  soundness still requires conservative knowledge or shared ownership of the authoritative
  permission.
- **Inventing environment ghost state before modeling the environment.** It mixes a trusted
  contract with a future implementation of the environment.
- **Passing tracked values as ordinary Rust arguments.** This changes runtime APIs and may add
  runtime effects.
- **Minting proof permissions at the interaction site.** Permissions should flow from the owner or
  allocator that established them.
- **Using only pointer addresses in a permission invariant.** Preserve provenance and the relation
  to the executable allocation where the verification model requires them.

## Completion Checklist

An environment-interface specification is ready to generalize when:

- all terminal interactions and bypasses are identified;
- each interaction has a small wrapper;
- cross-boundary assumptions and guarantees are separated from internal obligations;
- shared state has an appropriate ownership or knowledge abstraction;
- proof resources flow from creation to use without runtime effects;
- encodings and protocol values have explicit validity predicates;
- affected and unaffected state are specified;
- ordering, caching, and invalidation are represented by the correct interaction;
- ordinary compilation and formal verification both succeed.
