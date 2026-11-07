# Design decisions

## Compatibility foundation

The workspace has a compiler library and a CLI. Separate library modules keep
source handling, parsing, semantic analysis, and output independent without
adding a crate for every compiler stage.

The source owns bytes. Logical commands refer to half-open byte ranges and retain
continuation-marker ranges. Output copies original bytes unless a Lamlet node
requires expansion. Legacy mode accepts non-UTF-8 bytes.

Diagnostic columns count Unicode scalar values, starting at one. Invalid legacy
bytes count as replacement characters for display only. Offsets always refer to
original bytes. CRLF bytes remain unchanged.

The logical-line recognizer follows the official
[LAMMPS parsing rules](https://docs.lammps.org/Commands_parse.html), including
quoted continuations and triple-quoted multiline text. It does not apply shell
escape rules.

## Typed core

Expressions use a Pratt parser and carry original byte spans, including across
LAMMPS continuations. Bindings use lexical scope stacks. Integer arithmetic is
checked signed 64-bit arithmetic. Division produces a floating value. Mixed Int
and Float loop lists promote to Float. An empty loop parses its body but does
not evaluate unreachable bindings.

Dimensions use mass, length, time, and temperature exponents. Internal numerical
bases use femtoseconds, angstroms, kelvin, and electronvolts per particle. This
keeps common time conversions exact. Constants use the exact SI definitions of
the electronvolt and Avogadro constant, and 4184 joules per thermochemical kcal.
The target conventions follow the official [LAMMPS unit definitions](https://docs.lammps.org/units.html).
Pressure has an explicit target scale because LAMMPS pressure units are not
coherent with energy per volume. Other composed dimensions use the target
energy, length, time, and temperature bases.

Formatting uses Rust's shortest round-trip floating representation, with negative
zero emitted as zero. Conversion tests use relative tolerance 1e-12. This does
not promise identical MD trajectories to scripts using other rounded constants.

The default expansion limit counts physical lines emitted within loops and managed acquisition/cleanup lines, including
blank lines and comments. A second work limit of max(1024, 16 times the line
limit) bounds loops that emit no lines. Expressions allow 512 tokens and 128
nested parser calls. Lamlet permits 64 nested loop/scope blocks. These limits do not restrict
identity compilation of large legacy files.

## Conservative validation

The schema registry declares argument counts, operands, keyword patterns, state
effects, and partial coverage. The evaluated HIR keeps typed quantities separate
from their formatted output so operand checks cannot confuse implicit legacy
numbers with explicit dimensions. Fix keyword validation follows only the
supported prefix. Once an opaque option appears, later tokens are not assigned
invented keyword boundaries.

Legacy `timestep` literals may be zero or negative. LAMMPS's pinned upstream
`TimeStep` unit test explicitly accepts both. The later compatibility requirement
takes precedence over the positive-legacy-scalar rule in spec Section 5.1;
`PositiveTime` remains enforced for Lamlet interpolations and covered damping
operands. Literal nonnumeric `timestep` and `run` arguments are errors, while
quoted runtime substitutions remain unknown. The upstream fixture provenance is
recorded in `tests/corpus/negative.json`.

The initial state uses LJ units, no box, atomic atom style, group `all`, no
user fixes, and LAMMPS's three default thermo computes. ID inventories distinguish
present, absent, and unknown, including an unknown fallback after a barrier.
Reusing a fix ID is allowed. Thermostats weaken compute inventory knowledge
because they can create internal computes. The defaults follow
[LAMMPS thermo compute definitions](https://docs.lammps.org/thermo_style.html).

All unregistered commands, including `include`, `if`, `jump`, and `next`, invalidate
units, box existence, atom style, and ID inventories. Ambiguous token boundaries
or unquoted runtime substitutions have the same effect. Group `all` remains
predefined. An unknown fix or compute style invalidates state, then establishes
the explicitly declared ID. Possible opaque callbacks also invalidate state at
subsequent `run` commands. `run` options are opaque and invalidate state.

`read_data` establishes a box but weakens group and fix inventories, since files
and optional fix callbacks are not inspected. `read_restart` establishes a box
and makes all other tracked facts unknown. `create_box` establishes a box.
Later known commands can re-establish individual facts.

This intentionally conservative policy means an ordinary command such as `region`
or `variable` can make unit conversion unavailable until units are explicitly
known again. Legacy output remains unchanged. A typed quantity is converted using
state immediately before its command, including an opaque command, and that
command's effects apply afterward. Legacy paths are not inspected. Explicit typed resources opt into filesystem
checks described in the [project guide](resources.md).

Strict mode requires schema proof for all newly interpolated operands, including
dimensionless ones. `@unchecked` acknowledges an unverified operand and always
warns. It bypasses operand checks, while unit conversion and definite global
state errors still apply. Strict mode accepts opaque legacy commands and runtime
values. Unknown findings do not establish validation success.

## Output and diagnostics

The CLI publishes one versioned JSON object after analysis and output handling.
This also covers CLI and filesystem errors, which return exit code 2. The schema
command exposes the actual declarative registry and a stable diagnostic catalog.
Diagnostics and generated lines follow execution order. Hash iteration does not
affect output.

Source maps point each generated physical line at the whole original logical
command. This avoids inaccurate line offsets when an interpolation consumes a
continuation. Expansion context records each loop variable and zero-based
iteration. Scripts contain no mapping comments.

Builds prepare output and optional map temporary files before publishing either.
Each destination is replaced atomically. The multi-file publication is not a
filesystem transaction: a filesystem error during the final rename can leave
only one destination updated. Compilation errors never publish either file.
Source paths and existing aliases cannot be output destinations.

## Coverage and remaining limitations

Milestones A through D are implemented. The required automated acceptance cases
run without LAMMPS. The original v0.1 requirements have no intentional feature deferrals.
Later project-resource and scope extensions are documented in
[resources](resources.md) and [scopes](scopes.md), including their deferred work. State analysis remains deliberately incomplete as described above;
unsupported commands are accepted, not validated. The compiler currently stops
at the first syntax or expression-evaluation error, while schema analysis can
report multiple errors.

The thermostat example assumes an external structure file and illustrates the
specification's typed syntax. The standalone legacy and typed examples use
`pair_style zero` and zero-step runs and need no external data.

The optional local engine check reached a completed zero-step legacy run using
LAMMPS 22 Jul 2025 Update 4, then failed in MPICH/libfabric finalization on `utun7`.
An unsandboxed retry also failed. A TCP loopback attempt failed during MPI
initialization. This is not counted as a passing integration check. The
automated Rust suite and CI do not require an engine or MPI.

CI is configured for stable Rust on Linux, macOS, and Windows. Only the local
macOS toolchain was exercised during implementation; hosted CI has not been run.
An independent corpus CI job fetches pinned inputs and evaluates preservation
and rejection. Its methods, exclusions, and coverage boundaries are documented
in [the corpus evaluation](corpus.md). The later compatibility correction adds
three Rust tests, bringing the local suite to 43 passing tests and one ignored
engine test.

Each milestone passed `cargo fmt --all`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace`, all with exit code 0. Test counts at the milestone
gates were 4, 14, 23, and 40. The final suite has one ignored optional engine
test. `cargo build --release --locked` also passed. Running the release CLI on
both standalone examples succeeded, and `cmp examples/legacy.in
target/validation/legacy.in` confirmed byte identity. Generated files and local
validation logs stay under ignored `target/validation/`.

## Milestone history

- A established byte-preserving compilation, logical lines, spans, and atomic
  CLI output.
- B added typed expressions, lexical bindings, compile-time loops, and units.
- C added declarative operand schemas, abstract state, and strict mode.
- D added capability queries, complete JSON reporting, source maps, acceptance
  tests, examples, CI, and reference documentation.

## Physical rule layer

The independent semantic engine consumes resolved commands and records known
temperature facts and three-valued rule findings at run sites. It never rewrites
physics. Rule metadata, warning evidence, scope, and unimplemented idioms are
documented in [physical semantics](semantics.md).
