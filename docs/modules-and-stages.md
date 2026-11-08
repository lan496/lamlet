# Modules, stages, and checkpoint contracts

Status: same-source modules, checked calls, typed stage bindings, individual
stage builds, and fresh-process execution are implemented. Checkpoint restoration
and workflow invocation graphs remain design requirements. Existing single-file
execution remains supported.

## Implemented module syntax

```lamlet
@module dynamics {
    @fn sample(steps: Int) requires [AtomsDefined, ModelDefined] ensures [AtomsDefined, ModelDefined] {
        @fix integrate all nve
        run @{steps}
    }
}
# Construct the system and pair model before calling this operation.
@call dynamics.sample(1000)
```

Modules contain function declarations and comments. Definitions are available
throughout the source. Calls use qualified names and positional expressions.
Parameters accept Int, Float, Bool, Time, Length, Temperature, Pressure, Energy,
and the existing resource types. Float accepts Int, and file resource subtyping
follows typed path bindings. Calls cannot capture caller bindings. Pass each
value explicitly. External module imports and value-returning functions are
not implemented.

The compiler rejects duplicate definitions, unresolved calls, recursion, and
call graphs deeper than 64 levels, including unused definitions. Expansion uses
the existing line and work bounds. Every instantiated call checks its arguments,
preconditions, sequential command effects, and postconditions. Uncalled function
bodies do not undergo expression or state-contract checking. A function's
`ensures` list is checked against its resulting state; it cannot assert a fact
into existence. Source maps retain definition locations and call expansion names.
Signatures are therefore checked at instantiation, not universal proofs for all
possible arguments. Symbolic, standalone module-body checking remains unimplemented.

Functions own an implicit resource scope. Managed resources are released before
postconditions are checked. Plain fix/compute/dump commands retain manual
lifetime management and may intentionally affect the caller. Modules therefore
do not automatically claim reentrancy.

State facts are separate from LAMMPS operand schemas:

| Fact | Evidence tracked in the initial subset |
| --- | --- |
| BoxDefined | create_box or read_data |
| AtomsDefined | create_atoms or read_data |
| ModelDefined | Supported literal pair_style followed by pair_coeff * * |
| IntegratorConfigured | Active nve/nvt/npt fix on group all |

AtomsDefined records an atom construction command, not a measured nonzero atom
count. ModelDefined records a model declaration, not proof of force-field
correctness or complete coefficients for every type. Pair styles are currently
lj/cut, eam, and eam/alloy. Clearing state removes facts; changing pair_style
invalidates ModelDefined. Releasing an integrator updates IntegratorConfigured.
Opaque legacy commands weaken facts to Unknown. Neither Unknown nor Absent
satisfies a required fact.

Inside a called function, unsupported effects, runtime substitutions, external
includes, shell operations, runtime control flow, plugins, and run callbacks are
errors even without --strict. The finite command/style subset is defined in
`module::Effects`. This is an intentionally conservative boundary, separate from
legacy acceptance. Its facts establish command ordering and declared setup;
they do not certify numerical or physical correctness.

The [programming model and language precedents](programming-model.md) separates
legacy, typed, stage/artifact, and backend responsibilities. It records which
ideas from Starlark, Nix, CUE, Modelica, and Koka inform these contracts without
adding their runtimes or promising their guarantees.

## Independent execution

Every Lamlet stage must be independently executable from explicitly declared
inputs, without relying on hidden state left by previous stages.

## Implemented stage syntax and execution

```lamlet
@stage relax(structure: InputFile, relaxed: OutputFile) {
    units metal
    atom_style atomic
    read_data @{structure} nocoeff
    pair_style lj/cut 2.5
    pair_coeff * * 1 1
    minimize 1e-8 1e-10 100 1000
    write_data @{relaxed}
}
```

Named OutputFile parameters declare outputs. This initial syntax supports multiple
outputs without an implicit `@output` or a return-type alias. Numeric and quantity
parameters use the same types as module calls. Stage file parameters are limited
to InputFile, PotentialFile, and OutputFile. Patterns, directories, scripts, and
dynamic paths cannot establish this initial independent-execution contract.

From the repository root, the complete [stage example](https://github.com/lan496/lamlet/blob/main/examples/project/stages.lamlet)
can be inspected, checked, and built without launching LAMMPS:

```sh
cargo run -- inspect stages examples/project/stages.lamlet --format json
cargo run -- check examples/project/stages.lamlet --stage relax --arg 'structure=path("structure.data")' --arg 'relaxed=output("relaxed.data")' --format json
cargo run -- build examples/project/stages.lamlet --stage relax --instance relax_a --arg 'structure=path("structure.data")' --arg 'relaxed=output("relaxed.data")' --format json
```

`--arg NAME=EXPR` evaluates a closed expression. Bind every parameter exactly once;
unknown names, missing arguments, duplicate arguments, and type mismatches fail.
`--instance` is an identifier and defaults to the stage name. Each instance uses
`<build.directory>/stages/<instance>/` as its working directory. The default script
is `simulation.in` there. Input paths resolve from the project root under the
configured input root. Output paths resolve inside the instance directory,
independently of the legacy `[paths].output_root`. Output escapes, symlink aliases
of instance directories, source/input collisions, and collisions with generated
scripts or sidecars are rejected. Use separate instance names for separate tasks.
Operations on the same instance must be serialized by the caller; Lamlet is not
a concurrent workflow scheduler.

`lamlet run` with the same bindings builds and starts a fresh process in that
directory. Stage execution requires an explicit `--lammps-bin` or configured
`[lammps].executable`. It does not select an engine through the legacy environment
fallback. The executable is resolved before entering the instance directory,
including relative PATH entries. Saved capability validation applies when enabled
in the project.
Offline check/build does not require a target profile; `target_profile_selected`
and `target_requirements` make missing availability evidence explicit. An
independent command stream does not certify an available target, MPI/GPU setup,
or a hermetic operating-system environment.

Each selected stage starts from empty simulation state. Stage files may contain
only stage definitions, module definitions, and comments at top level. Parameters
are the only incoming bindings. The stage body has an implicit resource scope,
so `@fix`, `@compute`, and `@dump` release in reverse acquisition order after the
body. Plain legacy declarations retain manual lifetimes. `write_restart` executes
where written, before later cleanup, but produces an ordinary file with no
resumability guarantee.

The bounded effect subset rejects opaque commands and runtime control flow.
`run` and `minimize` require AtomsDefined and ModelDefined; positive runs also
require an integrator on group all. `run 0` does not require an integrator.
File operands must interpolate declared resource parameters, including inside
called modules. Constructing an undeclared resource or using `@unchecked` fails.
Every declared output needs a generating command. Input data is read with
`nocoeff` so coefficient sections are not applied; see the official
[read_data specification](https://docs.lammps.org/read_data.html). The model and
potential dependencies must be declared in the source instead.

`check`, `inspect dependencies`, `build`, and `target requirements` accept the
same stage selection and bindings. `inspect stages` lists sorted definitions,
parameter types, and input/output directions without binding files or claiming
that unselected bodies were verified. Selected compilation checks one body and
its module calls. All definition names and module call graphs are checked.
`check` without a stage selection rejects a source containing stages.

JSON reports and build manifests include the bound `stage`, instance cwd,
arguments, dependencies, and target profile reference. Quantity argument values
use the compiler's fs/angstrom/K/eV bases and dimensional type names. A successful
selected compilation sets `independent_execution: true` for the supported command
and file-resource subset. File dependency completeness is scoped to those
declared resources, not input-file contents or the execution environment.
`resumable` is always false. After execution, `observed_outputs` reports which
declared regular files exist; a missing file turns an otherwise successful run
into exit code 3. Existing files do not prove freshness or provenance. Repeating
an instance may overwrite its declared outputs; no caching or content identity
is implied.

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

Cross-stage artifact identities, invocation DAGs, content hashes, full provenance,
checkpoint manifests, restoration contracts, and caching are not implemented.
The current interface exports individual definitions and bound tasks. External
tools can connect explicitly named file outputs and inputs, but Lamlet does not
yet verify their producer identity or export an invocation graph. `@system`,
`@resume`, implicit `@output`, Structure/Restart/Checkpoint type aliases, external
module imports, user contexts, and generic effect/constraint solvers remain
unavailable. They are not silently treated as verified boundaries.
