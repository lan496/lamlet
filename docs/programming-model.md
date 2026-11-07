# Lamlet's programming model and language precedents

Status: design direction, reviewed against primary sources on 2026-11-07.
The comparisons below motivate Lamlet decisions. They do not add syntax,
dependencies, or guarantees to the current compiler.

Lamlet builds a structured programming model over LAMMPS. Compatibility protects
existing command streams. Stronger boundaries make inputs, effects, resources,
and execution conditions explicit where new Lamlet constructs require them.
The language can preserve legacy programs without treating every accepted
command as a verified operation inside a module or stage.

## Layers and guarantees

These layers describe responsibilities, not separate file extensions or new CLI
modes. Some existing compiler modules already implement parts of this design.

| Layer | Responsibility | Current boundary |
| --- | --- | --- |
| Legacy LAMMPS | Preserve command bytes and order; check only definite errors | Implemented, with partial schemas and Unknown barriers |
| Typed Lamlet | Quantities, resources, state transitions, ownership, and physical constraints | Quantities, paths, local ownership, partial state analysis, checked module calls, and one physical rule exist |
| Stage and artifact | Explicit inputs and outputs, independent execution, environment and restoration contracts | Individual stages compile and run with explicit bindings and a fresh process; artifact identity, provenance, and restoration remain planned |
| LAMMPS backend | Emit ordinary input files and record their execution contract | Script generation, source maps, project cwd, and explicit engine execution exist |

Legacy preservation and strict module guarantees apply at different boundaries.
An opaque command remains acceptable legacy input. A checked module may expose
only facts supported by its analysis. A stage that needs an unknown prerequisite
must not be labeled independently executable. The extension `.lamlet` alone does
not imply these guarantees.

## Compile-time evaluation: Starlark

Starlark's [Language Design](https://github.com/bazelbuild/starlark/blob/master/design.md)
explains immutable module exports, static name resolution, top-level single
assignment, and deterministic dictionary iteration. Its
[specification](https://github.com/bazelbuild/starlark/blob/master/spec.md)
excludes recursion and unbounded loops. Starlark is dynamically typed and
embedded in a host application, which can supply additional operations. These
properties do not make every embedding free of external effects.

Lamlet adopts the restriction of compile-time computation to bounded expression
evaluation and finite expansion. Current immutable bindings, finite `@for`
lists, acyclic module calls, and expansion limits support that direction.
Module calls expand command bodies and do not return expression values. Future
value-returning functions need the same bounds and predictable collection
iteration. Finite evaluation still needs work and
size limits because nested finite expansions can exhaust resources.

Host operations need a separate contract. Typed paths currently consult the
filesystem, and explicit target inspection launches an executable. Compilation
is therefore not independent of external state. Future artifact and module
resolution should record the observed dependencies before evaluating a plan.
For a fixed source closure, resolved inputs, target snapshot, compiler version,
and options, plan generation should be deterministic. User expressions should
not read clocks, environment variables, networks, or launch commands implicitly.
Runtime LAMMPS commands keep their existing semantics and must be analyzed as
runtime effects.

This borrows Starlark's restrictions without embedding its interpreter, adopting
its dynamic type system, or replacing Lamlet's existing syntax. General Python
or Rust evaluation and arbitrary compile-time I/O remain outside the design.

## Task and artifact identity: Nix

Nix [derivations](https://nix.dev/manual/nix/2.34/language/derivations) describe
builders, system requirements, arguments, and named outputs. Its
[language tutorial](https://nix.dev/tutorials/nix-language) explains explicit
input paths and using one derivation's output as another's input.
The [reproducible-builds documentation](https://reproducible.nixos.org/)
also distinguishes dependency isolation from actual bit-for-bit reproducibility.

Lamlet's stage design adopts explicit artifact dependencies and isolated
invocations. An artifact reference needs producer and output identity as well
as a path and kind. Turning that reference into a backend filename must preserve
the dependency. A filename alone cannot establish which calculation produced it.

The proposed identities have different meanings:

| Identity | Information represented | Consequence |
| --- | --- | --- |
| Task key | Source/module closure, input content digests, parameters and seeds, compiler/backend version, and declared execution environment | Identifies a calculation description |
| Run ID | A particular attempt, including its actual execution metadata | Distinguishes repeated runs of that description |
| Artifact digest | Bytes produced by a completed run | Identifies the observed output |

A task key cannot imply an artifact digest. MPI layout, accelerator behavior,
randomness, and undeclared environmental differences can change results.
Future cache reuse needs an explicit policy and complete dependency information;
the same task key must not silently authorize overwriting or equating distinct
run results. Recording all inputs enables a better rerun contract without
proving numerical determinism or sandbox isolation.

Current path resolution and dependency inspection are partial. Current target
fingerprints use executable size and modification time, not content hashes.
They cannot support the complete identity above. Content hashing, an artifact
store, cache reuse, and execution sandboxing remain unimplemented. Lamlet does
not need to implement a Nix store or a workflow scheduler to expose a task graph.

## Constraint composition: CUE

CUE's [value lattice and unification](https://cuelang.org/docs/concept/the-logic-of-cue/)
combine values, types, and constraints independently of declaration order.
Contradictory constraints produce an error rather than a later value silently
overriding an earlier one.

Lamlet can use this idea for compatible facts at one program point. A proposed
constraint record carries its subject, predicate, applicability conditions,
source span, and evidence. Combining constraints for that subject can either
refine its description or expose a contradiction. Missing evidence yields
Unknown; it is distinct from both a proven contradiction and proven absence.

Command execution remains ordered. `fix`, `unfix`, and `clear` change which facts
hold, so they cannot be merged as unordered declarations. State transfer first
produces a snapshot for a particular program point and resource lifetime.
Constraint composition then checks that snapshot. An ID reused after release
refers to a new lifetime and must not inherit facts from its earlier use.

The existing state, ownership, and physical-rule analyses are partial and
separate. A general constraint IR, CUE integration, and SMT solving are not
implemented. The next step is explicit predicates and evidence for supported
commands, not a dependency on an external solver.

## Physical meaning: Modelica

The [Modelica 3.7 specification](https://specification.modelica.org/maint/3.7/MLS.html)
describes reusable equation-based physical models. Its
[connection restrictions](https://specification.modelica.org/maint/3.7/connectors-and-connections.html#S9.SS3)
require compatible quantity attributes and distinguish quantities that share
units, such as work and torque. This supports keeping semantic compatibility
separate from dimensional compatibility.

For Lamlet, a proposed TemperatureObservable record includes group identity,
velocity-bias policy, DOF policy, definition lifetime, and evidence. Equal kelvin
units do not establish equal temperature definitions. Unknown group overlap or
modified DOF rules must remain visible rather than guessed. Thermostat and
displayed temperature may intentionally differ, so the physical rule's
applicability and severity remain separate from the record's type.

These records would describe LAMMPS semantics; they are not Modelica types or
an equation solver. The current temperature rule recognizes only a narrow set
of definitions. General observables and automatic physical-model synthesis are
unimplemented, and no implicit DOF correction is proposed.

## Effects and ownership: Koka

The [Koka book](https://koka-lang.github.io/koka/doc/book.html)
tracks function effects alongside argument and result types, with effect
polymorphism and handlers. Lamlet needs the narrower ability to compose command
preconditions and state changes across a module boundary.

A proposed command contract records required facts, invalidated facts, newly
established facts, resource acquisition or release, artifact reads and writes,
and dynamics advancement. For example, `read_data` can establish atom-system
facts, `fix nvt` acquires a resource with integration and thermostat roles, and
`run` advances dynamics. The exact contract must include command options and
target-version assumptions. Unmodeled effects cannot be treated as an empty
effect set.

Effects alone do not prove resource ownership. Lamlet still needs dependency
and lifetime checks to reject double release and dangling references.
Generated cleanup also needs a control-flow guarantee; effect annotations cannot
make an escaping `jump` execute cleanup. Current scope restrictions continue
to apply. General effect rows, effect handlers, and first-class continuations
are not required for the initial module system.

## Migration and the next boundary

Sorbet's [gradual checking](https://sorbet.org/docs/gradual) provides a useful
migration precedent through explicit untyped values and differing strictness.
Lamlet preserves that migration choice while keeping the guarantees precise.
Unknown legacy commands remain accepted; a new checked boundary must expose
unresolved effects and cannot present them as verified independence.
`@unchecked` currently acknowledges operand validation only. It cannot establish
module preconditions, complete dependencies, resumability, or stage isolation.

The initial boundary tests now distinguish a proven postcondition, an invalidated
fact, and an unknown effect; exercise repeated resource IDs; and run stage
instances in separate directories and processes using declared inputs. See
[module tests](../crates/lamlet/tests/modules.rs),
[stage tests](../crates/lamlet/tests/stages.rs), and the
[CLI integration tests](../crates/lamlet-cli/tests/cli.rs). Engine execution tests
use a fake executable and do not validate molecular dynamics. Producer identity,
checkpoint restoration, and full provenance still need their own acceptance
criteria.

The [module and stage contract](modules-and-stages.md) records the remaining
syntax and checkpoint decisions. No external language runtime, solver, cache,
or scheduling dependency is introduced by this design.
