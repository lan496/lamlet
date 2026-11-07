# Lamlet

Lamlet v0.1 adds compile-time bindings, loops, physical units, typed file resources,
scoped fix/compute/dump management, and explicit target checks to LAMMPS input
scripts. It emits ordinary LAMMPS scripts. Checking and compilation require no
LAMMPS installation, Python or network access. Explicit typed input paths require their files to exist.

Legacy files remain unchanged. Commands outside Lamlet's partial schemas are
accepted and reported as unverified. A successful check means no errors were
detected, not that a simulation is physically meaningful or will run.

## Build and use

Install a stable Rust toolchain, then build the CLI:

```sh
cargo build --release --locked
./target/release/lamlet check examples/legacy.in
./target/release/lamlet build examples/legacy.in -o target/legacy.in
./target/release/lamlet build examples/typed.lamlet -o target/typed.in
```

The legacy output is byte-for-byte identical. The typed example emits
`timestep 0.002` from `2 fs` in metal units and unrolls two zero-step runs.
Both examples are standalone LAMMPS scripts after compilation. `check` and
`build` never invoke LAMMPS. `run` explicitly builds and launches the engine.

```lamlet
@let dt = 2 fs
units metal
timestep @{dt}
@for steps in [0, 100] {
    run @{steps}
}
```

For temperature and damping operand checking, see
[thermostat.lamlet](examples/thermostat.lamlet). It assumes `structure.data`
exists when LAMMPS runs. Lamlet does not open it.

## Use with an agent

```sh
./target/release/lamlet check examples/typed.lamlet --format json
./target/release/lamlet check examples/typed.lamlet --strict --format json
./target/release/lamlet schema --format json
./target/release/lamlet schema fix/nvt --format json
./target/release/lamlet build examples/typed.lamlet -o target/typed.in --source-map target/typed.map.json --format json
```

JSON output contains exactly one object. Exit codes are 0 for no detected
errors, 1 for validation errors, 2 for CLI or filesystem errors, and 3 for engine failures. Warnings and
unknown findings do not fail a compatible check. Strict mode requires schema
verification or an explicit `@unchecked{EXPR}` for newly interpolated operands.

Opaque commands weaken state knowledge. This includes ordinary commands whose
effects Lamlet has not modeled. Physical interpolation requires a known `metal`
or `real` unit style. See the [reference](docs/reference.md) and
[design decisions and limitations](docs/design.md) before using typed constructs
in a large legacy script.

## Resources and scopes

Use `path("...")` / `output("...")` through `@{...}` for checked file resources.
`lamlet inspect dependencies INPUT --format json` lists explicit dependencies.
`lamlet build INPUT` writes the configured build directory and an execution
manifest; `lamlet run INPUT` manages cwd and captures engine output. See the
[project guide](docs/resources.md) and [working example](examples/project/simulation.lamlet).

Use `@scope { ... }` with `@fix`, `@compute`, and `@dump` declarations for reverse
cleanup at block exit. `@drop ID` releases a resource early. Ordinary LAMMPS
resource commands retain manual lifetime management. See [scoped resources](docs/scopes.md)
for ownership, dependency, and control-flow restrictions.

Physical rules are independent of types and ownership. The initial temperature
consistency rule reports conditional warnings with evidence and suggested actions;
Unknown stays nonfatal. It never changes DOF or temperature definitions. See
[physical semantics](docs/semantics.md).

For a selected engine, `lamlet target inspect --format json` records compiled
packages/styles. Enable `[validation].check_capabilities` to check against that
saved profile without launching LAMMPS during compilation. Agents can query
`target list`, `target check`, `target explain`, and `target requirements`.
See [explicit targets](docs/capabilities.md) for configuration and plugin limits.

Modules, independently executable stages, and checkpoint restoration are the
next design layer. Their [contracts and open decisions](docs/modules-and-stages.md)
are documented; their proposed syntax is not implemented.

## Verify changes

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The default suite uses no external engine. An optional ignored test compiles
both standalone examples and runs LAMMPS for zero steps:

```sh
cargo test -p lamlet-cli --test cli optional_lammps_zero_step_integration -- --ignored --exact
```

Set `LAMLET_LAMMPS_BIN` to choose a binary other than `lmp`. The optional test
requires a working local LAMMPS and MPI installation.

## Evaluate upstream compatibility

The optional corpus evaluator fetches pinned input scripts from LAMMPS and
LAMMPS Tutorials, measures byte preservation and static rejection separately,
and writes deterministic JSON with command frequencies and syntax inventories:

```sh
cargo build --release --locked
python3 tools/corpus.py fetch
python3 tools/corpus.py audit --inventory target/corpus/inventory.json
```

This development tool requires Git and Python 3.9+. Fetching requires network
access; it does not run simulations. The selected 893 official examples, 29
benchmarks, and 76 tutorial inputs all round-trip exactly and have no standard
or strict rejections at the pinned revisions. See the
[evaluation methods and results](docs/corpus.md) for exclusions, uncovered
negative cases, and reproduction details. The [baseline JSON](tests/corpus/baseline.json)
records the measured results; large local corpora and inventories stay ignored.

The original normative requirements are in [LAMLET_SPEC.md](LAMLET_SPEC.md).
The guides document subsequent requested extensions and their current boundaries.
