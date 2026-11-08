# Static processing cost and diagnostic preservation in Lamlet

## Abstract

The processing cost of Lamlet was measured before and after indexing manual lint
rules by command name. The evaluation used 1,321 pinned LAMMPS inputs and
fragments, with five measurements per input on one Apple M1 host. Indexing reduced
the sum of median compilation times from 138.36 ms to 103.24 ms, a factor of
1.34, while preserving every diagnostic JSON object. Fresh-process CLI latency
did not improve. A separate reference measurement used the pinned LAMMPS-AST
parser to characterize parsing and initialization costs. The tools have different
acceptance contracts, so those measurements do not establish relative validation
quality or scientific correctness.

## Introduction

Lamlet checks existing LAMMPS scripts without requiring an engine launch. Its
manual lint catalog evaluates physical constraints as Satisfied, Violated, or
Unknown. Repeatedly inspecting unrelated rules adds processing cost as the
catalog grows. This evaluation tests whether indexing rules reduces that cost
without changing diagnostics.

LAMMPS-AST provides a versioned grammar, a sanitizer, and a JSON-producing parser
command [1]. It supplies an external reference workload for this engineering
measurement. Lamlet preserves unfamiliar commands and source bytes, whereas
LAMMPS-AST normalizes source and constructs an AST for a supported grammar.
These contracts require separate definitions of acceptance and timing.

## Methods

### Inputs and acceptance

The input set combines the eight-repository [compatibility corpus](corpus.md)
with 240 published generated scripts and six agent demonstration scripts from
the [pinned Holbrook source](agent-evaluation.md#published-baseline).
Selection manifests specify repository revisions and exclusions [2, 3]. Every
input is verified against its pinned Git blob before measurement. Notebook and
Markdown fragments use the same extraction rules as the compatibility audit.
Includes are not followed, and no simulation is executed.

Lamlet acceptance means that its report contains zero errors. Unsupported
commands can remain Unknown. LAMMPS-AST acceptance requires an AST and an empty
error list after `sanitize` and `parse_to_AST(..., lint=True)`. Exceptions and
timeouts count as failures to accept, without implying that the script is
invalid. The AST source is fixed at
`05fa329f175a8d50a1ac938ac55cc59791fd3e4c`. Its checked-out sources and grammar
must match that revision. The selected grammar is `20240829`.

### Timing definitions

Persistent workers exclude process startup and input-file reads from warm
measurements. The AST worker initializes the grammar before measurement, then
times sanitization and AST construction together. The Lamlet worker initializes
its catalog, then measures syntax parsing, full compilation, and report
serialization separately. Full compilation includes source construction, parsing,
analysis, lint findings, and lowering. The separate parsing measurement is not
added to compilation time. Its result is consumed to prevent dead-code removal.
Report serialization includes construction of the report and compact JSON.

Each input has five measurements per implementation. Worker order rotates
between inputs. For each stage, the reported aggregate is the sum of per-input
medians, rather than the elapsed time of the complete benchmark. Cross-tool
timings use only inputs accepted by both implementations. All inputs remain in
the acceptance denominator. A worker has a 10 s response limit for the complete
five-repeat request and is restarted after a timeout.

Fresh-process measurements use the actual upstream `sanitize_and_parse.py` and
`lamlet check --language lammps --format json` commands. For each source group,
the sample is the median-size jointly accepted input, with path order breaking
size ties. Eleven groups have such an input. Command order rotates over five
repetitions. These measurements include imports, grammar construction, filesystem
reads, and JSON output. Filesystem caches are warm; this is not a cold-cache
storage benchmark.

### Optimization and preservation

The baseline compiler is revision `fda3d3d` [4]. Both Rust builds use the same
benchmark adapter, stable compiler, lockfile, and release profile. The optimized
engine builds a command-to-rule index once. It preserves catalog order, removes
duplicate command names within a rule, and applies the original selectors to
each candidate. This avoids scanning all 282 rules for every command.

For each input, the benchmark compares SHA-256 hashes of the complete compact
diagnostic JSON before and after optimization. It fails if any hash differs.
The regular corpus audit separately checks emitted bytes and both normal and
strict acceptance. Identical summaries alone are insufficient for either check.

## Results

### Compilation cost and diagnostics

Compilation was measured on all 1,321 inputs. Indexing reduced the aggregate
compilation time by 25.4%, with a baseline-to-optimized ratio of 1.34. All
1,321 complete diagnostic JSON hashes agreed. Table 1 distinguishes this
improvement from fresh-process command latency.

Table 1. Lamlet processing time before and after rule indexing. The compilation
row sums per-input medians over all 1,321 inputs. The CLI row sums medians over
11 representative inputs and includes process startup. All entries are in ms.

| Measurement | Before | After | Before / after |
| --- | ---: | ---: | ---: |
| Warm compilation, 1,321 inputs | 138.36 | 103.24 | 1.34 |
| Fresh-process CLI, 11 inputs | 48.15 | 52.48 | 0.92 |

The CLI measurement did not show an improvement. Its several-millisecond
startup cost is much larger than compilation of these individual scripts.

### Reference workloads and acceptance

Both implementations accepted 254 inputs. Table 2 separates warm processing
from fresh-process execution. AST processing includes source normalization and
AST construction. Lamlet compilation includes its implemented semantic checks
and lowering, with JSON serialization shown separately. These stages perform
different work and produce different data structures.

Table 2. Aggregate processing times in ms. Warm rows sum per-input medians for
the 254 jointly accepted inputs. CLI times sum per-input medians for the
11-file sample. Parsing is already included in full Lamlet compilation.

| Stage | LAMMPS-AST | Lamlet |
| --- | ---: | ---: |
| Warm sanitization and AST construction | 194.42 | N/A |
| Warm syntax parsing alone | N/A | 3.22 |
| Warm full compilation | N/A | 14.60 |
| Report construction and JSON serialization | N/A | 3.08 |
| Fresh-process CLI, 11 inputs | 3,283.42 | 52.48 |

Acceptance over the complete set is reported in Table 3. Lamlet's permissive
handling of unknown commands accounts for its acceptance policy. Acceptance of
generated scripts does not establish that they satisfy their scientific tasks.

Table 3. Accepted inputs divided by selected inputs. AST failures include one
timeout on `simongravelle-inputs/inputs/pouring-granular/input.lammps`.
The timeout remains in the denominator and is excluded from paired timing.

| Input source | Selected | LAMMPS-AST accepted | Lamlet accepted |
| --- | ---: | ---: | ---: |
| Official examples and benchmarks | 922 | 23 | 922 |
| LAMMPS Tutorials | 76 | 10 | 76 |
| Six additional collections | 77 | 22 | 77 |
| Published generated scripts | 240 | 193 | 240 |
| Published agent demonstrations | 6 | 6 | 6 |
| Total | 1,321 | 254 | 1,321 |

## Discussion

The indexed engine improves repeated checking while preserving the observed
diagnostic contract. It does not establish a CLI latency improvement. The
fresh-process comparison is dominated by implementation and initialization costs,
including Python imports and Lark grammar construction in the reference parser.
It should not be extrapolated to persistent editor services or simulation time.

The grammar date and acceptance policies also limit interpretation. Many corpus
inputs use commands outside the reference grammar. Lamlet intentionally accepts
such commands with incomplete verification. Consequently, the acceptance counts
are neither a false-positive comparison nor a measure of scientific accuracy.
The shared-acceptance subset is dominated by the published generated scripts and
does not represent every LAMMPS command family. No memory-use comparison,
confidence interval across independent sessions, or hardware generalization was
measured.

## Conclusions

Indexing manual rules reduces warm Lamlet compilation cost on the selected
corpus without changing complete diagnostic JSON. Fresh-process latency remains
a separate optimization target. The reference parser measurements characterize
processing costs under different contracts and do not establish replacement
compatibility or scientific validation quality.

## References

1. E. Holbrook, [LAMMPS-AST, pinned parser sources](https://github.com/ethanholbrook/LAMMPS-AST/tree/05fa329f175a8d50a1ac938ac55cc59791fd3e4c/lammps_ast).
2. Lamlet, [compatibility source manifest](https://github.com/lan496/lamlet/blob/fda3d3d/tests/corpus/sources.json).
3. Lamlet, [published-artifact source manifest](https://github.com/lan496/lamlet/blob/fda3d3d/research/holbrook/source.json).
4. Lamlet, [baseline compiler revision](https://github.com/lan496/lamlet/commit/fda3d3d).

## Appendix: environment, artifacts, and reproduction

The measurements used an Apple M1, Darwin 25.5.0 arm64, Python 3.12.16, and
`rustc 1.99.1 (58cf8f9c3 2026-10-13)`. Exact Python dependencies are pinned in
[benchmark-requirements.txt](https://github.com/lan496/lamlet/blob/main/research/holbrook/benchmark-requirements.txt).
The [compressed JSON record](assets/evaluation/2026-11-08-performance.json.gz)
contains every measurement, input identity and hash, AST diagnostic, binary hash,
benchmark-source hash, and group summary. It records zero engine launches and
zero agent trials. The full measurement took 49.37 s, including worker
initialization, IPC, a timeout, and the CLI sample. This wall time is distinct
from the summed stage medians in the tables.

Fetch the pinned inputs and build the current implementation:

```sh
python3 tools/corpus.py fetch
python3 tools/evaluation.py fetch
cargo build --release --locked --example benchmark --bin lamlet
```

To compare the pre-index baseline as well, archive its committed source and copy
the current development adapter into it. The archive is an ignored build input,
not a separate Git worktree.

```sh
mkdir -p target/performance/baseline
git archive --format=tar --output=target/performance/baseline.tar fda3d3d
tar -xf target/performance/baseline.tar -C target/performance/baseline
mkdir -p target/performance/baseline/crates/lamlet/examples
cp crates/lamlet/examples/benchmark.rs target/performance/baseline/crates/lamlet/examples/benchmark.rs
cargo build --manifest-path target/performance/baseline/Cargo.toml --target-dir target/performance/baseline-target --release --locked --example benchmark --bin lamlet
```

Run the full measurement without other CPU-intensive checks. Omitting the two
`--before-*` arguments measures only the current compiler and reference parser.
Timing values will vary across runs and hosts.

```sh
uv run --cache-dir target/uv-cache --python 3.12 --with-requirements research/holbrook/benchmark-requirements.txt python tools/compare_ast.py --before-worker target/performance/baseline-target/release/examples/benchmark --before-binary target/performance/baseline-target/release/lamlet --output target/performance/comparison.json
```

Use `--dataset published --case holbrook/publication/agent_tests/opus47/al_npt_eval/al_npt.in`
for one representative timing pilot before a full sweep. The pilot measured
0.66 ms for warm AST processing and 0.10 ms for pre-index Lamlet compilation plus
JSON. It supported a minute-scale budget for the full sweep; it was not used as
the reported comparison.
