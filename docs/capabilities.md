# Explicit LAMMPS targets

Command schemas describe what Lamlet understands. A target profile describes
which packages and styles an explicitly selected LAMMPS binary provides. These
are separate: an opaque style can be available, and a modeled style can be
unavailable.

## Configure and inspect a target

```toml
[lammps]
executable = "/opt/lammps/bin/lmp"
profile = ".lamlet/target.json"

[lammps.requirements]
version = ">=2025-09-10"
packages = ["MANYBODY", "MOLECULE"]

[validation]
strict = true
check_capabilities = true
```

Version constraints compare calendar dates in `YYYY-MM-DD` form. Supported
operators are `=`, `==`, `>`, `>=`, `<`, and `<=`; comma-separated comparisons
are combined with AND. A bare date means equality. Semver and OR ranges are
rejected. Package names are case sensitive.

```sh
lamlet target inspect --format json
lamlet target list --category pair --format json
lamlet target check --category fix --style nvt --format json
lamlet target explain --category pair --style eam/alloy --format json
lamlet target requirements simulation.lamlet --format json
```

The first four commands accept `--project DIRECTORY`, which finds the closest
ancestor `lamlet.toml`. Without a configuration, that directory is the project
root. `inspect` also accepts `--lammps-bin EXECUTABLE`.

`target inspect` runs the selected executable with `-h` from the project root.
The official [help option](https://docs.lammps.org/Run_options.html) lists
compiled styles. Lamlet requires a successful process exit and recognizable
package and style inventories before atomically saving the profile. A failed
probe leaves any existing profile intact. The probe has a 10-second timeout and
a 16 MiB accepted stdout limit; it captures engine stdout/stderr so agent JSON
remains one object. A successful inventory is saved even if it fails the
project's declared package or version requirements; those failures return 1.

Executable selection is `--lammps-bin`, then `[lammps].executable`, then
`LAMLET_LAMMPS_BIN`, then `lmp` from PATH. Configured relative executable paths
with directory components resolve from the project root; a command-line
override resolves from the caller's directory. Lamlet never builds LAMMPS.

## Check and run

`check_capabilities` defaults to false. When true, `check`, `build`, and `run`
load the saved profile and validate recognized literal style positions and
project requirements. This explicit opt-in also applies to legacy files.
Checking and building never invoke an engine. Missing, malformed, or locally
stale profiles produce `E_TARGET_PROFILE` and exit 2; Lamlet does not silently
replace them with local features. Refresh them with `target inspect` in the
intended execution environment.

The profile records its schema version, LAMMPS version text and date, canonical
executable path, platform, probe method, file size and modification time,
`LAMMPS_PLUGIN_PATH`, sorted packages, and sorted style inventories. Size and
modification time detect ordinary binary changes; they are not a content hash
or a reproducibility guarantee. An offline profile can be checked when its
recorded executable is absent locally. If the configured executable resolves
locally, its identity and plugin-path environment must match the snapshot.
`run` always checks the actual selected executable and plugin environment
against an enabled profile before building or launching it.

Capability-enabled builds include a manifest even with an explicit `-o` path.
The manifest names the profile. Build outputs cannot overwrite that profile or
the project configuration. Target profiles contain environment-specific paths;
they are not portable execution environments or checkpoint manifests.

## Availability and coverage

The JSON availability values are:

| Value | Meaning |
| --- | --- |
| `built_in` | Listed in the compiled inventory |
| `plugin` | Listed in the profile's separately supplied plugin inventory |
| `unavailable` | Absent from a known category with no unresolved plugin source |
| `unknown` | Category, runtime style selection, or plugin effects are unresolved |

The probe collects compiled styles only. An optional `plugins` map in a supplied
profile can hold an independently observed inventory; this release does not
discover or load plugins. [LAMMPS plugins](https://docs.lammps.org/plugin.html)
can add and remove styles, and `LAMMPS_PLUGIN_PATH` can trigger automatic loading.
A nonempty plugin path therefore prevents absence from the compiled list from
proving unavailability. `plugin`, `include`, `jump`, `if`, Python, suffix/package
changes, ambiguous substitutions, and opaque run callbacks weaken subsequent
availability to Unknown. Lamlet does not claim to resolve arbitrary plugin code.

Covered selectors include pair, atom, bonded, kspace, integration, minimization,
fix, compute, dump, and region styles. Pair `hybrid`, `hybrid/overlay`, and
`hybrid/scaled` inspect children only while argument boundaries are known:
`eam`, `eam/alloy`, and numeric `lj/cut` arguments are supported. Bonded `hybrid`
lists are inspected. Other hybrid forms retain unresolved requirements instead
of guessing which argument words are style names. Accelerator suffix resolution
and all command availability are not fully modeled.

`target requirements INPUT` accepts the usual input analysis flags and works
without a profile: it reports Unknown availability and creates no files. If a
profile exists, it uses it, even when ordinary checking has not opted in. The
output includes original source spans, unresolved dynamic requirements, and
project requirements; it is a partial requirements inventory, not proof of a
complete runtime environment. Target queries emit a JSON report in either
format; human mode also renders diagnostics to stderr.

## Diagnostics and metadata

An unavailable recognized style produces `LML-CAP-001`; a failed version
requirement produces `LML-CAP-003`; a missing required package produces
`LML-CAP-004`. They are errors with exit 1. Unknown availability uses
`U_TARGET_CAPABILITY` and remains nonfatal in both modes. Probe errors use
`E_TARGET_PROBE` with exit 2.

The small metadata registry links `eam`, `eam/alloy`, `lj/cut`, `nve`, `nvt`, and
`npt` to official documentation and any existing Lamlet operand schema.
`eam/alloy` requires MANYBODY according to the
[official EAM documentation](https://docs.lammps.org/pair_eam.html); an
unavailable entry can therefore suggest `-D PKG_MANYBODY=yes`. Package metadata
never substitutes for checking the actual style inventory. Unknown metadata is
JSON null. `lamlet schema --format json` publishes this registry.

The [LAMMPS C configuration API](https://docs.lammps.org/Library_config.html)
offers package/style queries. Direct library loading, live plugin inventories,
MPI/GPU configuration capture, content hashes, and cross-version behavior
metadata are unimplemented. The executable probe avoids linking the compiler
to a particular LAMMPS library or MPI runtime.
