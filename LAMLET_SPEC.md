# Lamlet — Language Specification & Implementation Plan

**Status:** Draft for initial implementation (v0.1)
**Implementation language:** Rust (stable toolchain)
**Backend:** standard LAMMPS input scripts
**Primary audience:** scientists, maintainers of legacy LAMMPS scripts, and AI coding agents

> **Lamlet is a statically checked, unit-aware, incrementally adoptable extension of the LAMMPS input language.** Its compiler emits ordinary LAMMPS scripts; LAMMPS remains the execution engine.

## 1. Goals and non-goals

### Goals (ordered)

1. **Legacy first.** Accept existing LAMMPS input files without requiring a rewrite. Never silently rewrite an unmodified legacy command.
2. **Incremental adoption.** A file can mix existing LAMMPS commands and opt-in Lamlet constructs. No wholesale translation to Rust/Python or another host language.
3. **Early, trustworthy validation.** Detect type, dimensional, syntax, and selected command-order errors before invoking LAMMPS. Differentiate errors proven by analysis from properties that cannot be determined statically.
4. **Physical units.** Support explicit units and dimension checking; convert typed quantities into the active LAMMPS unit style during compilation.
5. **Agent-friendly interface.** Deterministic CLI, stable structured diagnostics, machine-readable capability descriptions, and source locations that refer to the original file.
6. **Minimal runtime assumptions.** `check` and `build` work without an installed LAMMPS executable, Python environment, network access, or simulation data files.
7. **Open and maintainable design.** Rust workspace, a testable semantic core, and declarative command schemas that can expand independently of the parser.

### Non-goals for v0.1

- Reimplementing the LAMMPS engine, running simulations, or replacing its Python/C++ APIs.
- Parsing or type-checking every `pair_style`, `fix`, `compute`, plugin, potential file, or LAMMPS expression.
- Guaranteeing physical correctness or numerical stability of MD simulations.
- Static interpretation of general legacy `if`/`jump`/`next` control flow or runtime `${...}`/`$(...)` substitutions.
- General-purpose functions, arbitrary runtime loops, procedural macros, a package manager, LSP, automatic code migration, and SMT-based theorem proving.
- Reformatting or normalizing legacy scripts.

**Design principle:** sound claims over broad guesses. An unknown result is not a validation success; it is also not a compilation error unless the user requests a stricter level of assurance.

## 2. File formats and interoperability

- `.in`, `.lmp`, and other explicitly passed legacy input files are **legacy mode** by default. `lamlet check file.in` analyzes them. `lamlet build file.in -o output.in` copies the original bytes unchanged.
- `.lamlet` is **extended mode**. It accepts all ordinary LAMMPS command lines and introduces opt-in directives and interpolation.
- A `.lamlet` file with no Lamlet syntax or reserved interpolation delimiters must produce byte-for-byte identical output (including whitespace, comments, newline style, and trailing newline).
- A legacy file passed explicitly with `--language lamlet` enables extended mode regardless of suffix. `--language lammps` forces legacy mode.
- The generated `.in` script must contain **no Lamlet directives** and be suitable for an ordinary LAMMPS installation. Never rewrite `${name}`, `$x`, or `$(...)` LAMMPS expansions.
- Lamlet syntax uses a separate namespace (`@let`, `@for`, `@{...}`) to minimize collisions. If a legacy script contains these forms literally, use legacy mode rather than implicitly interpreting them as Lamlet syntax.
- Output and diagnostics must be deterministic for the same input and compiler version.
- Source text is UTF-8 for Lamlet directives. For legacy pass-through, retain original bytes; avoid unintended ASCII/Unicode normalization.

### Compatibility contract

**Strictly guaranteed:** legacy `build` is byte-for-byte identity; legacy `check` does not mutate the input; unknown commands remain accepted.

**Extended mode:** every ordinary LAMMPS command that contains no Lamlet-reserved syntax is preserved verbatim and in order. Expansion may change line count but not the relative order of commands. Unsupported LAMMPS syntax is treated as opaque, not reinterpreted.

Do **not** claim that every old input can be exhaustively validated or that any arbitrary legacy script automatically becomes a well-typed Lamlet program.

## 3. Language surface syntax (v0.1)

### 3.1 Examples

The example below is illustrative; the data file is assumed to exist at LAMMPS execution time.

```lamlet
@let dt = 2 fs
@let tdamp = 100 fs
@let steps = 10000

units metal
atom_style atomic
read_data structure.data

timestep @{dt}

@for target in [300 K, 400 K, 500 K] {
    fix thermostat all nvt temp @{target} @{target} @{tdamp}
    run @{steps}
    unfix thermostat
}
```

Its output has `timestep 0.002`, `fix ... temp 300 300 0.1`, etc. Since the `@for` loop is compile-time, the output contains three sequential `fix` / `run` / `unfix` blocks.

### 3.2 Directives

- `@let NAME = EXPR` — immutable, compile-time lexical binding; no forward references.
- `@for NAME in [EXPR, EXPR, ...] {` — compile-time loop over a finite list. Body ends with a standalone `}`. Nested `@for` blocks are supported.
- `@{EXPR}` — compile-time interpolation inside a *single* legacy command argument. For v0.1, expression interpolation must occupy a complete unquoted argument; it cannot synthesize multiple arguments, command names, line endings, or comments.
- `@unchecked{EXPR}` — explicit single-argument interpolation that opts out of operand-schema validation; its use is always diagnosed. This is an escape hatch, not an ordinary alternative to `@{EXPR}`.
- Directives are recognized only when appearing as the first non-whitespace content of a logical LAMMPS line, outside quoted/triple-quoted content. The closing `}` is recognized only when inside a Lamlet block.
- `#` trailing comments on directive lines are allowed. Ordinary LAMMPS comments are preserved exactly.
- All bindings are lexically scoped. Redefining a name in the same scope is an error; nested scopes may shadow. Loop variables and bindings in their bodies do not escape their loop scope.
- `@for` lists must be known at compile time and homogeneous in type. Zero-length lists are permitted. Expansion has a configurable hard resource limit; default max **100,000 lines generated by expansion**, with a clear error on overflow. The limit must not prevent byte-for-byte identity builds of large legacy files.
- Lamlet directives do not emit LAMMPS `variable` commands and do not modify LAMMPS's runtime variable namespace.

### 3.3 Expression grammar

MVP supported expressions:

- integer literals (`0`, `100`, `-1`), finite floating literals (`0.25`, `1e-3`), and Boolean literals (`true`, `false`);
- physical quantities as `NUMBER UNIT`, e.g. `2 fs`, `0.1 ps`, `300 K`, `1 bar`;
- identifiers, parentheses, unary `-`, binary `+`, `-`, `*`, `/`, with usual precedence;
- homogeneous list literals `[EXPR, ...]` only as `@for` iterables for v0.1.

No strings, arbitrary functions, runtime LAMMPS values, comparisons, interpolation inside quotes, or user-defined operators in v0.1. Reject unsupported Lamlet expressions with explicit diagnostics; never silently forward malformed `@` syntax.

The implementation may use a Pratt parser for expressions. Store source spans for every node.

### 3.4 Parsing ordinary LAMMPS lines

LAMMPS parses input incrementally and has its own handling of `&` continuations, comments, `$` substitutions, quoting, and triple-quoted strings. These are **not** a conventional shell grammar.

- Preserve full raw command text and its physical source span(s) in a lossless representation.
- Correctly identify *logical command boundaries* across `&` and triple quotes. A directive-looking line inside such a command must remain plain LAMMPS text.
- Do not modify LAMMPS quoting, whitespace, `$` forms, or comments.
- For validation, tokenize only commands for which a schema is implemented and only when token boundaries can be determined without ambiguity.
- If a legacy command's argument structure cannot safely be determined, preserve it as opaque and return `unknown`, never invent an argument list.
- Interpolate only within an unambiguous, complete argument token and outside comments/quotes. The `@{...}` syntax may be surrounded by whitespace but not embedded into another token in v0.1.
- If any directive or interpolation is syntactically malformed, fail with a span-aware diagnostic.

**Important:** The official LAMMPS parsing rules are the normative reference, not shell parsing rules: https://docs.lammps.org/Commands_parse.html

## 4. Static type system

### 4.1 Types

Use a compiler-internal algebraic type system (do **not** attempt to encode the input program's typestate in Rust generics):

```text
Type = Int | Float | Bool | Quantity(Dimension) | List(Type) | Unknown
```

- `Int` converts to `Float` in numeric contexts, but not vice versa without an explicit operation.
- Numerical quantities and bare numerical values are distinguished. A bare `100` in an ordinary LAMMPS command follows LAMMPS's implicit active-unit convention. A typed `100 fs` carries an explicit `Time` dimension.
- `Quantity(D)` supports addition/subtraction only when dimensions match. Multiplication/division compose dimensions. Division by a statically known zero is a compile error.
- `Bool` does not implicitly convert to an integer.
- Reject nonfinite compile-time results (`NaN`, infinities) and overflows; do not emit invalid numeric literals.
- A target command schema can require types such as `NonNegativeInt`, `PositiveTime`, or `Temperature`. These are refinements checked when the value is known, not unrestricted theorem proving.
- Distinguish errors **proven** by known expressions (`timestep @{300 K}` is a type error) from unknown runtime values (`timestep ${dt}` cannot be validated statically).

### 4.2 Dimensions and unit values

Represent physical dimensions separately from the active LAMMPS unit style. Prefer a dimension-exponent vector and a normalized numerical value rather than fixed enums scattered across the parser.

Initial named dimensions: `Time`, `Length`, `Temperature`, `Pressure`, and `Energy`, plus dimensionless derived results. The internal design must be extensible to mass, charge, force, and other dimensions without redesigning AST/IR.

**v0.1 explicit unit symbols**:

- Time: `fs`, `ps`, `ns`, `s`.
- Length: `angstrom`, `nm`, `m`.
- Temperature: `K`.
- Pressure: `Pa`, `bar`, `atm`.
- Energy: `eV`, `kcal/mol` (treated as an energy-per-particle convention consistent with LAMMPS `real` units, using Avogadro's constant in conversion).

Do not support Celsius with an additive offset in the initial implementation. Conversion constants should be centralized, documented, and covered by tests.

### 4.3 Target unit styles

- Supported for typed compilation in v0.1: `units metal` and `units real`.
- For `metal`, LAMMPS expects `ps`, Å, `K`, `bar`, `eV` for the above dimensions.
- For `real`, LAMMPS expects `fs`, Å, `K`, `atm`, `kcal/mol`.
- Other valid LAMMPS unit styles (`lj`, `si`, `cgs`, `electron`, `micro`, `nano`) are **accepted unchanged in legacy commands**, but are not available as targets for typed physical-quantity interpolation in v0.1. Produce a targeted unsupported-unit diagnostic if conversion is required.
- `units lj` is *reduced* units; do not infer physical conversion constants or coerce `300 K` to a reduced LJ temperature.
- Before any explicit `units`, LAMMPS defaults to `lj`. Thus interpolating a dimensionful Lamlet value before a supported `units` declaration is an error.
- If dynamic script constructs make the active unit style unknown, fail only when typed interpolation requires that information.

Example conversions:

```text
units metal; timestep @{2 fs}                      -> timestep 0.002
units real;  timestep @{0.1 ps}                    -> timestep 100
units metal; fix t all nvt temp @{300 K} @{300 K} @{100 fs}
             -> fix t all nvt temp 300 300 0.1
```

When formatting numbers, use a deterministic, round-trip-safe representation. Define a numerical tolerance for conversion tests and do not promise bitwise-identical floating-point trajectories to an alternative hand-written unit representation.

### 4.4 Validation at interpolation sites

- For schema-covered operands, compare the expression's inferred type and dimension with the required type; a mismatch is a hard error.
- For a bare number supplied to a legacy numeric operand, keep LAMMPS's implicit unit behavior; do not insist on adding annotations to existing scripts.
- For typed quantities supplied to an opaque/unsupported command, conversion can be performed from the *known current* LAMMPS unit style, but no operand-dimension verification is possible. Emit an `UNVERIFIED_ARGUMENT` warning and mark the result as unverified. In `--strict` mode, require an explicit `@unchecked{EXPR}` escape hatch for this case, which must remain visible as a warning in diagnostics.
- In standard mode, uncertain legacy arguments are accepted, not treated as known-correct. In strict mode, strictness applies to *newly introduced typed constructs* and proof obligations in supported schemas; it must not make all legacy plugin commands unusable.
- `@unchecked{EXPR}` is the only escape hatch in v0.1. It occupies exactly one argument, converts known quantities using the active target unit style, and bypasses the operand-schema type check, recording an `UNCHECKED_INTERPOLATION` warning. It is invalid without a known conversion if `EXPR` is dimensionful.

## 5. Command schemas and semantic analysis

### 5.1 Declarative command signatures

Make a registry of schemas separate from lexical/parser logic. A schema may express:

- argument count / syntax patterns;
- typed argument positions (including style-specific keyword arguments);
- validation refinements;
- prerequisites and known state effects;
- applicability to certain LAMMPS versions (metadata, future extension);
- whether a property is verified, unsupported, or unknown.

Initial schema targets:

| Command | v0.1 checks |
| --- | --- |
| `units` | recognize unit style, track current style, detect definitely invalid ordering when a box is already known to exist |
| `atom_style` | recognize and track raw style, detect definite post-box misuse |
| `read_data`, `read_restart`, `create_box` | record box-exists state when unambiguous; do not inspect files |
| `group` | record clearly declared IDs when syntax is statically unambiguous; `all` is predefined |
| `fix`, `unfix` | track clearly defined/removed IDs, group references where certain; recognize style `nve`, and typed keywords `temp` for `nvt`/`npt`, pressure keywords `iso` for `npt`, without pretending to cover every style |
| `compute`, `uncompute` | track clearly defined/removed IDs only when statically unambiguous |
| `timestep` | expect positive `Time` for typed input, positive scalar for known literal input |
| `run` | first argument is a nonnegative integer count when determinable (zero allowed) |

For `fix nvt/npt` in covered forms, `temp Tstart Tstop Tdamp` is `(Temperature, Temperature, PositiveTime)`; for `fix npt iso Pstart Pstop Pdamp`, `(Pressure, Pressure, PositiveTime)`. Other optional fix keywords are left opaque unless individually described by a schema. Do not reject a syntactically valid fix solely because the schema covers only some keywords.

A LAMMPS command may be *known by name* but *not fully understood*. A command schema must never claim comprehensive validation when it only checks a subset.

### 5.2 Abstract state and certainty

Track a compact abstract state (at minimum: `units`, `box_exists`, `atom_style`, known group IDs, fix IDs, compute IDs). For state facts, distinguish `known present`, `known absent`, and `unknown`; for unit styles, track a known style or `unknown`.

LAMMPS commands are executed sequentially, so validation is order-sensitive. `read_data` can establish box existence, but opaque commands, dynamic arguments, `include`, `if`, `jump`, and plugin commands can prevent definite analysis. In those cases, weaken affected facts to `unknown` instead of inferring that a previous state still holds.

- Only emit a hard semantic error for a violation that is definitely established by the available facts.
- Emit a warning or `unknown` finding for possible violations that cannot be proven.
- A known command after an opaque command may still be checked for local type errors, even when global state knowledge has been invalidated.
- In v0.1, do not attempt full CFG, arbitrary loops, branch joins, or fixed-point abstract interpretation for legacy runtime control flow. Treat such constructs as barriers; document precisely which state facts are invalidated.
- Lamlet `@for` is statically unrolled, so emitted command sequences *can* undergo sequential abstract-state analysis.

**No false certainty:** neither `check` nor `--strict` may claim to prove that a generated MD run is physically meaningful, or that an external data/potential file exists and is compatible.

## 6. Compiler architecture

Pipeline:

```text
source bytes
  -> lossless lexer / logical LAMMPS line recognizer
  -> CST + extension AST (opaque legacy commands and Lamlet nodes)
  -> name resolution + expression type inference
  -> typed intermediate representation (HIR)
  -> compile-time lowering (@let/@for/interpolation)
  -> schema validation + abstract state tracking
  -> ordinary LAMMPS script + source mapping + diagnostics
```

Use explicit, source-span-carrying AST/HIR types. Preserve opaque text as borrowed slices or exact source ranges wherever feasible. Rust's type system protects the implementation; the source language's types are represented as values in the compiler.

Suggested workspace:

```text
lamlet/
  Cargo.toml
  crates/
    lamlet-syntax/      # lossless source handling, LAMMPS line recognition, extension parser
    lamlet-hir/         # expressions, types, spans, lowered nodes
    lamlet-semantics/   # units, inference, command registry, state analysis
    lamlet-codegen/     # source-preserving output, source maps
    lamlet-cli/         # CLI, diagnostics, capability descriptions
  tests/
    fixtures/
    golden/
  examples/
  docs/
```

Recommended Rust crates (not mandatory): `chumsky` for Lamlet expression parsing, `rowan` for a lossless tree if helpful, `logos` where lexing benefits from it, `serde`/`serde_json` for machine-readable output, `clap` for CLI, `miette` or `ariadne` for human diagnostics, `insta` and `proptest` for tests. **Do not overengineer the CST or create needless abstraction crates to satisfy the directory diagram.** Favor correctness and source preservation over framework complexity.

## 7. Command-line and agent interface

### 7.1 Commands

```bash
lamlet check path/to/old.in
lamlet check simulation.lamlet
lamlet check simulation.lamlet --strict
lamlet check simulation.lamlet --format json

lamlet build simulation.lamlet -o generated.in
lamlet build simulation.lamlet -o generated.in --source-map generated.in.map.json
lamlet build old.in -o untouched.in

lamlet schema --format json
lamlet schema fix/nvt --format json
```

- `check` never runs LAMMPS or writes compiled scripts. It performs the same semantic analysis as `build`.
- `build` must fail without writing a partial output when it encounters a hard error. Prefer atomic output replacement.
- `check --format json` writes exactly one JSON object to stdout; human/progress output goes to stderr (or is suppressed). No ANSI escapes in JSON.
- `schema` describes supported unit styles, commands, argument types, diagnostic codes, and **coverage level** for AI agents. Unknown commands must not be represented as unsupported by LAMMPS itself—only as *not yet verified by Lamlet*.
- Do not run arbitrary user code, fetch network resources, or invoke a simulation during `check`/`build`/`schema`.

### 7.2 Machine-readable diagnostics

The JSON envelope is versioned (`schema_version: 1`) with stable error codes. Example:

```json
{
  "schema_version": 1,
  "status": "error",
  "input": "simulation.lamlet",
  "diagnostics": [
    {
      "severity": "error",
      "code": "E_DIMENSION_MISMATCH",
      "message": "timestep expects Time, but this expression has Temperature",
      "file": "simulation.lamlet",
      "span": { "start_byte": 75, "end_byte": 79, "start_line": 6, "start_column": 10, "end_line": 6, "end_column": 14 },
      "expected": "Time",
      "actual": "Temperature",
      "notes": [],
      "suggestion": "Use a time quantity such as 2 fs"
    }
  ],
  "summary": { "errors": 1, "warnings": 0, "unknowns": 0 }
}
```

Rules:

- Line/column indexes are 1-based; byte offsets are 0-based half-open ranges; document whether columns count Unicode scalar values or bytes (prefer Unicode scalars).
- Diagnostics point to the *original Lamlet source*, even for commands generated by `@for`. Multiple instances may reference the same original span, with optional loop-expansion context.
- Stable diagnostic code prefixes: `E_` error, `W_` warning, `U_` unknown/unverified.
- `status` is `ok`, `warning`, or `error`. A script can be `ok` only in the sense of *no detected errors*, never a proof of full LAMMPS runtime correctness.
- Exit status: `0` if no errors (warnings/unknowns permitted in compatible mode), `1` for validation/build errors, `2` for CLI/IO errors.
- Schema and diagnostics can evolve by adding fields without changing the major JSON schema version; do not change or remove existing field meanings silently.

### 7.3 Source maps

An emitted line should map back to one source span (or a small span set) and carry optional expansion context. JSON source map includes format version, original source path, output path, and generated line-to-source references. `--source-map` is optional. Generated text must not embed hidden sentinel comments solely to recover mapping.

## 8. Tests and acceptance criteria

Mandatory automated tests, with no external LAMMPS installation required:

1. **Identity golden tests:** `.in` and unextended `.lamlet` round-trip exactly, including blank lines, indentation, comments, CRLF, trailing newline, `$` references, `&` continuations, single/double quotes, and triple quotes.
2. **Directive shielding:** `@let` and `@{...}` occurring in legacy comments or quoted/triple-quoted content are never executed as directives/interpolations.
3. **Basic compile:** `@let`, nested `@for`, lexical scope, shadowing, zero-length loops, valid interpolation; stable output and original order.
4. **Unit tests:** `2 fs -> 0.002` for `metal` timestep; `0.1 ps -> 100` for `real` timestep; `100 fs -> 0.1` for `metal` damping; pressure and energy conversions use tested constants.
5. **Type errors:** `timestep @{300 K}` and `fix ... temp @{2 fs} ...` yield `E_DIMENSION_MISMATCH` on the correct source range; invalid `@let` arithmetic is rejected.
6. **No false rejection:** legacy `${dt}`, opaque plugin commands, unknown fix styles, `include`, `if`, and `jump` are preserved; they may produce unknown-analysis diagnostics, not invented hard errors.
7. **Known-state diagnostics:** definite `atom_style`-after-box and `unfix` of a definitely absent ID are diagnosed; state-uncertain equivalents are warnings/unknown rather than hard errors.
8. **Safety:** wrong/surprising use of unsupported unit styles for typed interpolation fails; `@unchecked{...}` works only with clearly known conversion state and produces a warning; cannot inject multiple commands via interpolation.
9. **Agent JSON:** exact one-object stdout, stable envelope, correct spans, deterministic order, meaningful command schema coverage.
10. **Failure behavior:** source not modified, no partial build output, correct process exit status, clear error on expansion limit.
11. **Build quality:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace` pass.

Optional integration tests, *only if a LAMMPS binary is present*: compile a tiny fixture and use LAMMPS `-skiprun` or a zero-step run to catch incompatibilities. Do not require it in CI for v0.1.

## 9. Work breakdown for Codex

Implement in incremental, testable changes rather than writing everything in one pass.

**Milestone A — Compatibility foundation**
- Rust workspace, CLI `check`/`build`, lossless logical-line recognizer, identity codegen, source spans, diagnostics infrastructure.
- Required gate: golden legacy tests pass; LAMMPS-specific quoting and line continuation covered.

**Milestone B — Typed language core**
- Parse `@let`, `@for`, `@{...}` and expressions; lexical resolution; compile-time unrolling; units/dimension algebra; `metal`/`real` conversion.
- Required gate: unit and type golden tests pass; malformed extension syntax cannot accidentally become raw LAMMPS.

**Milestone C — Static validation**
- Command schema registry, initial typed command operands, abstract state, conservative unknown barriers, compatible/strict modes.
- Required gate: known-invalid inputs are diagnosed; unrecognized legacy commands are still accepted.

**Milestone D — Agent ergonomics and polish**
- JSON diagnostics, `schema` command, optional source maps, examples, CI, reference docs and limitations.
- Required gate: all acceptance tests pass without running LAMMPS.

### Codex execution instructions

- Read this file first and treat **Sections 1–8 as normative**.
- Make reasonable local design choices where details are unspecified; document decisions briefly in `docs/design.md` instead of stalling for minor questions.
- Prioritize correctness, backwards compatibility, and test coverage above adding features beyond v0.1.
- Work through milestones in order. After each milestone, run formatter, linter, and tests, and fix failures before proceeding.
- If scope/time constraints force partial implementation, deliver the smallest **working, tested milestone** and list the missing criteria explicitly. Do not misrepresent partial implementation as v0.1 complete.
- Provide at least one runnable example using a legacy script and one using `.lamlet` with explicit units.

## 10. References (normative for LAMMPS behavior)

- LAMMPS command execution and ordering: https://docs.lammps.org/Commands_input.html
- LAMMPS input parsing rules: https://docs.lammps.org/Commands_parse.html
- LAMMPS unit styles and conversion conventions: https://docs.lammps.org/units.html
- LAMMPS variable model: https://docs.lammps.org/variable.html
- LAMMPS `fix nvt/npt/nph` parameters: https://docs.lammps.org/fix_nh.html
- LAMMPS `atom_style` prerequisites: https://docs.lammps.org/atom_style.html
- LAMMPS fix semantics: https://docs.lammps.org/fix.html

This specification targets the LAMMPS documentation available in **November 2026**. The compiler should record an explicitly declared LAMMPS target version in future releases; do not silently infer the local engine's features in v0.1.

## 11. Subsequent requested design additions

The original milestones above remain the implementation history. Later explicit
user requirements extend v0.1 as documented below; they take precedence where
they change an earlier restriction. Existing LAMMPS syntax remains accepted.

- Paths are typed resources, not strings, in the new typed syntax. Legacy string
  paths remain legal. Project roots, typed path kinds, dependency inspection,
  portable build manifests, and managed run directories follow
  [the resource contract](docs/resources.md).
- Managed resources use `@scope`, `@fix`/`@compute`/`@dump`, and `@drop`, replacing
  the earlier `@with` proposal. Ordinary resource commands retain their LAMMPS
  behavior. Ownership and control-flow restrictions follow
  [the scope contract](docs/scopes.md).
- Physical semantics is separate from type checking. Rules carry evidence and
  applicability metadata, use Satisfied/Violated/Unknown outcomes, and never
  silently change the physical model. Current coverage is defined in
  [physical semantics](docs/semantics.md).
- Explicit target profiles are part of v0.1. This supersedes the deferral in
  Section 10: features are inspected only by an explicit target operation, then
  checked from a saved profile. Schema coverage and runtime availability are
  distinct. See [the capability contract](docs/capabilities.md).

The next language-design priorities are compatibility; typed inputs, outputs,
units and paths; modules with static effects; independently executable stages;
local scoped resources; checkpoint/provenance; and physical semantic rules.

Every Lamlet stage must be independently executable from explicitly declared
inputs, without relying on hidden state left by previous stages.

Modules reuse stateful code under preconditions and effects. Stages define
independent execution tasks. Reentrancy and checkpoint-based resumability are
separate guarantees; neither promises bitwise identical trajectories. The
[module and stage design](docs/modules-and-stages.md) specifies the intended
boundaries and records unresolved invocation/restoration contracts. The proposed
Same-source module calls and state contracts are implemented as documented there;
stage/checkpoint syntax and commands are not implemented yet.

### 11.1 Programming-model boundaries

Lamlet separates legacy LAMMPS acceptance, typed semantics, stage/artifact
contracts, and the LAMMPS backend. Preserving an opaque legacy command does not
prove that command satisfies a checked module or stage contract. Unknown effects
must remain explicit and cannot establish independent execution.

Compile-time user evaluation remains bounded and deterministic given resolved
inputs and options. Host filesystem resolution and explicit target probing have
separate dependency contracts. Constraint composition applies to facts at one
program point; it does not reorder LAMMPS state transitions. A future task key,
run identity, and output-content identity have different meanings. Effects and
ownership also remain distinct checks.

[Programming model and language precedents](docs/programming-model.md) records
the supporting primary sources and the design decisions informed by Starlark,
Nix, CUE, Modelica, Koka, and Sorbet. It defines planned boundaries, not new syntax
or additional guarantees in the current compiler.
