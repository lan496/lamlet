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

The default expansion limit counts physical lines emitted within loops, including
blank lines and comments. A second work limit of max(1024, 16 times the line
limit) bounds loops that emit no lines. Expressions allow 512 tokens and 128
nested parser calls. Loops allow 64 nested blocks. These limits do not restrict
identity compilation of large legacy files.

## Conservative validation

The schema registry declares argument counts, operands, keyword patterns, state
effects, and partial coverage. The evaluated HIR keeps typed quantities separate
from their formatted output so operand checks cannot confuse implicit legacy
numbers with explicit dimensions. Fix keyword validation follows only the
supported prefix. Once an opaque option appears, later tokens are not assigned
invented keyword boundaries.

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
command's effects apply afterward. The compiler never opens referenced files.

Strict mode requires schema proof for all newly interpolated operands, including
dimensionless ones. `@unchecked` acknowledges an unverified operand and always
warns. It bypasses operand checks, while unit conversion and definite global
state errors still apply. Strict mode accepts opaque legacy commands and runtime
values. Unknown findings do not establish validation success.

## Milestone status

Milestones A through C implement compatibility, typed language constructs, unit
conversion, operand checks, state analysis, strict mode, and the configurable
expansion limit. CLI capability descriptions, source maps, and final
documentation remain for milestone D.
