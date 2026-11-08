# Evaluating Lamlet as an agent validation tool

## Development pilot results, 8 November 2026

A post hoc regrade with P3 protocol-v2 resolved all four submitted inputs as
passing the nine implemented protocol criteria. Table 1 retains the two trials
that produced no input because of provider usage limits. Among submitted inputs,
both conditions passed 2/2; among assigned trials, both passed 2/3. This small,
post hoc comparison provides no evidence of an improvement from Lamlet.

*Table 1. Native protocol-v2 regrade of the unchanged development-pilot submissions.
Provider-limit trials remain in the assigned denominator and are unassessed.*

| Condition | Assigned | Inputs submitted | Protocol pass | Protocol fail | Unassessed |
| --- | ---: | ---: | ---: | ---: | ---: |
| A: shell and LAMMPS | 3 | 2 | 2 | 0 | 1 |
| C: A plus Lamlet checker | 3 | 2 | 2 | 0 | 1 |

Protocol-v2 replaces the per-region coordinate and volume limits with bounds on
atom creation and simulation-box resources. It accepts NVT and NVE fixes on
nonempty, disjoint groups when actual atom memberships cover every atom exactly
once. Overlap, incomplete coverage, and non-300 K equilibration remain failures.
The [P3 contract](p3-pilot.md#understand-the-pilot-contract) defines the limits.
Native admission matched 24/24 expected outcomes: eight valid inputs, thirteen
physical mutations, and three unsupported or resource-limited inputs.

The [regrade report](assets/evaluation/2026-11-08-protocol-v2/report.json),
[admission evidence](assets/evaluation/2026-11-08-protocol-v2/admission.json),
individual assessments for
[A1](assets/evaluation/2026-11-08-protocol-v2/A1/assessment.json),
[A2](assets/evaluation/2026-11-08-protocol-v2/A2/assessment.json),
[C1](assets/evaluation/2026-11-08-protocol-v2/C1/assessment.json), and
[C2](assets/evaluation/2026-11-08-protocol-v2/C2/assessment.json), and
[checksums](assets/evaluation/2026-11-08-protocol-v2/hashes.json) preserve the
measurements and evaluator source. The source submissions and original archive
were verified against their original hashes before and after regrading.
No new model trials were run, and no revised feedback was returned to an agent.

These are native, bounded smoke-test grades: each requested run executes at most
two steps. Full equilibration and collision dynamics remain unmeasured. The
separate Harbor admission below checks transport of the revised grades. Both
conditions have identification bounds of 2/3 to 1 on protocol success when
unassessed trials are retained; these bounds are not confidence intervals.
The frozen protocol-v1 outcomes below remain the original campaign record.

### Protocol-v2 admission through Harbor, 9 November 2026

The frozen 24-case corpus was evaluated through Harbor 0.25.0 with separate
agent and verifier containers. All expected outcomes matched: eight valid
alternatives passed, thirteen deliberate physical mutations failed their
specified criteria, and three unsupported or resource-limited inputs remained
unassessed. The predeclared case set and checks are retained in the
[admission plan](assets/evaluation/2026-11-09-harbor-v2/plan.json) and
[report](assets/evaluation/2026-11-09-harbor-v2/report.json).

Each authored solution confirmed that verifier entry points were absent from
the agent container and submitted forged reward artifacts. The separate verifier
ignored those artifacts. Assessed inputs received the expected binary reward;
unassessed inputs produced no reward and retained Harbor's
`RewardFileNotFoundError`. All 24 assessments matched the frozen oracle and
submission hashes. The [checksums](assets/evaluation/2026-11-09-harbor-v2/hashes.json)
cover the published inputs, evaluator source, assessments, and transport evidence.

This admission establishes the tested verifier behavior and grade transport,
not an agent-treatment effect or full scientific validation. It used authored
solutions and bounded smoke execution, with no new model trials. Domain review,
held-out task coverage, and the prospective comparison remain outstanding.
The [P3 method](p3-pilot.md#review-the-evidence) gives reproduction commands and
the limits of the local image records.

### Original protocol-v1 campaign

A six-trial P3 development pilot compared the current Codex configuration with
and without Lamlet. The comparison did not yield an estimate of Lamlet's effect:
four submitted inputs exceeded the independent verifier's region bounds, and
two trials stopped at the provider's usage limit before producing an input.
Table 2 retains every assigned trial and separates these outcomes from physical
errors. No submission received a scientific correctness label.

*Table 2. Final outcomes under the frozen P3 protocol-v1 verifier.*

| Condition | Assigned | Inputs submitted | Correct | Incorrect | Unassessed | Provider limit |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A: shell and LAMMPS | 3 | 2 | 0 | 0 | 3 | 1 |
| C: A plus Lamlet checker | 3 | 2 | 0 | 0 | 3 | 1 |

Confirmed success was 0/3 in each condition. With all scientific outcomes
unresolved, the corresponding identification bounds are 0–100%, rather than
confidence intervals or evidence of equal performance. Infrastructure outcomes
were retained without replacement. The region limit was not changed after
observing submissions, and no oracle feedback was returned to the agents.

The pilot used `gpt-6.1-sol`, `xhigh` reasoning effort, the local `fast` service
tier setting, Codex CLI 0.166.0, and Harbor 0.25.0. Each fresh container had two
CPUs, 4096 MB memory, and a 600-second agent limit. Both conditions wrote ordinary
LAMMPS inputs and received the same public P3 prompt, pinned potential, and
bounded command-subset instructions. Seed 496 randomized condition order within
three blocks: A1, C1, A2, C2, C3, A3. No model sampling seed or exact backend
snapshot was available. Token and tool-call ceilings were not separately imposed.

Table 3 reports agent execution intervals and provider-reported usage. Cached
tokens are included in input tokens; they must not be added to input totals.
Both C submissions invoked `lamlet check`. Their diagnostics did not establish
the requested geometry or physics.

*Table 3. Trial-level execution and usage evidence. Dashes denote unavailable usage.*

| Trial | Agent seconds | Input tokens | Cached tokens | Output tokens | Tool calls | Lamlet check |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| A1 | 81.911 | 90,901 | 79,232 | 4,435 | 5 | No |
| C1 | 116.501 | 200,946 | 170,240 | 6,383 | 7 | Yes |
| A2 | 125.169 | 225,322 | 213,376 | 6,281 | 10 | No |
| C2 | 96.515 | 192,401 | 157,440 | 5,167 | 6 | Yes |
| C3 | 5.078 | — | — | — | 0 | No; provider limit |
| A3 | 3.851 | — | — | — | 0 | No; provider limit |

The frozen verifier limited region coordinates to magnitude 1000 and region
volumes to 2,000,000 in box units or 50,000 in lattice units. The four inputs
used larger vacuum boxes, yielding `Unknown` before geometry observations.
Those numeric limits were not disclosed in the task instructions. The verifier
also recognizes only a narrow all-atom NVT schedule; this does not cover every
valid implementation of isochoric equilibration. Protocol-v2 addresses these two
coverage limits through the post hoc admission and regrade above. The original results remain an operational pilot rather than
a scientific accuracy comparison.

Independent admission and checker coverage were measured separately. The native
oracle matched all 13 expected outcomes: three valid inputs, nine deliberate
physical errors, and one unsupported command. Two live Harbor admission runs
produced the expected reference/wrong-gap grades and ignored forged assessment
artifacts. Lamlet accepted all 13 native cases with zero errors and zero warnings,
including all nine physical mutations. Thus it rejected 0/9 known physical errors
in this small suite; its absence of a general geometry checker is material here.

The published-artifact replay is summarized in Table 4. Every input passed normal
and strict checking and was emitted byte-for-byte unchanged; every input also
contained at least one `Unknown` finding. Replay acceptance establishes neither
scientific correctness nor diagnostic precision. The replay took 5.179 seconds
on this host and launched no agents or LAMMPS processes.

*Table 4. Offline replay of the pinned public artifact release.*

| Artifact set | Inputs | Normal accepted | Strict accepted | Byte-identical | With Unknown |
| --- | ---: | ---: | ---: | ---: | ---: |
| Published one-shot P1 | 80 | 80 | 80 | 80 | 80 |
| Published one-shot P2 | 80 | 80 | 80 | 80 | 80 |
| Published one-shot P3 | 80 | 80 | 80 | 80 | 80 |
| Published agent examples, P1–P3 | 6 | 6 | 6 | 6 | 6 |

The [pilot manifest and trial summary](assets/evaluation/2026-11-08/pilot.json),
[complete replay diagnostics](assets/evaluation/2026-11-08/replay.json.gz),
[replay summary](assets/evaluation/2026-11-08/replay-summary.json),
[native admission](assets/evaluation/2026-11-08/native-admission.json),
[Harbor admission](assets/evaluation/2026-11-08/harbor-admission.json), and
[static mutation checks](assets/evaluation/2026-11-08/static-mutations.json)
archive the measured outcomes. The archive includes final inputs and tool evidence
for [A1](assets/evaluation/2026-11-08/A1/tools.json),
[C1](assets/evaluation/2026-11-08/C1/tools.json),
[A2](assets/evaluation/2026-11-08/A2/tools.json), and
[C2](assets/evaluation/2026-11-08/C2/tools.json), the
[A instructions](assets/evaluation/2026-11-08/A-instruction.md),
[C instructions](assets/evaluation/2026-11-08/C-instruction.md), and
[SHA-256 checksums](assets/evaluation/2026-11-08/hashes.json).
Private model reasoning and credentials are excluded. The
[P3 guide](p3-pilot.md#run-the-six-trial-codex-pilot) describes reproduction.

Full equilibration and collision dynamics, expert scientific review, Pass@1,
repair iterations, CPU time, and actual monetary cost were not measured. There
are no B/D trials or held-out tasks. The larger study below remains proposed.

## Research question

Can compiler feedback improve the scientific correctness of molecular dynamics
inputs generated by a coding agent, within a fixed resource budget?

The planned primary comparison uses ordinary LAMMPS inputs in both conditions. One agent
receives the published LAMMPS-AST skill. The other receives Lamlet diagnostics.
This tests the value of the validation tools. A later comparison lets the agent
write typed Lamlet and tests the value of the language separately.

The A/C development pilot above precedes this proposed B/C study. The
[offline replay](https://github.com/lan496/lamlet/blob/main/research/holbrook/README.md)
provides a separate compatibility measurement on published artifacts.

## Published baseline

Holbrook, Verduzco, and Strachan evaluated eight models on three tasks, with ten
generations per task and model. Their pipeline combines normalization, parsing,
shortened execution, and expert assessment. Table 4 reports 58, 23, and 1
scientifically accurate scripts out of 80 for P1, P2, and P3, respectively.
Their separate agent demonstration produced five correct artifacts in six
attempts. These are different experiments, not a controlled estimate of the
skill's effect. See the [paper](https://arxiv.org/html/2603.20630).

The released skill retries failed execution with `pair_style zero`. A successful
fallback diagnoses some execution problems while replacing the intended force
model. Record original-potential and fallback outcomes separately. Neither a
parser pass nor a ten-step run establishes scientific correctness. See the
[published execution script](https://github.com/ethanholbrook/LAMMPS-AST/blob/05fa329f175a8d50a1ac938ac55cc59791fd3e4c/lammps-ast-eval/scripts/run_lammps.py).

The replay pins public release commit
`05fa329f175a8d50a1ac938ac55cc59791fd3e4c`, rather than a moving branch.
That release postdates the paper's arXiv v2. Its presence does not establish
the exact code revision used for every published experiment. Historical model
snapshots, prompts, and environment details still need a replication audit.

## Task contracts

Use the upstream method descriptions unchanged for the replication arm. Fetch
them through the pinned source checkout instead of maintaining another copy.
The following summaries identify the required observables, not replacement
prompts.

| Task | Required calculation | Independent checks to prepare |
| --- | --- | --- |
| P1 | Al FCC, 5 cells in each direction, NPT at 300 K and 1 atm for 500 ps; Ercolessi–Adams potential; damping times 0.1 and 1 ps. | Crystal and atom count, potential identity, ensemble, unit conversion, damping times, total physical duration. |
| P2 | Ni, 10 cells in each direction; initial velocities at 600 K; continuous 300–2500 K ramp at 10 K/ps and 1 atm; Mishin 1999 potential. | Initial velocity distribution, ramp endpoints and duration, pressure control, potential identity, physical time across run segments. |
| P3 | BCC Nb projectile 20×20×20 and target 20×20×40 cells; impact along [001], 1.5 nm gap, 2 km/s added to projectile thermal velocity; 300 K equilibration for 100 ps. | Actual dimensions, group membership, gap convention, boundaries, relative COM velocity, velocity addition, equilibration, force model, and adiabatic impact integration. |

These conditions come from the pinned
[P1](https://github.com/ethanholbrook/LAMMPS-AST/blob/05fa329f175a8d50a1ac938ac55cc59791fd3e4c/publication/agent_tests/opus47/al_npt_eval/al_npt_prompt.txt),
[P2](https://github.com/ethanholbrook/LAMMPS-AST/blob/05fa329f175a8d50a1ac938ac55cc59791fd3e4c/publication/agent_tests/opus47/ni_melting_eval/ni_melting_prompt.txt), and
[P3](https://github.com/ethanholbrook/LAMMPS-AST/blob/05fa329f175a8d50a1ac938ac55cc59791fd3e4c/publication/agent_tests/opus47/spall_nb_eval/spall_nb_prompt.txt)
method descriptions. In P2, the 600 K initial velocities and the 300 K start of
the thermostat ramp are distinct requirements. The implied ramp duration is
220 ps, calculated as `(2500 - 300) / 10`.

Resolve ambiguous conventions before comparing tools. For example, geometric
region separation and the distance between outermost atom planes differ.
Freeze the intended interpretation of the P3 gap and its tolerance, and record
how surface atom placement affects atom-count expectations.

## Experimental conditions

| Condition | Agent writes | Additional agent feedback |
| --- | --- | --- |
| A | LAMMPS | Common shell, documentation, and LAMMPS execution tools. |
| B | LAMMPS | Pinned LAMMPS-AST skill and its structured diagnostics. |
| C | LAMMPS | Lamlet static diagnostics and command schemas. |
| D | Lamlet, compiled to LAMMPS | C plus typed expressions and supported language features. |

B versus C is the primary comparison. Keep A as a reference. Analyze D
separately because it changes the source language and the available checks.
Begin with one fixed coding model, three tasks, four conditions, and ten trials
per task and condition. This proposed pilot has 120 trials. It is not a power
analysis or evidence that ten trials can detect the expected effect.

The complete upstream skill includes normalization, execution, and domain
review instructions. An end-to-end B versus C comparison therefore measures
the whole tool package. To isolate static feedback, add a separate ablation
that holds review instructions and smoke-test tools constant and swaps only
the AST parser for the Lamlet checker. Do not describe the full comparison as
an isolated effect of dimensional types.

## Shared environment and budgets

Before an agent campaign, freeze the following items in a campaign manifest.
The current replay source manifest does not fill in these campaign choices.

- Exact model snapshot, reasoning effort, agent version, system instructions,
  context policy, and condition-specific instructions.
- Task prompt hashes, potential files and hashes, starting workspace, reference
  documentation, Lamlet binary hash, and upstream skill revision.
- LAMMPS binary hash, version, capability profile, MPI size, CPU and GPU policy,
  operating environment, and working-directory convention.
- Maximum elapsed time, tool calls, model-token budget, LAMMPS launches, and CPU
  time. Specify which limit ends a trial and how interrupted calls are counted.
- Oracle version, checks, tolerances, adjudication procedure, and exclusions.

Give all conditions the same input resources and execution privileges. Restrict
network access identically, or provide the same frozen documentation snapshot.
Only the declared intervention changes. Target capability information belongs
in that declaration if C receives it and B does not.

Create a fresh workspace and process for every trial. Randomize condition order
within task blocks and preserve the randomization seed. Pair model seeds when
the provider supports them, without assuming that a seed makes agent behavior
deterministic. Prevent artifacts or feedback from leaking between trials.

Archive the initial generated artifact before validation feedback. Then archive
every submitted repair, tool result, model usage record, elapsed time, and final
artifact. Store scripts and tool outputs, rather than asking for private model
reasoning. Tag infrastructure failures separately from incorrect submissions.
Define exclusion and replacement rules before inspecting condition outcomes.

## Harbor execution and benchmark maintenance

Terminal-Bench motivates evaluating an agent system, including its tools and
execution policy. It is not a LAMMPS benchmark. Harbor supplies the task format
and runner for our domain-specific tasks. Its local task-directory option is
`--path`; `--dataset` selects a named dataset. See the official
[job guide](https://docs.harborframework.com/jobs/run-a-job).

The [Harbor prototype](https://github.com/lan496/lamlet/blob/main/research/harbor/README.md) prepares P1–P3 task templates
for each A–D condition. It uses a separate verifier container, explicit artifact
transfer, and an adapter for an independently implemented scientific oracle.
The original method text stays intact. Common submission instructions and a
condition-specific tool description are appended and hashed. Conditions share
the final LAMMPS submission format, potential files, and required criteria.

Task structure alone does not make tests inaccessible. Harbor defaults to a
shared environment when no separate verifier is configured. The prototype
explicitly requests separate verification and copies no tests into the agent
image. The P3 live admission checks absence of verifier entry points during the
agent phase and rejection of forged assessment artifacts. Broader isolation
properties still need review. See the official [separate verifier guide](https://docs.harborframework.com/tasks/separate-verifier).

The [P3 pilot](p3-pilot.md) adds a bounded independent oracle, native mutation
admission, and a live reference run through separate Harbor containers. Its
instrumented runs establish the implemented protocol checks, without executing
100 ps of equilibration. P1 and P2 remain templates; the six-trial development
campaign above used only P3.

Scientific assessment is a separate outcome from the runner's reward. Missing
oracle code, engine failures, incomplete evidence, and unresolved criteria must
not become scientific successes. A missing submitted artifact is a task failure
without a scientific assessment. Archive detailed assessment reports so an
infrastructure error cannot disappear into an aggregate score.

Benchmark revisions and resource limits are part of the experiment.
Buchanan et al. report that repairing Terminal-Bench tasks and changing time
budgets can change measured performance and comparisons between agent systems.
That motivates versioning task instructions, images, required criteria, and
tolerances together. A verifier repair requires regrading archived artifacts
across all conditions where possible, with old and new labels retained. See
[Benchmarks as Software](https://arxiv.org/html/2611.08417).

Before admitting a task, require a reviewed reference submission to pass and
targeted incorrect submissions to fail. Include wrong physical units, wrong
geometry, removed velocity components, a substituted potential, and omitted
equilibration. Also test valid alternative implementations, malicious or stale
reward artifacts, missing outputs, and engine startup failures. Do not repair a
verifier by merely accepting a script that the current agent happened to write.

## Development and held-out evaluation

All published P1–P3 tasks are development and replication tasks. The current
manifest lists no held-out tasks. They cannot support a claim about unseen
LAMMPS problems, even if a particular prompt has not been used in a local run.

Wang et al. compare agent optimizers under fixed budgets and find that static
gains do not consistently transfer to newly introduced tasks. This is evidence
for testing transfer separately, not evidence about Lamlet's effectiveness.
See [Do Agent Optimizers Compound?](https://arxiv.org/html/2607.14004).

Freeze a held-out suite before final evaluation. Split by task family and source
lineage, rather than placing minor numerical variants of the same script in
both development and evaluation sets. Group related include trees, potential
families, and repair templates to limit leakage. Hold out generation and repair
tasks separately, and report their outcomes separately.

Candidate repair families include changing units, switching integrators,
adapting to missing packages, and extracting reusable modules. Restart and
checkpoint tasks belong to a later language evaluation because Lamlet does not
yet implement its proposed checkpoint guarantees. Freeze the checker, tool
instructions, and budgets before using the held-out suite. Add regression cases
to development after the study rather than tuning against the final test set.

## Independent correctness oracle

Lamlet's exit code is agent feedback. It is never the final correctness label.
The oracle must assess the emitted LAMMPS input with a separate implementation
and a frozen task contract. Agents must not access hidden oracle tests or their
intermediate results during the primary comparison.

For geometry and initial state, collect coordinates, box bounds, group
membership, masses, and velocities from a trusted LAMMPS observation procedure.
Do not accept an agent-written JSON description as evidence of the simulated
system. Convert engine observations to common physical units before checking
them. Keep the observation procedure and any script instrumentation fixed.

For P3, examine the state after equilibration and velocity assignment and before
impact dynamics. Check cell dimensions and population separately from region
bounds. Measure relative COM velocity and check that the thermal component is
retained. A correct velocity magnitude alone does not prove that it points
toward the target. A shortened run also cannot verify the full equilibration
duration or the intended integration schedule.

Validate potential identity and runtime use with pinned files. A zero-force
fallback cannot satisfy that requirement. Distinguish successful execution of
the unchanged physical model from a diagnostic execution with modified input.
For numerical checks, fix tolerances and evaluation resources before the trial.
Use blinded expert review for conditions the automated oracle cannot establish.

Oracle outcomes are `correct`, `incorrect`, and `unassessed`, with evidence per
criterion. Preserve unresolved outcomes. If unassessed submissions remain,
report their fraction and success bounds rather than silently treating them as
correct or omitting them from the denominator. The replay tool implements no
scientific oracle and writes `null` for scientific outcomes.

## Outcomes and analysis

| Outcome | Definition |
| --- | --- |
| Success@budget, primary | Fraction of assigned trials with an independently correct submitted artifact within every fixed budget. Assess archived submissions offline to avoid leaking oracle feedback. |
| Initial accuracy, Pass@1 | Fraction with an independently correct first artifact, captured before validation feedback. |
| Final accuracy | Fraction with an independently correct artifact selected by the agent at trial termination. |
| Repair iterations | Submitted revisions after the initial artifact, including unsuccessful trials and their stopping reason. |
| Diagnostic precision | Independently confirmed actionable findings divided by adjudicated actionable findings. Fix a deduplication rule and retain unresolved findings. |
| False rejection rate | Known-correct scripts rejected by the checker divided by independently known-correct scripts. Unknown findings are not rejections. |
| Cost | Model usage by reported token category, tool calls, elapsed time, engine launches, and CPU time. Report evaluator cost separately. |

Also report parser and original-potential execution outcomes to compare with the
published pipeline. Report fallback execution in its own column. Static
acceptance is a coverage observation and cannot estimate diagnostic precision
or the false rejection rate without independently labeled inputs.

Report per-task results and uncertainty, alongside the aggregate. Retain all
assigned trials under the preregistered failure policy. Choose the statistical
method before running the campaign. Treat ten trials per cell as a pilot and
use its uncertainty to plan a larger study. Keep new task variants separate
from the published tasks, whose public availability creates a contamination
risk.

## Current coverage and implementation order

| Needed feedback | Current Lamlet behavior | Work before claiming coverage |
| --- | --- | --- |
| Command arguments and state | Partial schemas, ID tracking, conservative Unknown after opaque effects. | Audit findings against the published artifacts and independent labels. |
| Units and physical duration | Typed quantities convert to known `metal` or `real` units. | Task-aware reasoning about legacy numbers, ramps, and accumulated run time. A bare number does not reveal the author's intended unit. |
| Geometry and impact setup | No general geometry checker. | Model a bounded subset of lattice, region, group, and velocity operations, with explicit Unknown for dynamic cases. |
| Physical consistency | Ten physical and idiom rules, 286 additional manual constraints, and supported state contracts. | Independent labels and precision measurements for the target tasks. |
| Target capabilities | Explicit profiles and style checks. | Freeze the actual evaluation target and plugin state. |
| Scope and stages | Supported opt-in resource and stage checks. | Separate tasks that measure cleanup and independent execution. P1–P3 do not establish their benefit. |

The additional [rules derived from the manual](lint-rules.md) were implemented
after the A/C pilot and its archived replay. Those measurements describe the
earlier binary identified in the pilot manifest. They do not measure the new
rules or establish an improvement in agent performance.

The separate [lint-condition evaluation](lint-evaluation.md) now records
detection, false warnings, and Unknown outcomes on a fixed engineering corpus.
Its labels and measurements concern selected diagnostic conditions, without
assigning scientific correctness to the pilot submissions.

The next implementation sequence is:

1. Replay the released artifacts and audit a labeled subset. Preserve raw inputs
   and diagnostics. Keep the six agent examples separate from the 240 one-shot
   scripts.
2. Implement and validate an independent P1–P3 oracle, starting with P3 geometry
   and velocity observations. Freeze the campaign manifest and tolerances.
3. Add narrowly justified legacy-input checks and evaluate their precision on
   both correct cases and deliberate mutations. Preserve unsupported scripts.
4. Run the controlled B versus C pilot with A as a reference. Add D only when
   the typed-language intervention and its cost are specified.
5. Add feature ablations and later multistage tasks for modules, restart, and
   checkpoint behavior. These remain separate hypotheses.

The offline replay, Harbor task preparation, and bounded P3 oracle are
implemented. Protocol-v2 native and separate-container Harbor admission both
match all 24 expected outcomes. P1 and P2 oracles, external scientific review, full
equilibration and collision validation, a larger prospective model comparison, general geometry checks,
and checkpoint guarantees remain outstanding. The [P3 guide](p3-pilot.md)
defines exactly what its protocol grade establishes.
