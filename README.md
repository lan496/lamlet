# Lamlet

Lamlet v0.1 adds compile-time bindings, loops, and physical units to LAMMPS input
scripts. It emits ordinary LAMMPS scripts. Checking and compilation require no
LAMMPS installation, simulation files, Python, or network access.

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
Both examples are standalone LAMMPS scripts after compilation. Lamlet never
invokes LAMMPS itself.

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
errors, 1 for validation errors, and 2 for CLI or filesystem errors. Warnings and
unknown findings do not fail a compatible check. Strict mode requires schema
verification or an explicit `@unchecked{EXPR}` for newly interpolated operands.

Opaque commands weaken state knowledge. This includes ordinary commands whose
effects Lamlet has not modeled. Physical interpolation requires a known `metal`
or `real` unit style. See the [reference](docs/reference.md) and
[design decisions and limitations](docs/design.md) before using typed constructs
in a large legacy script.

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

The normative requirements are in [LAMLET_SPEC.md](LAMLET_SPEC.md).
