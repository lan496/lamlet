# Run the P3 verification pilot

This development pilot grades a bounded subset of the public Nb impact task.
It combines an independent Python oracle, the official LAMMPS Python API, and
Harbor's separate verifier. The oracle never imports Lamlet or uses its verdict.
P1 and P2 still have task templates only.

The original protocol-v1 [recorded admission baseline](https://github.com/lan496/lamlet/blob/main/research/harbor/p3/admission-baseline.json)
contains the oracle and reference hashes, engine pin, 13 native case outcomes,
and two live Harbor outcomes from 2026-11-08. All expected outcomes matched.
The Harbor checks also confirmed absent verifier entry points in the agent
phase and ignored forged reward artifacts. These admission runs used authored
solutions. A subsequent six-trial Codex A/C pilot is reported on the
[evaluation page](agent-evaluation.md). A post hoc native protocol-v2 regrade
passes all four unchanged submissions; the two provider-limit trials remain
unassessed. The [v2 admission evidence](assets/evaluation/2026-11-08-protocol-v2/admission.json)
records 24/24 expected outcomes. The
[separate-container Harbor admission on 9 November](assets/evaluation/2026-11-09-harbor-v2/report.json)
also matches all 24 expected outcomes. It used authored solutions and made no
new model calls. See the [results](agent-evaluation.md#protocol-v2-admission-through-harbor-9-november-2026).

## Understand the pilot contract

The reference contains a 16000-atom BCC Nb projectile and a 32000-atom target,
periodic x and y boundaries, a free z boundary, and the pinned EAM potential.
For this pilot, the initial 1.5 nm gap means **15 Angstrom between the outermost
atomic planes**. Half-open region bounds avoid extra surface planes.

The oracle observes the actual atoms, group membership, periodicity, potential
energy, and the per-atom velocity change. It checks a requested 100 ps, 300 K NVT
schedule followed by a 2 km/s projectile boost and NVE impact integration.
It replaces each `run N` with at most two actual steps for admission testing.
NVT and NVE may each use one all-atom fix or multiple fixes on nonempty,
disjoint groups that cover every atom exactly once. The evaluator checks actual
LAMMPS group membership; group names have no special meaning. Every NVT fix
must request 300 K at both endpoints with positive damping. LAMMPS updates the
atoms in each fix group, as specified in the [Nose–Hoover documentation](https://docs.lammps.org/fix_nh.html).

Protocol-v2 limits atom count to 100000. Define the resource bound
`B(lengths) = 2 * product(ceil(length/lattice_spacing) + 4)` for three box lengths.
The simulation box must have `B <= 5000000`, including vacuum. Before each
`create_atoms`, existing atom count plus `B` of the intersection between the box
and the creation region must be at most 100000. The margin includes boundary
cells; this conservative admission limit can return `Unknown` even below the
actual atom-count limit. Box coordinates divided by lattice spacing must have
magnitude at most 100000000 to bound lattice indexing. Static unused regions
have no separate size cap. These limits replace v1's arbitrary per-region limits
and are disclosed in newly prepared campaign instructions.

Consequently, `correct` in this pilot means that the implemented protocol checks
pass. It does not mean that 100 ps of equilibration was executed, that the slabs
are equilibrated, or that collision and spallation results are converged.
Reports explicitly record `full_equilibration_executed: false`.

## Build the pinned engine

Run from the repository root. Git, CMake 3.27 or later, Ninja, a C++ compiler,
and Python 3.9 or later are required. Fetching requires network access.

```sh
python3 tools/evaluation.py fetch
python3 tools/p3_runtime.py --jobs 4
python3 tools/p3_admit.py --output target/p3-admission/local
```

Use a fresh output directory for each admission. The engine pin and build flags
are in [runtime.json](https://github.com/lan496/lamlet/blob/main/research/harbor/p3/runtime.json).
The serial build enables MANYBODY and disables MPI and OpenMP.

The v2 admission report covers 24 cases: eight valid inputs, thirteen deliberate
physical errors, and three unsupported or resource-limited inputs. Errors
include the wrong gap, speed, direction, atom count, boundaries, schedule,
integrators, potential, and replacement of thermal velocities.
Unsupported syntax returns `Unknown` without executing that syntax. Excessive
atom creation is rejected before allocation. Valid alternatives include large
vacuum boxes, translated origins, split run schedules, and disjoint NVT/NVE fixes.

## Regrade archived submissions

After successful native admission, regrade the preserved inputs without editing
them or rerunning the agent:

```sh
python3 tools/p3_regrade.py --admission target/p3-admission/local --output target/p3-regrade/local
```

Use a fresh output directory. The command verifies all original archive hashes,
requires admission of the exact oracle and runtime, checks runtime library
hashes, and records new assessments alongside the original statuses. It archives
evaluator source, admission evidence, and checksums. Missing submissions remain
unassessed. A post hoc regrade is distinct from a new controlled campaign.

## Build the container task

With a working Docker daemon, Python 3.12, and uv:

```sh
python3 tools/p3_container.py --output target/p3-container/local
uv run --python 3.12 --with harbor==0.25.0 harbor run --path target/p3-container/local/tasks --agent oracle --n-attempts 1 --n-concurrent 1 --jobs-dir target/p3-harbor --job-name reference
```

`--docker-config` and `--docker-host` on the builder select an explicit Docker
configuration and daemon. Set the corresponding `DOCKER_CONFIG` and
`DOCKER_HOST` environment variables for Harbor if you use those options.

The builder packages the reference, pinned potential, and runtime into a local
condition-A task. Content-derived local tags and image configuration IDs are
recorded in `images.json`. These are daemon-local identifiers, not portable
registry digest references. Debian's base image and LAMMPS source are pinned;
APT packages are not snapshot-pinned. Archive the tested images for a campaign.

The agent image contains the engine and potential. The separate verifier image
adds the oracle and tests, with networking disabled. Only the submission
directory transfers between phases. The adapter ignores agent-written reward
and assessment files and obtains fresh evidence from the trusted oracle.

## Review the evidence

Inspect `report.json` from native admission and Harbor's `result.json`, trial
logs, and verifier `assessment.json`. A Harbor command can finish while its
trial reports an exception, so check the trial outcome and reward explicitly.
An unresolved criterion produces no scientific success reward.

First time the reference in the pinned Harbor environment:

```sh
uv run --python 3.12 --with harbor==0.25.0 python tools/p3_harbor_admit.py --container target/p3-container/local --output target/p3-harbor/timing --case reference
```

Use its measured duration to budget the full 24-case admission, then run:

```sh
uv run --python 3.12 --with harbor==0.25.0 python tools/p3_harbor_admit.py --container target/p3-container/local --output target/p3-harbor/admission
```

Pass the same Docker options as the builder when required. The admission runner
passes those settings to Harbor as well. It verifies frozen task files, image
IDs, and the oracle hash inside the verifier image before starting. A plan records
every expected grade, mutation criterion, and submission hash before any case
runs. Reports retain failed cases and are saved after each case; the reference
must pass before the remaining cases start.

All 24 cases use authored solutions, without model calls. During each agent
phase, the solution asserts that the oracle and test entry points are absent.
It submits forged reward and assessment files. The fresh, separate verifier must
ignore those files and produce the expected grade, including the specific failed
criterion for each physical mutation. These checks cover the tested isolation
conditions, not every possible container attack.

An unsupported or resource-limited input must produce `unassessed` and no reward
file. Harbor 0.25.0 represents this as `RewardFileNotFoundError`. Admission accepts
that exception only when independent evidence establishes the expected Unknown
outcome and no infrastructure error occurred. Other exceptions or missing
evidence fail admission.

Archive a successful full admission without rerunning it:

```sh
python3 tools/p3_harbor_admit.py --output target/p3-harbor/admission --publish docs/assets/evaluation/YYYY-MM-DD-harbor-v2
```

Use a fresh destination. Publication rechecks frozen sources, submissions, and
verifier evidence and copies the inputs, source, assessments, Harbor results,
and checksums. Timing subsets and incomplete or failed runs cannot be published
as a full admission. Local image IDs record the measured environment; they are
not portable registry digests or evidence of an image archive.

The oracle accepts a limited straight-line command subset with static
arithmetic variables. Includes, jumps, shell commands, Python callbacks,
dynamic schedules, arbitrary styles, and changes to the model after dynamics
begins are outside the pilot. This restriction belongs to the evaluator;
Lamlet's legacy parser remains permissive.

## Run the six-trial Codex pilot

The [campaign Dockerfile](https://github.com/lan496/lamlet/blob/main/research/harbor/p3/Campaign.Dockerfile)
builds a common shell/LAMMPS image (`common`) and adds the Linux Lamlet CLI
(`checker`). Supply the admitted runtime image as `RUNTIME_IMAGE`. Build from a
small context containing that Dockerfile, `Cargo.toml`, `Cargo.lock`, and `crates/`.
Use the same common image for A and C; only C adds Lamlet. APT dependencies are
not snapshot-pinned, so archive the resulting images for exact environment reuse.

With those images tagged `lamlet-p3-campaign-a:v1` and
`lamlet-p3-campaign-c:v1`, and the admitted container directory available:

```sh
python3 tools/p3_campaign.py prepare --container target/p3-container/local --output target/p3-campaign/local
python3 tools/p3_campaign.py run --output target/p3-campaign/local
```

The runner uses the current Codex `auth.json`, or `CODEX_AUTH_JSON_PATH` when
set. Authentication is supplied to Harbor without copying it into public
evidence. The prepared manifest freezes model settings, image IDs, instruction
hashes, ordering, and the oracle hash. Six trials run serially without replacement;
results and a resumable `state.md` are written after each trial.

The `static` subcommand checks the native admission inputs with the real Lamlet
CLI. The `publish` subcommand archives a completed campaign, replay, admissions,
and static report. Use `--help` for explicit input paths. It publishes tool calls
and observations, not model reasoning or credentials. Retain unassessed and
infrastructure outcomes when interpreting its summary.

## Prepare a larger scientific campaign

Before further model trials, obtain domain review of the reference, tolerances, gap
definition, and oracle coverage. Extend runtime checks for the intended
scientific claims, implement P1 and P2, freeze model settings and budgets, and
prepare held-out tasks. Protocol-v2 has been admitted in the separate Harbor
verifier image; changes to the frozen oracle or runtime require fresh admission
before prospective model trials. The native revision and post hoc regrade
retain all old grades. The current pilot does not establish that Lamlet improves
scientific correctness.

See the [agent evaluation protocol](agent-evaluation.md) and
[oracle transport contract](https://github.com/lan496/lamlet/blob/main/research/harbor/oracle-contract.md).
