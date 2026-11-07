# Byte preservation and static rejection in upstream LAMMPS inputs

## Abstract

Lamlet's compatibility with existing input scripts was evaluated independently
of numerical simulation. Two pinned upstream repositories supplied 998 LAMMPS
inputs and input fragments: 893 official examples, 29 benchmarks, and 76 tutorial
files. All build outputs were byte-identical to their inputs, and neither
standard nor strict checking rejected a selected file. Three covered upstream
negative cases produced the expected errors; five cases outside the declared
coverage remained unknown. These results establish preservation and acceptance
for this finite corpus. They do not establish runtime validity or numerical
equivalence for typed programs.

## Introduction

The LAMMPS distribution includes short examples and more complex examples of
methods and packages [1]. This range makes its existing scripts useful for
evaluating whether a gradual language extension preserves source text and avoids
rejecting unfamiliar commands. The tutorial collection supplies a second source
of input styles [3]. The evaluation separates byte preservation, static
rejection, detection of covered errors, and semantic regression so that accepting
opaque commands cannot be mistaken for comprehensive validation.

## Methods

### Corpus selection

The official repository was fixed at
`d291d572cf3c73a8a720774b78482a7c931304ca` [2]. All regular files whose basenames
begin with `in.` under `examples/` and `bench/` were candidates. The tutorial
repository was fixed at `dd3dce70b883af02df293ce52491beabd8e01928` [3]; its
`.lmp`, `.lammps`, and `.inc` files were candidates, including parameter fragments
and exercises. Candidate counts and exclusions are given in Table 1.

Table 1. Corpus selection at the fixed revisions. Exclusions identify files
outside the LAMMPS input language; no rejected LAMMPS input was excluded.

| Source | Candidates | Excluded | Selected |
| --- | ---: | --- | ---: |
| Official `examples/`, `in.*` | 894 | `COUPLE/lammps_spparks/in.spparks`: SPPARKS input | 893 |
| Official `bench/`, `in.*` | 29 | None | 29 |
| Tutorials, `.lmp`/`.lammps`/`.inc` | 77 | `tutorial4/videos/minimization/data.lammps`: atom data | 76 |

The SPPARKS file identifies itself in its header and uses `app_style`,
`create_sites`, and `solve_style`. Treating it as LAMMPS produces a correct
`create_box` argument-count error. Its presence explains why the unfiltered
894-file set does not achieve a compiler round trip. The data file identifies
itself as output from `write_data`. Both exclusions are explicit in
[sources.json](../tests/corpus/sources.json).

Selection uses the pinned Git tree rather than a filesystem glob. Each selected
file must exist and match its Git blob object ID. Missing or changed inputs fail
the evaluation instead of reducing its denominator. Sparse checkout retrieves
inputs and the upstream command tests; simulation data and potential files are
not required. Includes are counted as source constructs and are not followed.

### Evaluation criteria

For each selected input, the actual release CLI runs `check`, `check --strict`,
and `build`, always with `--language lammps --format json`. A round trip succeeds
only if build succeeds and its output bytes equal the original bytes. A rejected
build and an emitted byte mismatch are separate outcomes. Static rejection is
measured independently from the `check` result; its JSON diagnostics must equal
the build diagnostics. CLI or I/O failures invalidate the evaluation and are not
counted as static rejections.

The round-trip rate is the number of identical outputs divided by the selected
file count. Standard and strict rejection counts are reported separately. The
upstream files were not executed to establish which are runnable under a
particular package configuration. Consequently, rejection counts are observed
quantities; a runtime-validated false-positive rate is not estimated.

Eight minimal command fragments are associated with named C++ tests and exact
anchors in the pinned upstream `test_simple_commands.cpp` [4]. The collector
verifies those anchors. Three fragments test covered argument-count or numeric
literal errors. Five test deliberately uncovered behavior. Both checking modes
must detect the covered cases and preserve unknown findings for the others.

Command frequency follows the proposed physical-line rule:
`^[ \t]*([a-zA-Z_][a-zA-Z0-9_]*)`. Counts therefore include lexical candidates
inside continuations and multiline strings; they are not executed-command
counts. Syntax inventories likewise identify lexical markers: command-line
prefixes for `include`, `jump`, `if`, and `next`; dollar substitutions;
end-of-line ampersands; and triple quotes. These inventories support choosing
fixtures and schema priorities without claiming full parsing coverage.

## Results

### Preservation and acceptance

Byte preservation and static acceptance were measured on every selected file.
Table 2 reports all selected inputs, including fragments and exercises. Every
successful output matched its input exactly.

Table 2. Measured compatibility outcomes. Rejections are file counts, and Unknown
is the number of diagnostic findings from standard checking.

| Corpus | Identical outputs / files | Round-trip rate | Standard rejections | Strict rejections | Unknown |
| --- | ---: | ---: | ---: | ---: | ---: |
| Official examples | 893 / 893 | 100% | 0 | 0 | 25,760 |
| Official benchmarks | 29 / 29 | 100% | 0 | 0 | 453 |
| Tutorials | 76 / 76 | 100% | 0 | 0 | 2,248 |

The requested focus directories contribute 3 files from `UNITS`, 1 from
`ELASTIC`, 1 from `MC-LOOP`, and 420 from `PACKAGES`. All are included in Table 2
and meet the same criteria. `UNITS` tests preservation of LJ, real, and metal
scripts here; typed conversion accuracy is tested separately by the Rust suite.

### Command and syntax distribution

The selected official examples contain 28,532 physical-line command candidates.
The ten most frequent names are shown in Table 3. The complete top 40 for each
corpus is retained in [baseline.json](../tests/corpus/baseline.json).

Table 3. Most frequent lexical command candidates in the 893 official LAMMPS
examples, excluding the SPPARKS file. Counts follow the physical-line rule in
Methods and do not imply schema coverage.

| Candidate | Count |
| --- | ---: |
| `variable` | 4,243 |
| `pair_coeff` | 2,412 |
| `fix` | 2,195 |
| `compute` | 1,187 |
| `run` | 1,135 |
| `pair_style` | 874 |
| `thermo` | 859 |
| `units` | 816 |
| `region` | 801 |
| `atom_style` | 788 |

The official examples contain dollar-substitution markers in 415 files,
continuations in 344, `include` in 62, `if` in 26, `jump` and `next` in 14 each,
and triple quotes in 11. These sets overlap. The generated inventory lists the
paths for every marker and the SHA-256 digest and outcome of every input.

### Negative cases

All three covered negative cases were detected in both modes: `timestep` without
an argument, with two arguments, and with the literal `xxx`. Five upstream
negative cases remain outside coverage: a negative `reset_timestep` number, an
unknown command, a noninteger `quit` argument, an incomplete opaque `run` option,
and an unknown unit-style name. They are accepted with Unknown findings rather
than counted as detected errors. This is a coverage boundary, not an overall
negative-corpus detection rate of 100%.

## Discussion

The corpus meets the byte-preservation and zero-rejection targets for its
selected LAMMPS inputs. The large number of Unknown findings shows that this
acceptance does not imply complete type checking. The frequency distribution
supports prioritizing narrow forms of commonly used commands in future work;
no additional command schemas were introduced for this evaluation.

The upstream `TimeStep` test explicitly accepts zero and negative timestep
sizes [4]. Preserving these legacy literals required a compatibility correction
to spec Section 5.1's positive-scalar rule. The later compatibility instruction
takes precedence for legacy input; Lamlet expressions retain the `PositiveTime`
contract. A negative timestep size is distinct from a negative timestep number
in `reset_timestep`, which is outside the current schemas.

Semantic regression was not run. The report records this outcome explicitly,
and no LAMMPS engine was invoked by the evaluator. Includes, external simulation
files, package availability, runtime branches, and numerical trajectories were
not validated. The official regression framework and `lammps-testing` have not
been integrated. Byte equality gives a strong preservation check for unchanged
legacy text, while numerical regression remains necessary for typed lowering and
future transformations.

## Conclusions

Lamlet preserves and accepts the selected upstream input corpus while reporting
unverified behavior separately. Explicit selection provenance prevents foreign
language files and missing downloads from distorting the compatibility result.
The recorded syntax inventories and negative-case boundaries provide a basis for
expanding validation without treating unfamiliar commands as errors.

## References

1. [LAMMPS documentation, Example problems](https://docs.lammps.org/Examples.html).
2. [LAMMPS source at the evaluated revision](https://github.com/lammps/lammps/tree/d291d572cf3c73a8a720774b78482a7c931304ca).
3. [LAMMPS Tutorials input collection at the evaluated revision](https://github.com/lammpstutorials/lammpstutorials-inputs/tree/dd3dce70b883af02df293ce52491beabd8e01928).
4. [LAMMPS simple command tests at the evaluated revision](https://github.com/lammps/lammps/blob/d291d572cf3c73a8a720774b78482a7c931304ca/unittest/commands/test_simple_commands.cpp).

## Appendix: Reproduction and computational details

The evaluator requires Git, Python 3.9 or newer, and the release Lamlet CLI.
Python is a development-tool dependency; the compiler itself has no Python
dependency. Fetching requires network access. Inputs stay in ignored `target/`
directories, retain their upstream licenses, and are not vendored into this
repository. The compiler revision used for the baseline is `e2164c0`, including
typed resources, scopes, physical semantics, explicit targets, modules, and
independent stages. The report also records the exact executable's SHA-256 digest.

```sh
cargo build --release --locked
python3 tools/corpus.py fetch
python3 tools/corpus.py audit --inventory target/corpus/inventory.json
python3 -m unittest discover -s tools -p 'test_*.py'
```

`fetch` refuses dirty existing checkouts and checks out the manifest's full
commit IDs. `audit` does not fetch or update sources. It writes
`target/corpus/report.json` by default. Exit 0 means the selected preservation,
acceptance, and declared negative-case expectations passed; exit 1 means an
expectation failed; exit 2 means the evaluation could not complete. A nonzero
exit is required for any static rejection or nonidentical build output.

`--cache PATH`, `--binary PATH`, `--output PATH`, and `--inventory PATH` select
local artifacts. `--limit N` evaluates a deterministic prefix for a pilot and
records the limit; a limited run is not a full compatibility result. Input
enumeration and digest verification still cover the pinned selection.

The pilot processed one file in 0.034 seconds for three CLI invocations. Its
derived budget for the 998 selected files was approximately 34 seconds. The
full 998-file processing pass measured 15.595 seconds on the local macOS host,
excluding download, initial enumeration, and negative-fixture checks. This is
one measured pass, not a comparative performance benchmark. Timings and absolute
cache paths are omitted from JSON so repeated evaluation with the same binary,
sources, and evaluator produces identical reports and inventories.

Revalidation after the module and stage additions reproduced all prior selection,
rejection, diagnostic-count, frequency, and negative-case outcomes; only the
executable digest in the baseline changed. No target profile was enabled for this legacy
compatibility measurement. Capability errors from an explicitly selected target
are evaluated separately by the Rust tests, not counted as compatibility errors.

The CI corpus job performs these steps independently of the offline Rust suite
and uploads the report and inventory. Hosted CI has not been run locally or
published. Numerical semantic regression and the optional LAMMPS engine test
were not run for this evaluation.
