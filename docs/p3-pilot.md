# Run the P3 verification pilot

This development pilot grades a bounded subset of the public Nb impact task.
It combines an independent Python oracle, the official LAMMPS Python API, and
Harbor's separate verifier. The oracle never imports Lamlet or uses its verdict.
P1 and P2 still have task templates only.

The [recorded admission baseline](https://github.com/lan496/lamlet/blob/main/research/harbor/p3/admission-baseline.json)
contains the oracle and reference hashes, engine pin, 13 native case outcomes,
and two live Harbor outcomes from 2026-11-08. All expected outcomes matched.
The Harbor checks also confirmed absent verifier entry points in the agent
phase and ignored forged reward artifacts. No model trials were run.

## Understand the pilot contract

The reference contains a 16000-atom BCC Nb projectile and a 32000-atom target,
periodic x and y boundaries, a free z boundary, and the pinned EAM potential.
For this pilot, the initial 1.5 nm gap means **15 Angstrom between the outermost
atomic planes**. Half-open region bounds avoid extra surface planes.

The oracle observes the actual atoms, group membership, periodicity, potential
energy, and the per-atom velocity change. It checks a requested 100 ps, 300 K NVT
schedule followed by a 2 km/s projectile boost and NVE impact integration.
It replaces each `run N` with at most two actual steps for admission testing.

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

The admission report covers 13 cases: the reference, two valid alternatives,
nine deliberate physical errors, and an unsupported shell command. Errors
include the wrong gap, speed, direction, atom count, boundaries, schedule,
integrators, potential, and replacement of thermal velocities.
Unsupported syntax returns `Unknown` without executing that syntax.

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

To repeat the live reference, wrong-gap, and forged-artifact checks with
`harbor` on `PATH`:

```sh
python3 tools/p3_harbor_admit.py --container target/p3-container/local --output target/p3-harbor/admission
```

Pass the same Docker options as the builder when required. This command verifies
the local image IDs before starting. During the agent phase it asserts that the
oracle and test entry points are absent. Both submissions contain forged reward
and assessment files; the fresh verifier must ignore those files and return the
expected independent grade. These checks cover those concrete isolation cases,
not every possible container attack.

The oracle accepts a limited straight-line command subset with static
arithmetic variables. Includes, jumps, shell commands, Python callbacks,
dynamic schedules, arbitrary styles, and changes to the model after dynamics
begins are outside the pilot. This restriction belongs to the evaluator;
Lamlet's legacy parser remains permissive.

## Prepare a scientific campaign

Before model trials, obtain domain review of the reference, tolerances, gap
definition, and oracle coverage. Extend runtime checks for the intended
scientific claims, implement P1 and P2, freeze model settings and budgets, and
prepare held-out tasks. This pilot has no measured agent success rate and does
not establish that Lamlet improves scientific correctness.

See the [agent evaluation protocol](agent-evaluation.md) and
[oracle transport contract](https://github.com/lan496/lamlet/blob/main/research/harbor/oracle-contract.md).
