# Build your first explicit LAMMPS task

Build an existing script unchanged, add a physical unit, and compile a calculation
stage at two temperatures. You will finish with ordinary LAMMPS inputs and
manifests that name each task's files and working directory.

Use a stable Rust toolchain and Python 3.9 or newer, with no extra Python
packages. No LAMMPS installation is needed for this
tutorial. The shell examples below use a POSIX shell.

## Prepare a workspace

From the repository root, build the CLI and run the demo:

```sh
cargo build --release --locked
python3 demo/run.py
```

Expect six `PASS` lines. Open `target/demo/index.html` for a visual walkthrough.
The runner copies the fixtures into `target/demo/workspace/` and checks the
commands used below. It never starts LAMMPS.

Save the binary's path, then enter that generated workspace:

```sh
LAMLET_DEMO_BIN="$PWD/target/release/lamlet"
cd target/demo/workspace
```

## Preserve an existing script

Build the unmodified input:

```sh
"$LAMLET_DEMO_BIN" build legacy.in -o legacy.generated.in --format json
cmp legacy.in legacy.generated.in
```

Both commands exit 0. `cmp` prints nothing when the files are identical. Legacy
mode preserves comments, whitespace, and source bytes. Unsupported commands
remain accepted, with Unknown findings where their behavior is not verified.

## Put a physical unit in the source

The prepared `units.lamlet` contains:

```lamlet
@let dt = 2 fs
units metal
timestep @{dt}
@for steps in [0, 100] {
	run @{steps}
}
```

Compile it:

```sh
"$LAMLET_DEMO_BIN" build units.lamlet -o units.generated.in --strict --format json
cat units.generated.in
```

The emitted commands are:

```lammps
units metal
timestep 0.002
    run 0
    run 100
```

Metal units use picoseconds for time, so `2 fs` becomes `0.002`. The loop is
expanded at compile time. This short file demonstrates compilation; it does not
construct an atom system to run.

## Inspect a rejected edit

`wrong-unit.lamlet` places `300 K` in the timestep operand. Check it:

```sh
"$LAMLET_DEMO_BIN" check wrong-unit.lamlet --format json
```

Expect exit code 1 and a diagnostic with code `E_DIMENSION_MISMATCH`, expected
`Time`, and actual `Temperature`. The JSON includes the source span. This is an
intentional error. Repeating the command produces identical JSON. A rejected
build leaves an existing output untouched.

## Build one task from explicit inputs

Open `workflow.lamlet`. Its module establishes the pair model under a checked
BoxDefined precondition. Its `sample` stage takes an InputFile, an OutputFile,
and a Temperature. It reads the structure with `nocoeff`, calls the module, and
owns an NVT fix and trajectory dump.

List its stage signature:

```sh
"$LAMLET_DEMO_BIN" inspect stages workflow.lamlet --format json
```

Build the 85 K instance:

```sh
"$LAMLET_DEMO_BIN" build workflow.lamlet --stage sample --instance cold \
	--arg 'structure=path("structure.data")' \
	--arg 'trajectory=output("trajectory.lammpstrj")' \
	--arg 'temperature=85 K' --strict --format json
```

Expect exit 0. Open `.lamlet/build/stages/cold/simulation.in`. It references the
input as `../../../../structure.data`, emits the converted timestep and damping,
and ends with:

```lammps
undump traj
unfix thermostat
```

The adjacent `simulation.in.manifest.json` records the bound arguments, declared
files, and working directory. Input paths resolve from the project root. Stage
outputs belong to the selected instance directory.

## Change a condition without sharing the output

Build another instance:

```sh
"$LAMLET_DEMO_BIN" build workflow.lamlet --stage sample --instance warm \
	--arg 'structure=path("structure.data")' \
	--arg 'trajectory=output("trajectory.lammpstrj")' \
	--arg 'temperature=110 K' --strict --format json
```

The second script appears under `.lamlet/build/stages/warm/`. The cold script
remains unchanged. Inspect the warm task's file dependencies:

```sh
"$LAMLET_DEMO_BIN" inspect dependencies workflow.lamlet --stage sample --instance warm \
	--arg 'structure=path("structure.data")' \
	--arg 'trajectory=output("trajectory.lammpstrj")' \
	--arg 'temperature=110 K' --strict --format json
```

The output path is `.lamlet/build/stages/warm/trajectory.lammpstrj`. It need not
exist yet. A build emits the script and manifest, not a trajectory.

## Check the state boundary

The prepared `missing-state.lamlet` declares a stage containing only `run 100`.
Check it:

```sh
"$LAMLET_DEMO_BIN" check missing-state.lamlet --stage production --format json
```

Expect exit 1 with `E_STATE_CONTRACT`. The stage has not established its atoms or
model. A selected stage begins with empty simulation state, and unsupported
effects are rejected within its boundary. Its guarantee applies to the
documented effect subset, not every LAMMPS command or all input-file contents.

## Continue with a real calculation

Review the generated script before using `lamlet run`. Stage execution requires
an explicit `--lammps-bin` or `[lammps].executable` and launches a fresh process
in the instance directory. Saved target checks are opt-in. The
[target guide](capabilities.md) explains how to inspect the chosen binary.

This tutorial does not run MD. Its fixture is not a validated materials model.
Checkpoint restoration, workflow DAGs, and full provenance remain unimplemented.
See [module and stage contracts](modules-and-stages.md) for the exact guarantees,
[resources](resources.md) for project configuration, and the
[CLI reference](reference.md) for commands and exit codes.
