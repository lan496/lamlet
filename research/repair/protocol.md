# Prospective repair pilot

## Question and status

Does adding the Lamlet checker change completion and validation cost for three
small LAMMPS repair tasks under a fixed time allowance? This is a development
pilot, separate from the earlier P3 generation pilot. Independent human review
of the lint labels and repair contracts is pending. These trials are not the
confirmatory experiment requested in Issue #3.

## Tasks and split

`tools/repair_tasks.py` defines six authored tasks. Three development tasks cover
an NVT-to-NVE transition, time-averaging intervals, and lattice orientation.
Each supplies a complete, short, Lennard-Jones simulation with one defect. The
instructions specify the intended model and permitted edits. The independent
oracle checks those edit restrictions and explicit arithmetic, then executes
the entire requested short trajectory. It does not import or invoke Lamlet.
No potential file is needed for this analytic pair model.

Three additional parameterizations are reserved as `held_out`. They are frozen
with the task contracts but are neither used for oracle admission nor assigned
to agents. This split is a development reservation, not evidence that a model
has never seen related material. The development inputs were not used to change
the checker. Human review and a broader externally authored task set are needed
before a confirmatory campaign.

The physical and mathematical conditions come from the pinned LAMMPS manual:
[integration](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nve.rst),
[Nose-Hoover dynamics](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst),
[averaging](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst),
and [lattice directions](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst).

## Conditions and budget

A provides the shell and the pinned serial LAMMPS engine. C adds the existing
Lamlet binary and its invocation instructions. Both start from the same base
image, wrappers, starter input, and task instructions. Codex CLI 0.166.0 runs
`gpt-6.1-sol` with `xhigh` reasoning effort, service tier `fast`, and web search
disabled. The provider does not expose an exact backend snapshot or sampling
seed. Generic documentation access remains available through the shell in both
conditions. Agents are instructed not to retrieve benchmark sources or solutions.

Six trials are assigned before launch, one per task and condition. Seed 496
shuffles task order and condition order within each task. Each fresh agent
container receives two CPUs, 2048 MB memory, and 180 seconds for execution.
Stopping occurs at agent completion or timeout. No separate token or tool-call
ceiling is imposed. The assigned maximum is 1080 agent seconds, excluding setup
and verification. Dollar cost and CPU seconds are unmeasured. A non-task provider
probe precedes collection. The first assigned trial provides a wall-time estimate
for the remaining five; its outcome remains in the analysis.

Provider failures, timeout, missing submissions, unsupported syntax, and verifier
infrastructure failures remain in the assigned denominator without replacement.
They are reported separately from assessed failures. Historical archives and
the checker remain unchanged after observing submissions.

## Verifier admission and isolation

Native admission checks references, unchanged defects, unsupported expressions,
and a valid scaled orientation. Harbor 0.25.0 admission runs seven authored
submissions through separate verifier containers. It tests three references,
three defects, and one unsupported case. The agent environment must lack the
verifier and its tests. Forged reward files submitted by the authored solutions
must not affect the separate grade. Unsupported cases receive no reward.

Freeze task files, instructions, oracle and runner source, image IDs, reference
solutions, runtime identities, and admission evidence before model trials.
Recheck frozen files and local image identities before every trial. The verifier
has no network and returns no feedback during a trial. Local image IDs identify
the tested environment; they are not portable registry digests or snapshot-pinned
Debian package repositories.

## Measurements

The primary measure is independently verified completion within 180 agent
seconds divided by all assigned trials. Correctness concerns the stated edit
contract and short-run execution. It does not establish equilibrium, material
properties, numerical reproducibility, or general scientific correctness.
Report task-level outcomes, agent and wall time, tool calls, provider-reported
input/output/cached tokens, and instrumented LAMMPS launches. Cached tokens are
included in input totals.

Wrappers record candidate hashes each time `lmp` or `lamlet check` is invoked.
Validated candidate transitions count adjacent changes in these hashes plus the
final submission. They approximate repair activity but omit edits never passed
to a validator. The initial defective input can be one such candidate. These are
not all editing iterations or Pass@1. The wrapper evidence is agent-writable and
can be bypassed by invoking the underlying binary. It is usage instrumentation,
not part of the correctness oracle. Publish missing measurements as null.

Measure checker warnings separately on the three authored references and final
submissions. Do not use checker warnings to assign oracle grades. Report warning
codes and unresolved findings; a documented advisory is not automatically a
scientific false positive. Independent domain review remains required.
