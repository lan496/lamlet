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

## Milestone status

Milestone A implements identity compilation, logical lines, spans, diagnostics,
and atomic CLI output. Typed syntax, semantic validation, capability descriptions,
and source maps remain for milestones B through D.
