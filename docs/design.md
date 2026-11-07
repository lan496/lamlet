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

## Milestone status

Milestones A and B implement identity compilation, logical lines, spans,
diagnostics, atomic CLI output, typed expressions, lexical loops, and unit
conversion. Command operand validation and conservative state analysis remain
for milestone C. CLI capability descriptions, source maps, and final
documentation remain for milestone D.
