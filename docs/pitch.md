# Lamlet: make the calculation explicit

For researchers maintaining LAMMPS inputs, research software developers, and
teams using AI agents to edit simulation code. This pitch introduces the current
v0.1 implementation. The accompanying [demo](../demo/README.md) takes about five
minutes to present.

## The 30-second pitch

Lamlet gives LAMMPS inputs explicit units, file dependencies, and calculation
boundaries. Start with an existing script, add checked values where they help,
and compile back to ordinary LAMMPS. New stages declare their inputs and outputs
and build independently. Researchers can inspect the generated script; AI agents
get deterministic JSON diagnostics. The engine remains LAMMPS, and unsupported
legacy commands remain usable.

## The three-minute pitch

### A script carries more than commands

A LAMMPS input records a calculation, but some of its assumptions live elsewhere.
What unit does this number use? Which directory supplies this file? Where does
this fix stop being active? Does this production step construct its own system,
or does it depend on whatever the previous script left behind?

Those questions matter when a researcher revisits a calculation. They also
matter when an AI agent changes one parameter in a larger script. A plausible
edit is not enough. We need feedback about the assumptions the edit touches.

### Make those assumptions explicit

Lamlet compiles to ordinary LAMMPS input. You can start with an unchanged `.in`
file and adopt its features incrementally.

Write `2 fs` next to the timestep. Lamlet emits `0.002` for metal units and
rejects a temperature in that position. Declare an input with `path(...)` and an
output with `output(...)`. The compiler checks the typed reference and includes
it in the dependency report. Declare a managed fix inside a scope, and the
compiler emits its cleanup when that scope ends.

These checks produce a script you can read and run with LAMMPS.

### Give a calculation its own boundary

Modules reuse command sequences under checked state contracts. Stages describe
individual calculation tasks with explicit file inputs, outputs, and parameters.

Build the same stage at two temperatures. Each instance gets a separate script
and output directory. A selected stage begins with empty simulation state.
Within the supported subset, it must establish the atoms, model, and integrator
that its run requires. It cannot borrow an invisible system from a previous
process. Unsupported effects fail at this boundary.

### Make the feedback usable

The CLI returns one JSON object with diagnostic codes, source spans, and exit
codes. An agent can inspect a dependency, change a value, rebuild one stage, and
check the result. Unknown findings remain explicit. The compiler does not
silently change the physical model.

Our recorded corpus contains 998 selected upstream inputs and fragments. Every
output was byte-identical, with no standard or strict rejections. That measures
preservation and acceptance for those pinned inputs, not numerical correctness.
The live demo separately checks units, error reporting, stage bindings, and
resource cleanup without starting LAMMPS.

### Start with one change you can inspect

Try Lamlet on one existing input. Make one timestep explicit, inspect the emitted
LAMMPS, then extract one calculation stage when its inputs and outputs are clear.
That is the adoption path we can demonstrate today.

## Claims to keep precise

| Say | Scope |
| --- | --- |
| Existing scripts can pass through unchanged. | Legacy compilation preserves bytes; partial validation can still diagnose known errors. |
| The recorded corpus has 998 identical round trips and zero rejections. | Selected inputs at pinned revisions; see [methods and results](corpus.md). |
| Stages build individually and run in fresh processes. | Checked command subset, declared resources, and explicit runtime engine; the demo builds without running MD. |
| Agents receive deterministic diagnostics. | Repeated input and options produce the same diagnostics for the same resolved resources and target profile. |
| The compiler catches supported contract violations. | A successful check does not prove complete coverage, physical validity, or numerical stability. |

Checkpoint restoration, workflow invocation DAGs, full provenance, and numerical
reproducibility guarantees are not implemented. Keep them in the future-work
discussion rather than the product demonstration. Current module and stage
limits are listed in [the contract](modules-and-stages.md).

## Present the implementation

Run `python3 demo/run.py` after building the release CLI. Open the generated
`target/demo/index.html` and use the arrow keys to present. The HTML embeds the
captured source, generated scripts, commands, diagnostics, and evidence. It can
be shared as a single offline file.

Use the [presenter runbook](../demo/README.md) for the sequence and the
[getting-started guide](getting-started.md) for hands-on use.

## The research pitch

Can compiler feedback help an AI agent produce scientifically correct molecular
dynamics inputs within a fixed budget? Lamlet makes that question testable on
ordinary LAMMPS scripts. Compare its diagnostics with a published AST-based
skill, keep the coding model and resources fixed, and judge the resulting
simulations with an independent evaluator. Test typed Lamlet separately so the
value of the checker and the value of the language remain distinct questions.

This is a research hypothesis, with no measured improvement claimed. The
[protocol](agent-evaluation.md) lists the controlled comparison, oracle
requirements, and current gaps. The [offline pilot](../research/holbrook/README.md)
replays published inputs without model calls or MD execution.
