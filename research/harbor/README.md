# Prepare the Harbor benchmark prototype

Generate the Holbrook P1–P3 task templates for one agent-tool condition, or run
the [P3 verification pilot](../../docs/p3-pilot.md). P3 now has an independent
oracle, an authored reference, a pinned serial EAM runtime, native mutation
tests, and a live Harbor reference run. The pilot checks a requested protocol
using instrumented smoke execution. It does not execute full equilibration.
P1 and P2 still lack scientific oracles and reference solutions.

The generic preparation report keeps `ready_for_trials: false`. A six-trial
Codex A/C development pilot has run on P3. Four submissions exceeded verifier
coverage and two trials hit a provider usage limit; the pilot cannot estimate
Lamlet's effect. See the [recorded results](../../docs/agent-evaluation.md).

The [evaluation protocol](../../docs/agent-evaluation.md) separates the checker
comparison from the typed-language comparison and defines the independent
assessment requirements. All three public tasks are development tasks. No
held-out suite is claimed.

## Generate task directories

Fetch the pinned inputs, then prepare condition C:

```sh
python3 tools/evaluation.py fetch
python3 tools/harbor_tasks.py --condition C --output target/harbor-preview/C
```

The output contains three task directories and `preparation.json`. Each task has
`instruction.md`, `task.toml`, `environment/Dockerfile`, and a separate verifier
under `tests/`. Upstream prompts and potentials must match their recorded Git
blob IDs. The preparation report records their SHA-256 values and the final
instruction hash. Existing output directories are never overwritten.

Select A, B, or D with `--condition` and use a fresh output directory. A provides
the common shell and engine, B adds the upstream AST skill, C adds Lamlet's
legacy-input checker, and D asks for typed Lamlet compiled to LAMMPS. Tool
installation belongs in audited base images. The generator changes the tool
instructions but does not install or prove the presence of those tools.
Use `--task P3` to prepare just that task.

The agent submits `/app/submission/solution.in`. Each task includes the pinned
potential in `/app/inputs`. The separate verifier receives the submission and
its own clean copy of the potential. Tests are bundled only into the verifier
image. Published model outputs are not installed as reference solutions.

## Validate the task structure without running a model

The prototype targets Harbor 0.25.0 and task schema 1.4. Python 3.12 or later is
required for Harbor. The preparation and verifier tools use the standard
library and work on Python 3.9 or later.

To inspect the parsed tasks with the real Harbor package:

```sh
uv run --cache-dir target/uv-cache --python 3.12 --with harbor==0.25.0 python -c 'from pathlib import Path; from harbor.models.task.task import Task; print([Task(p).name for p in sorted(Path("target/harbor-preview/C").iterdir()) if p.is_dir()])'
```

To preview a local job configuration without starting a trial:

```sh
uv run --cache-dir target/uv-cache --python 3.12 --with harbor==0.25.0 harbor run --path target/harbor-preview/C --agent nop --n-attempts 1 --n-concurrent 1 --print-config
```

`--path` selects local task directories. `--dataset` selects a named dataset.
The `nop` agent and `--print-config` example validate configuration only. They do
not provide correctness evidence. See Harbor's official
[job guide](https://docs.harborframework.com/jobs/run-a-job).

## Supply audited runtime images

Without image arguments, the generated Dockerfiles contain a required
`BASE_IMAGE` argument with no default. They are reviewable templates and cannot
build as generated. Supply `--agent-image` and `--verifier-image` as immutable
`NAME@sha256:DIGEST` references when audited images are available. Tags are
rejected. Generate into a fresh directory after changing the inputs.

The common agent image must provide Python, shell tools, and the fixed LAMMPS
engine. B also needs the pinned AST skill and dependencies at
`/opt/agent-tools/lammps-ast-eval`. C and D need the Linux Lamlet binary on `PATH`;
D also needs its frozen language documentation. Keep engine versions, packages,
resources, and common documentation identical across conditions.

The verifier image must provide Python and the trusted executable
`/opt/lamlet-bench/oracle`, implementing the [oracle contract](oracle-contract.md).
It must independently inspect the submitted LAMMPS model and obtain trusted
engine observations. It must not use Lamlet's verdict or agent-written claims
as scientific labels. Candidate scripts remain untrusted input. Before runtime
validation, define engine execution privileges and protect the oracle, tests,
and reward directory from candidate-side operations.

The draft uses two CPUs, 4096 MB memory, 10240 MB storage, and provisional
600-second agent and verifier limits. Change the latter with `--agent-seconds`
and `--verifier-seconds`. The oracle subprocess has a 300-second default limit.
These are template settings, not measured sufficient budgets for P1–P3.
Agent networking is public in the template so model tools can connect. Freeze
the allowed endpoints and enforce the same policy across conditions before a
campaign. Verifier networking is disabled.

## Test the adapter and complete task admission

Run the offline transport and integrity tests:

```sh
python3 -m unittest discover -s tools -p 'test_*.py'
```

The tests cover missing or incomplete oracle evidence, stale rewards, subprocess
failures, digest requirements, and task packaging. Fixture oracle reports test
the adapter contract. They do not validate molecular dynamics.

Before launching paid trials, review the P3 oracle and reference with a domain
expert and implement the P1 and P2 oracles. Test valid alternatives and deliberate physical errors.
Build the containers and run Harbor's reference agent on every task. Confirm
that tests are unavailable during the agent phase and cannot be changed through
submitted artifacts. Freeze model settings, budgets, and task versions, then
prepare held-out task families. The generic task templates have no reference
solution. The P3 container builder supplies one for the `oracle` agent.

The P3 runtime and verifier images were built and the authored reference passed
through Harbor's separate verifier. The native admission also exercises two
valid alternatives, nine deliberate physical errors, and unsupported syntax.
Follow the [pilot guide](../../docs/p3-pilot.md) to reproduce these checks.
