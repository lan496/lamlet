# Typed resources and projects

Paths are typed resources in new Lamlet syntax. Legacy filename strings retain
LAMMPS semantics and are never checked, rewritten, or resolved through the
compiler's environment.

## Declare and inspect resources

```lamlet
@let structure: InputFile = path("structures/Si.data")
@let potential: PotentialFile = path("potentials/Si.eam.alloy")
@let trajectory = output("results/traj.lammpstrj")
units metal
atom_style atomic
read_data @{structure}
pair_style eam/alloy
pair_coeff * * @{potential} Si
dump traj all custom 100 @{trajectory} id type x y z
```

`path()`, `output()`, `dir()`, `pattern()`, and `dynamic()` each accept one
double-quoted literal. Resources support bindings and homogeneous loop lists;
they do not support arithmetic. General strings and user functions are unavailable.
Use `@{...}` to interpolate a resource, including in `@dump` declarations.
A bare `output("...")` in a command is legacy text, not an expression.

| Type | Constructor / annotation | Compile-time checks |
| --- | --- | --- |
| InputFile | `path("...")` | Existing regular file under input_root |
| PotentialFile | `@let p: PotentialFile = path("...")` | InputFile checks; potential content is not parsed |
| InputScript | `@let s: InputScript = path("...")` | InputFile checks; transitive includes are not compiled |
| OutputFile | `output("...")` | Under output_root; absent or regular file; collisions |
| Directory | `dir("...")` | Existing directory inside the project |
| InputPattern | `pattern("restart.*")` | Fixed parent exists under input_root; matches are not enumerated |
| DynamicPath | `dynamic("structures/${name}.data")` | Fixed prefix inside project; runtime value remains unverified |

PotentialFile and InputScript can be used where InputFile is expected. Annotating
an InputFile binding with either kind refines its declared role, without parsing
its contents. A Directory cannot be supplied to `read_data`.

Input existence is checked even for unused resource bindings. Exact static paths
cannot contain `$`, `*`, or `%`. InputPattern permits LAMMPS `*` / `%` filename
patterns, and OutputFile permits output patterns. Substitutions and patterns are
restricted to the basename. DynamicPath warns; strict mode requires an explicit
`@unchecked{dynamic("...")}` acknowledgement. It cannot guarantee existence,
runtime containment, kind, or collision avoidance.

Names use portable `/` separators. Absolute paths, escapes through `..` or
symlinks, quotes, backslashes, colons, and control characters are rejected.
Spaces, `#`, and braces in static names are preserved through quoted output.
Existing aliases are resolved; Unix hard links are also checked for collisions.
Runtime filesystem changes after checking remain outside the guarantee.

The initial path schemas cover `read_data`, `read_restart`, `include`, `dump`,
`write_data`, `write_restart`, `log`, `shell cd`, and the file operand of
`pair_coeff` under a known `pair_style eam/alloy`. Generic pair styles and
optional filename operands remain unverified. `read_restart` accepts InputPattern.
The registry is available through `lamlet schema --format json`.

Output resources may be absent. Multiple known commands writing the same typed
output are rejected, including repeated expanded loop uses. Input/output aliases
and source aliases are rejected regardless of declaration order. Wildcard overlap
and collisions with legacy or runtime filenames are not inferred.

## Configure the execution root

```toml
[project]
name = "silicon-md"
[paths]
input_root = "."
output_root = "results"
[build]
directory = ".lamlet/build"
```

Every constructor argument is relative to the project root. `input_root` and
`output_root` are allowed boundaries, not prefixes: use
`output("results/traj.lammpstrj")`, not `output("traj.lammpstrj")`, with this
configuration. A Directory may refer to either input or output locations.

The root is the closest ancestor of the source containing `lamlet.toml`; without
one it is the source's directory. Configuration paths stay inside that root.
input_root must exist; output_root and build.directory may be absent. Unknown
configuration keys are errors. Library callers supply `Options.project` through
`Project::load` or `Project::discover`; context-free compilation refuses typed paths.

```sh
lamlet check simulation.lamlet --format json
lamlet inspect dependencies simulation.lamlet --format json
lamlet build simulation.lamlet --format json
lamlet run simulation.lamlet --lammps-bin lmp --format json
```

Check and inspect are read-only. Inspect adds a `dependencies` object containing
sorted inputs, outputs, directories, dynamic names, source uses, and known legacy
filename positions in `unresolved`. `complete` is always false: this is a list of
explicit references, not the transitive dependency closure. Pattern and dynamic
existence is null. Missing explicit input resources fail the check.

## Build and run

Without `-o`, build creates build.directory and emits `<source-stem>.in` plus
`<source-stem>.in.manifest.json`. An explicit `-o` keeps the existing parent-directory
requirement; a typed-resource build also emits the manifest beside that output.
The manifest records dependencies and the project root relative to the generated
script's directory. Run the generated script **from that project root**. Moving
the entire project together preserves this contract. Moving the script alone does not.
The manifest currently contains no content hashes and is not a reproducibility proof.

`run` builds, creates parents of declared output resources, and launches LAMMPS
with the project root as cwd. It uses `--lammps-bin`, then LAMLET_LAMMPS_BIN, then
`lmp`; it does not invoke a shell. Relative executable paths with separators are
relative to the caller. The script receives `-in`, a build-directory `-log`, and
`-screen none`. Engine stdout and stderr go to `.in.stdout` and `.in.stderr` files,
so JSON stdout remains one object. Exit code 3 identifies engine launch/failure;
the JSON execution object records the engine's own exit code when available.

Build artifacts and capture files cannot overwrite the source, typed inputs,
typed outputs, configuration, or each other. Payloads are prepared before
publication; individual replacements are atomic, but publishing several artifacts
is not a filesystem transaction. Check/build never start LAMMPS. Only `run` does.

A literal, resolvable `shell cd` rebases subsequent typed references. After
runtime `shell cd`, include, jump, if, Python, or `run every`, cwd is unknown and
later typed interpolation fails with E_PATH_CONTEXT. No full control-flow or
callback analysis is claimed. Legacy commands continue to pass unchanged.
[LAMMPS shell cd](https://docs.lammps.org/shell.html) affects execution cwd;
[potential lookup](https://docs.lammps.org/pair_coeff.html) also searches
LAMMPS_POTENTIALS. Typed input files resolve directly and do not use that fallback.

Potential UNITS metadata, format parsing, transitive Lamlet includes, dependency
hashes, remote staging, and full runtime path analysis remain unimplemented.
An explicit include whose filename ends in `.lamlet` is rejected to avoid sending
uncompiled extensions to LAMMPS; compile included sources separately.
