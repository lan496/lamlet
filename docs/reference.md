# Lamlet v0.1 reference

## Commands and flags

| Command | Behavior |
| --- | --- |
| `lamlet check INPUT` | Analyze without writing output or invoking LAMMPS |
| `lamlet build INPUT [-o OUTPUT]` | Analyze and emit an ordinary LAMMPS script |
| `lamlet inspect dependencies INPUT` | List explicit resources and unresolved legacy paths |
| `lamlet run INPUT` | Build and explicitly launch LAMMPS from the project root |
| `lamlet schema [COMMAND]` | Describe capabilities and partial schema coverage |
| `lamlet target inspect` | Probe an explicitly selected engine and save its profile |
| `lamlet target list/check/explain` | Query a saved target by category/style |
| `lamlet target requirements INPUT` | Extract partial style requirements, with optional offline profile |

`check`, `build`, `run`, and `inspect dependencies` accept `--language lammps` or `--language lamlet`, `--strict`,
`--max-expanded-lines N`, and `--format human` or `--format json`. The default
line limit is 100000. A limit of zero prohibits lines emitted by loops but
permits identity compilation.

`build` also accepts `--source-map PATH`. Source, output, and map paths must be
distinct. Existing input aliases, including Unix hard links, are protected.
Compilation errors leave existing output and map files untouched. Output uses
temporary files in each destination directory followed by atomic replacement.
Explicit output directories must already exist. Default builds create the
configured build directory. See [projects and typed resources](resources.md) for
configuration, manifests, cwd, and engine execution.

`schema` accepts `--format human` or `--format json`. Queries use names such as
`timestep`, `fix/nvt`, and `fix/npt`. Unregistered queries report `opaque`, which
describes Lamlet's coverage and says nothing about LAMMPS availability.
The [target guide](capabilities.md) describes separate availability queries and
the `[lammps]`, `[lammps.requirements]`, and `[validation]` configuration tables.

The `.lamlet` extension selects extended mode. All other explicitly passed
files default to legacy mode. `--language` overrides the extension. Legacy mode
preserves source bytes, including non-UTF-8 bytes and literal Lamlet delimiters.

## Directives and expressions

`@let NAME = EXPR` declares an immutable lexical binding. `@for NAME in
[EXPR, ...] {` starts a loop whose closing `}` occupies its own logical line.
Both directive headers and closing braces allow trailing `#` comments.
Bindings in the same scope cannot repeat. Nested scopes can shadow outer names.
Loop lists can be empty and must have one type, with Int-to-Float promotion.
Names use ASCII letters, digits, and underscores, starting with a letter or
underscore.

`@{EXPR}` replaces one complete unquoted command argument. It cannot generate
a command name, multiple arguments, comments, or line endings. Quoted and
commented delimiters remain literal. `@unchecked{EXPR}` uses the same syntax and
conversion rules but bypasses operand validation. Every use warns.

Expressions support signed integers, finite floats with optional exponents,
Booleans, quantities, names, parentheses, unary minus, and `+`, `-`, `*`, `/`.
Multiplication and division bind more tightly than addition and subtraction.
Integers use checked 64-bit arithmetic. Division returns Float. Bool is not
numeric. Only loop iterables accept list literals. General strings, user functions, comparisons, runtime LAMMPS expressions, and
other operators are unavailable. The resource constructors `path`, `output`,
`dir`, `pattern`, and `dynamic` accept one quoted filename; see [resources](resources.md).
Resource annotations use `@let NAME: ResourceType = EXPR`.

`@scope`, `@fix`, `@compute`, `@dump`, and `@drop` manage explicitly owned
resources; see [scope semantics](scopes.md).

Quantities require whitespace between the number and unit. Addition and
subtraction require matching dimensions. Multiplication and division compose
dimension exponents. A dimensionless quantity result becomes Float. Known
zero divisors, overflow, and nonfinite results are errors.

## Units

| Dimension | Explicit symbols | `metal` target | `real` target |
| --- | --- | --- | --- |
| Time | `fs`, `ps`, `ns`, `s` | ps | fs |
| Length | `angstrom`, `nm`, `m` | angstrom | angstrom |
| Temperature | `K` | K | K |
| Pressure | `Pa`, `bar`, `atm` | bar | atm |
| Energy per particle | `eV`, `kcal/mol` | eV | kcal/mol |

LAMMPS defaults to `lj`. Physical interpolation before a supported unit
declaration fails. Other legacy unit styles remain accepted. Unknown active
units, including after a state barrier, prevent physical conversion even with
`@unchecked`. Bare numbers retain implicit LAMMPS unit conventions.

## Validation coverage

All schemas are partial. `schema --format json` is the machine-readable registry.
Operand `position` values start at one after the command name. Keyword operand
arrays list the arguments immediately following that keyword.

| Command | Checked facts |
| --- | --- |
| `units`, `atom_style` | Raw style and definite post-box ordering violations |
| `read_data`, `read_restart`, `create_box` | Box existence without inspecting external files |
| `group` | Clearly declared or deleted IDs, with `all` predefined |
| `fix`, `unfix` | Fix IDs, certain group references, recognized `nve` style |
| `fix` with `nvt`, `npt` | `temp`: Temperature, Temperature, PositiveTime |
| `fix` with `npt` | `iso`: Pressure, Pressure, PositiveTime |
| `compute`, `uncompute` | Clearly defined or removed compute IDs |
| `timestep` | Positive Time for Lamlet expressions; legacy zero and negative scalars preserved |
| `run` | First operand is a nonnegative Int, including zero |

Fix keyword checking stops at the first unrecognized option. Other styles and
options remain opaque. Runtime substitutions can change argument counts, so
commands with unquoted `$` substitutions receive unknown analysis. A known local
operand is still checked after an earlier state barrier.

Standard mode warns on an interpolated operand without schema coverage. Strict
mode makes this an error unless it uses `@unchecked`. Legacy plugin commands,
unknown runtime operands, and unknown state findings remain accepted in strict
mode. Explicit typed resources validate file existence/kind and roots. Explicit
target profiles validate covered style positions and declared requirements.
Legacy paths, unresolved runtime capabilities, numerical stability, and general
physical correctness remain unverified.

## Diagnostic protocol

The JSON envelope has `schema_version: 1`, `status`, `input`, `diagnostics`, and
`summary`. Status is `error` when errors exist, `warning` when only warnings or
unknowns exist, and `ok` otherwise. Summary contains `errors`, `warnings`, and
`unknowns`. No detected errors is not proof of complete validation.

Every diagnostic has `severity`, `code`, `message`, `file`, `span`, `expected`,
`actual`, `notes`, and `suggestion`. Optional values are JSON null. Codes use
`E_`, `W_`, and `U_` prefixes in the original protocol. Later physical/capability
rules also use `LML-PHYS-*` and `LML-CAP-*`; consumers must read `severity`.
`schema` publishes each code and its meaning. Physical warnings may include
`evidence` and `suggested_actions`; `physical_findings` records tri-state results.
Diagnostics follow deterministic expansion and analysis order. Repeated loop
instances may point at the same original span.

Spans use zero-based half-open byte offsets, one-based lines, and one-based
Unicode scalar columns. End positions are exclusive. A span ending after a
newline ends on the following line at column one. CRLF remains two source bytes.
CLI and I/O failures use an empty span at the start of the input when no better
location exists. CLI argument errors use an empty input path.

JSON mode writes exactly one object to stdout, without ANSI escapes or progress
messages. Human mode writes diagnostic reports to stderr. Schema descriptions,
help, and version text go to stdout. Exit codes are 0 for no errors, 1 for
validation failure, 2 for CLI, configuration, probe, or I/O failure, and 3 for
an explicitly launched engine failure.

## Source maps

The optional JSON map has `format_version: 1`, `original_source`, `output`, and
`lines`. Every emitted physical line has a `generated_line`, original `source`
span, and `expansion` array. Source spans cover the full original logical command,
which remains accurate when interpolation collapses a continuation. Expansion
entries identify the loop `variable` and zero-based `iteration`, outermost first.
The generated script contains no hidden mapping comments.

## Resource limits

The line limit counts physical lines generated inside loops, including blank
lines and comments, and managed acquisition/cleanup lines. Unexpanded legacy
lines do not consume it. A work
limit of max(1024, 16 times the line limit) bounds empty nested expansions.
Expression limits are 512 tokens and 128 nested parser calls. Loop/scope nesting
is limited to 64 levels. Errors produce no compiled output.
