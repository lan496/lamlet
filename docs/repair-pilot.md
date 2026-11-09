# Prospective LAMMPS repair pilot

## Scope

A six-trial development pilot compared shell and LAMMPS access (A) with the
same environment plus the Lamlet checker (C) on three short repair tasks.
All three A trials produced independently accepted inputs. Two C trials
produced accepted inputs; the third stopped during environment setup without
starting Codex. This experiment does not establish an improvement from Lamlet.
The checker, task contracts, verifier, assignments, and budgets were frozen
before model trials. Independent human review remains pending.

This pilot is separate from the [earlier P3 generation experiment](agent-evaluation.md).
It evaluates compliance with restricted repair contracts and successful short
trajectories. It does not measure equilibrium, material properties, or general
scientific correctness.

## Methods

### Tasks and independent assessment

The three development tasks used a periodic, 256-atom Lennard-Jones system.
Each starter contained one defect. Table 1 describes the permitted repairs.
The [archived contracts](assets/evaluation/2026-11-09-repair/task-contracts.json)
also contain three reserved parameter variants. Those variants were not
executed, admitted, or assigned to agents. They remain a development reservation
within these same task families, rather than an independent test of transfer.

*Table 1. Authored development tasks and their restricted edit contracts.*

| Task | Defect | Required repair |
| --- | --- | --- |
| Integrator transition | NVT remains active when NVE begins. | Insert `unfix thermal` between the 20-step NVT and 40-step NVE phases, preserving other commands. |
| Time averaging | `fix ave/time` uses `4 7 26`. | Change only the output interval to 28, preserving the requested contiguous, nonoverlapping windows and 120-step trajectory. |
| Lattice orientation | Fixed x = (1, 1, 0) and z = (0, 0, 1), with y = (0, 1, 0). | Change only integer y components to give nonzero, orthogonal, right-handed directions; execute 40 steps. |

The [independent verifier source](assets/evaluation/2026-11-09-repair/sources/tools/repair_tasks.py)
checks command preservation, resource lifetimes, or integer arithmetic as
appropriate. It then executes the full requested short trajectory in a fresh
LAMMPS process. It neither imports nor invokes Lamlet. Its command reader
supports the disclosed straight-line subset; dynamic expressions and unsupported
syntax produce `unassessed`, rather than a guessed grade. Successful execution
alone cannot override a failed edit contract.

Native admission matched all 10 expected outcomes: three valid references,
three unchanged defects, three unsupported cases, and one valid scaled
orientation. Separate-container Harbor admission matched all seven authored
cases: three references, three defects, and one unsupported case. The agent
containers lacked verifier tests, and forged reward artifacts did not change
the separate grades. These are verifier-admission results, not agent successes.
The [native report](assets/evaluation/2026-11-09-repair/admission.json) and
[Harbor report](assets/evaluation/2026-11-09-repair/harbor-admission.json)
retain the assessments and engine output.

### Conditions, freezing, and budget

Codex CLI 0.166.0 used `gpt-6.1-sol`, `xhigh` reasoning effort, and service tier
`fast`, matching the active local settings. Harbor 0.25.0 created fresh
containers with two CPUs and 2048 MB memory. A provided the shell and serial
LAMMPS; C added the current source-built Lamlet binary and invocation guidance.
Instructions, instrumentation, starter inputs, and engine were otherwise shared.
Generic documentation access was available to both conditions. The hidden
verifier returned no feedback during agent execution.

One trial per task and condition was assigned using shuffle seed 496. The
configured agent timeout was 180 seconds per trial, with no separate token or
tool-call ceiling. Infrastructure failures remained in the six-trial denominator
without replacement. A non-task provider probe succeeded before collection.
The first assigned trial took 113.823 host monotonic seconds; multiplying that
measurement by five gave an estimated 569.114 seconds for the remaining trials.
That extrapolation was a planning estimate, not a measured campaign duration.

The [frozen manifest](assets/evaluation/2026-11-09-repair/manifest.json) records
assignments, source hashes, binary identities, and local Docker image IDs.
Its SHA-256 is
`cfb5e28b6f4e6effebf504847e605f1ad4bcc97db0cd85a5264948ee572c53e9`.
The [precollection protocol](assets/evaluation/2026-11-09-repair/sources/research/repair/protocol.md)
defines stopping and missing-outcome policies. Neither the verifier nor the
checker changed after model submissions were observed. An earlier preparation
was superseded after preflight detected a stale cached checker; no model trials
ran in that preparation.

## Results

### Completion and validation

Table 2 retains all six assigned trials. A completed 3/3, and C completed 2/3.
Among submitted inputs, A passed 3/3 and C passed 2/2. The remaining C trial
failed while downloading Node.js 22.23.4 during agent setup, before Codex ran
or Lamlet was invoked. It has no submitted input, token usage, or scientific
assessment. The [infrastructure record](assets/evaluation/2026-11-09-repair/infrastructure.json)
preserves the HTTP/2 download error. It is neither an assessed repair failure
nor evidence of a Lamlet diagnostic failure.

*Table 2. Completion under the configured 180-second agent allowance. Unassessed
trials remain in the assigned denominator; no replacement trials were run.*

| Condition | Assigned | Submitted | Accepted | Assessed incorrect | Unassessed | Completion / assigned |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A: shell and LAMMPS | 3 | 3 | 3 | 0 | 0 | 3/3 |
| C: A plus Lamlet | 3 | 2 | 2 | 0 | 1 | 2/3 |

All five submissions passed the independent edit contract and completed their
requested trajectories. Instrumented validation recorded no candidate hash
transitions: agents repaired the inputs before their first recorded validation.
This observation does not measure Pass@1 or every edit. It provides no evidence
that checker feedback caused a repair. Instrumented Lamlet invocations occurred
in both C trials that reached agent execution.

### Usage and timing

Table 3 reports measured host monotonic durations and provider usage. Cached
input tokens are included in total input tokens. Tool calls count recorded
agent tool calls; one call can contain multiple shell commands. Engine launches
count wrapper invocations during the agent trial and exclude the independent
verifier. Wrappers and their logs were agent-writable and could be bypassed.

*Table 3. Per-trial measurements in assigned order. Host seconds include setup,
agent execution, and verification. Dashes denote missing observations.*

| Task / condition | Host seconds | Input tokens | Cached input | Output tokens | Tool calls | Engine launches | Lamlet invocations |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Integrator / A | 113.823 | 50,731 | 29,056 | 557 | 3 | 1 | 0 |
| Integrator / C | 72.361 | — | — | — | 0 | 0 | 0 |
| Orientation / C | 136.614 | 68,620 | 35,328 | 1,282 | 4 | 2 | 2 |
| Orientation / A | 141.997 | 50,369 | 24,832 | 574 | 3 | 1 | 0 |
| Averaging / A | 142.435 | 52,504 | 42,880 | 572 | 3 | 1 | 0 |
| Averaging / C | 158.710 | 72,888 | 71,552 | 819 | 4 | 1 | 1 |

Harbor timestamp intervals disagreed with host monotonic measurements. Three
reported agent intervals exceeded the enclosing trial durations, reaching
985–1057 seconds. The cause was not established. Agent-only execution time is
therefore missing for all trials; raw timestamps are retained but excluded from
runtime comparisons. All five successful trials completed within 180 seconds
on the host monotonic clock. This does not resolve the clock discrepancy or
validate an agent wall-time speed comparison. CPU seconds and monetary costs
were not measured. See the [timing audit](assets/evaluation/2026-11-09-repair/timing-quality.json)
and [trial summary](assets/evaluation/2026-11-09-repair/results.json).

### Checker observations

Before collection, the frozen checker warned on all three original defects
and produced no warnings on the three authored references. The corresponding
codes were `LML-PHYS-001`, `LML-MAN-AVE-TIME`, and
`LML-MAN-LATTICE-ORIENTATION`. The five final submissions also produced no
warnings or errors. Every checked input retained unresolved findings: 14–16
`Unknown` diagnostics per input. Absence of warnings does not establish complete
static verification.

The [pretrial observations](assets/evaluation/2026-11-09-repair/pretrial-checker.json)
and [complete checker report](assets/evaluation/2026-11-09-repair/checker-report.json)
are separate from oracle grades. These small, author-labeled controls establish
coverage of the selected diagnostic conditions. They do not estimate diagnostic
precision or the false-positive rate on independently labeled scientific inputs.

## Limitations and interpretation

The three narrow, authored tasks exercise existing rules and are readily solved
without the checker. One trial per task and condition, together with a missing
C execution, cannot support an effectiveness estimate. No corrective feedback
loop was observed. Broader tasks and independent contract and label review are
required before a confirmatory comparison. The reserved variants have not been
evaluated and share families with the development tasks; they cannot establish
generalization to unseen scientific problems.

Binary hashes and local image IDs identify the tested environment, but the
runtime is not fully hermetic. Agent setup downloaded Node.js from the network,
and package repositories were not snapshot-pinned. The exact model backend
snapshot was not exposed. Provider availability, setup reliability, timing
consistency, and instrumentation integrity remain separate from scientific
assessment. No full equilibration, experimental agreement, or checkpoint
guarantee was tested.

## Evidence and reproduction

The [archive checksums](assets/evaluation/2026-11-09-repair/hashes.json) cover
inputs, assignments, tool calls, provider usage, assessments, validation events,
and frozen sources. Private model reasoning and credentials are excluded.
The postprocessing audit checks completeness, frozen hashes, submission digests,
grades, usage totals, and aggregate denominators:

```sh
python3 tools/repair_evidence.py verify --public docs/assets/evaluation/2026-11-09-repair
```

To collect a new campaign, first provision the pinned native engine and the A
base image described in the [P3 runtime guide](p3-pilot.md#run-the-six-trial-codex-pilot).
The repair driver expects the base image named by `BASELINE` in its archived
source. Configure Docker access and provide Codex authentication privately;
never place credentials in an archive. Use a new output directory:

```sh
uv run --python 3.12 --with harbor==0.25.0 python tools/repair_campaign.py prepare --output target/repair-new
uv run --python 3.12 --with harbor==0.25.0 python tools/repair_campaign.py admit --output target/repair-new
python3 tools/repair_diagnostics.py --campaign target/repair-new --output target/repair-new/reference-checker.json
uv run --python 3.12 --with harbor==0.25.0 python tools/repair_campaign.py run --output target/repair-new --first-only
uv run --python 3.12 --with harbor==0.25.0 python tools/repair_campaign.py run --output target/repair-new
python3 tools/repair_evidence.py publish --output target/repair-new --public target/repair-new-public
```

Record a non-task provider probe as `provider-probe.json` before publication.
Preparation and admission must pass before model collection; measure the first
trial before continuing the sweep. The publisher is specific to this development
protocol, including its timing-quality policy. A future timing repair or broader
study requires a new protocol and dated archive, preserving these outcomes.
