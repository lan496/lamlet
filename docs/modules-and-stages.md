# Modules, stages, and checkpoint contracts

Status: accepted design direction; syntax, invocation, lowering, and execution
are not implemented. The examples in the design discussion are proposals, not
accepted compiler syntax. Existing single-file execution remains supported.

The [programming model and language precedents](programming-model.md) separates
legacy, typed, stage/artifact, and backend responsibilities. It records which
ideas from Starlark, Nix, CUE, Modelica, and Koka inform these contracts without
adding their runtimes or promising their guarantees.

## Independent execution

Every Lamlet stage must be independently executable from explicitly declared
inputs, without relying on hidden state left by previous stages.

The implementation order for the next language extensions is compatibility;
typed inputs, outputs, units, and paths; modules and static effects; stages and
independent execution; local scoped resources; checkpoint/provenance; and
physical semantic rules. Existing scope and initial physical-rule support
remain available while the earlier layers are developed.

Modularity means explicit module inputs, outputs, and dependencies. Reentrancy
means repeated use has no dependency on undeclared caller state or leaked
resources. Resumability adds a validated restoration contract for an interrupted
calculation. None of these implies bitwise deterministic MD trajectories.

## Module and stage boundaries

A module reuses code inside a simulation. Its callable operations have declared
preconditions and state effects, and may change the current LAMMPS state. A
module call is not assumed to be pure or independently runnable. Checking must
compose those effects with the caller's facts rather than infer purity from
function-like syntax.

A stage is an execution task with typed artifact inputs and outputs, calculation
conditions, a target environment, and declared effects. Each invocation needs an
isolated working directory and an independently generated script. It must work
in a fresh LAMMPS process. Input artifacts cannot be modified implicitly, and
output ownership must make collisions detectable. Reusing a stage requires
separate invocation identity from the reusable stage definition.

State requirements should be separate facts, initially including model
definition, atoms present, and integrator configuration. A generic ReadyToRun
marker cannot establish an ensemble or a complete restoration contract. Opaque
legacy fragments remain valid outside these new boundaries. Inside a stage,
undeclared dependencies or unprovable required effects cannot earn an independent
execution guarantee: the boundary must reject them or require an explicit
contract that makes the unresolved effect visible.

`@scope` owns local fix/compute/dump lifetimes within this model. Stage completion
does not imply indiscriminately inserting `unfix`: checkpoint output may need
state saved before local cleanup. Persistent resources require explicit export
and restoration metadata rather than references to a previous process.

## Checkpoint contract

A checkpoint combines the original LAMMPS restart artifact with a Lamlet
sidecar. The proposed sidecar records the model definition and parameter files,
resource restoration declarations, target profile, input hashes, and provenance.
It does not change LAMMPS's restart format. Producing a restart file alone must
not confer a Resumable type.

LAMMPS [restart documentation](https://docs.lammps.org/read_restart.html) states
that commands for fixes, computes, variables, regions, and output need
re-declaration. Some force fields require coefficients to be supplied again.
Fixes with saved internal state require matching IDs and styles during
restoration, before commands discard unused saved state. Restart binaries are
not a portable state format across arbitrary executables and platforms.

The restoration contract must classify resources by supported persistence
behavior, preserve matching identity where needed, and verify the ordering of
restoration operations. For example,
[`compute msd`](https://docs.lammps.org/compute_msd.html) relies on its internal
fix identity to restore reference positions; its `average yes` sample count is
not restored. Such a calculation cannot be labeled exactly continuous from a
generic resource-ID match alone.

Target, MPI layout, numerical settings, and resource behavior constrain what
continuation can promise. A contract may guarantee reconstructible inputs and
supported state restoration without promising an identical trajectory. The
compiler must report the applicable guarantee explicitly.

## Workflow interface and open decisions

Lamlet should export stage definitions, invocation dependencies, input/output
artifacts, target requirements, and unresolved effects as deterministic JSON.
An external workflow engine may schedule processes and handle retry policy.
Caching cannot assume that stage syntax denotes a pure function: seeds,
execution environment, and calculation conditions belong in provenance and
any future cache policy.

The following decisions are required before stage syntax can be implemented:

- Invocation and artifact binding: how a reusable stage definition receives
  concrete files, quantities, and outputs, and how dependencies connect.
- Module exports and effect declarations: the syntax and the supported facts
  that a caller can require or establish.
- State construction and restoration: the exact contracts for `@system`,
  `@resume`, and resource persistence, including unsupported styles.
- Output selection: how a stage with multiple artifacts names its outputs and
  how a checkpoint is exported before managed resources are released.

`@module`, `@fn`, `@stage`, `@system`, `@resume`, `@output`, stage selection,
workflow JSON, and checkpoint manifests are therefore unavailable in this
release. They are not silently treated as verified module boundaries.
