# Diagnostic coverage of source-derived LAMMPS conditions

## Abstract

This audit examines whether existing Lamlet diagnostics identify bounded
conditions reproduced from public LAMMPS reports. Ten frozen inputs from four
source discussions were checked in normal, strict, and build modes. The lattice
integer-argument rule warned on the rejected command and reported satisfaction
after correction. The dynamic-degrees-of-freedom rule produced no finding for
either variant of an instrumented GCMC input, despite an independently measured
66-fold temperature-normalization mismatch before correction. A velocity-unit
reconstruction depended on externally stated intent and had no mapped detector.
All ten legacy inputs were accepted and emitted byte for byte. The audit exposes
a coverage gap while retaining non-reproductions and timeouts; its development
sample and pending expert review do not support an accuracy estimate.

## Introduction

The checker revision and observations on this page remain frozen. A
[separate GCMC regrade](gcmc-diagnostic-recovery.md) records the subsequent
semantic-state fix against the same inputs and native reference conditions.

The [natural defect collection](defect-corpus.md) preserved source reports,
proposed corrections, missing dependencies, and native execution evidence. Its
bounded measurements provide conditions against which checker behavior can be
examined without using Lamlet to determine the reference outcome. This audit
asks whether the existing rules emit the declared finding, whether unresolved
analysis is visible, and whether checking preserves legacy acceptance and text.
It complements the synthetic [lint-condition evaluation](lint-evaluation.md).

## Methods

### Inputs and reference conditions

The five replay plans contain before and after variants from four discussions
[1–4]. Both GCMC plans derive from the same source. The native inputs,
transformations, measurements, and relative tolerance of 10⁻⁸ are unchanged from
the [native replay methods](defect-corpus.md#native-replay-and-independent-measurements).
The audit verifies source-cache hashes and reproduces each transformation before
invoking Lamlet. Full trajectories that timed out remain unassessed; successful
execution of the averaging input receives only an execution outcome.

For the instrumented GCMC input, the reference compares the thermostat
temperature with <var>T</var><sub>kin</sub> = 2<var>K</var> /
[<var>k</var><sub>B</sub>(3<var>N</var> − 3)], calculated from native kinetic
energy and current atom count. The criterion concerns normalization during the
400-step derivative, not equilibration or the original long-time freezing. For
the one-atom velocity reconstruction, the reference compares kinetic energy
with an externally prescribed speed of 100 Å ps⁻¹. This is a reconstructed unit
test, not a reproduction of the source irradiation model.

### Diagnostic observations

The [audit plan](assets/defect-diagnostics/2026-11-09/plan.json) was committed in
[`58b3b7d`](https://github.com/lan496/lamlet/commit/58b3b7d) before the first
checker capture. It pins the native evidence and checker source at
[`3b6e05a`](https://github.com/lan496/lamlet/commit/3b6e05a), identifies the target
rules, and declares expected outcomes. The lattice condition maps to
`LML-MAN-LATTICE-ORIENT-INTEGERS`. The GCMC condition maps to `LML-PHYS-010`,
restricted to the evidence subject `fix NVT temperature selection`. Neither
averaging nor velocity has an assumed detector. The original GCMC input retains
its static prediction but is excluded from comparisons requiring a native
condition assessment.

Each input is submitted to `lamlet check`, `lamlet check --strict`, and
`lamlet build`, all with `--language lammps --format json`. A target warning requires both the declared
rule and subject. Findings are retained as violated, satisfied, or unknown;
an absent target is recorded separately. An unavailable CLI result is an
infrastructure failure. Unrelated warnings and Unknown findings remain in the
full report but cannot satisfy the declared target. Build output is compared
with the frozen input by SHA-256.

Temporary source paths are normalized in captured JSON; spans, evidence, and
outcomes are preserved. The report records the hash of every full CLI response.
A second capture checks deterministic output with the same binary and inputs.
Wall-clock timing is stored separately from the deterministic report. Compiler
rules and native measurement code were unchanged throughout the audit.

## Results

### Condition coverage

Comparison with native evidence identifies a missing diagnostic for the GCMC
temperature normalization. Table 1 retains every replay plan, including cases
without a native condition label or a mapped rule.

*Table 1. Native condition assessments and declared Lamlet target findings for
each before/after pair. “Absent” means that the declared rule and subject have
no finding, including no Unknown finding. “Unmapped” means that the plan assumes
no detector. Native satisfaction concerns only the bounded condition stated in
the final column.*

| Replay plan | Native before → after | Target before → after | Assessment scope |
| --- | --- | --- | --- |
| Lattice orientation [3] | Violated → satisfied | Violated, warning → satisfied, no warning | Extracted integer arguments; reconstructed correction |
| Averaging [4] | Execution only → execution only | Unmapped → unmapped | Historical failure did not reproduce |
| Full GCMC [1] | Unassessed → unassessed | Absent → absent | Both native variants timed out |
| Instrumented GCMC [1] | Violated → satisfied | Absent → absent | Temperature normalization in a shortened derivative |
| Velocity [2] | Violated → satisfied | Unmapped → unmapped | New one-atom model with external speed contract |

At step 400 of the instrumented GCMC input, the previously archived thermostat
temperature was 1058.478 K, while normalization using the current atom count gave
16.03754 K. The reported correction restored agreement to within the frozen
tolerance. Neither variant produced the declared `LML-PHYS-010` finding. This is
an observed miss for the bounded negative condition, followed by absence of a
finding for the corrected condition. It is not an explicit Unknown assessment
of either target.

The velocity command alone does not encode the external intention to use box
units. Its measured energy discrepancy therefore remains an unmapped condition,
without an assumed static warning requirement. The original GCMC timeouts also
remain outside the native-condition comparison. These distinctions prevent
counting all ten inputs as independently labeled positive or negative cases.

### Compatibility and reproducibility

Table 2 reports operational checks separately from condition coverage. Nine
inputs contained at least one Unknown finding from other rules; this count does
not indicate coverage of the declared condition. Only the lattice before variant
produced any warning.

*Table 2. Operational results for ten frozen legacy inputs. Acceptance means
zero reported compile errors. JSON agreement compares all three modes for the
same input. Repeated-capture agreement compares the two captures after path
normalization and excludes timing and its inventory hash.*

| Check | Observed inputs |
| --- | --- |
| Normal check accepted | 10 / 10 |
| Strict check accepted | 10 / 10 |
| Build accepted | 10 / 10 |
| Emitted text identical to input | 10 / 10 |
| Normal, strict, and build JSON agree | 10 / 10 |
| Full CLI JSON agrees across repeated captures | 10 / 10 |
| CLI infrastructure failure | 0 / 10 |

## Discussion

The GCMC result shows that an implemented rule can have narrower practical
coverage than its condition description suggests. Source inspection identifies
state-loss paths that can explain this absence. The semantic engine calls
`weaken()` for ambiguous commands and unmodeled commands; this clears the tracked
atom-exchange facts and temperature definitions [5]. The source input uses
variable-expanded numerical arguments and a `restart` command. The tokenizer
marks variable substitutions as ambiguous [6], and `restart` is outside the
semantic engine's state-preserving command list. The audit did not isolate each
barrier in a reduced reproduction, so this inspection does not establish a
single causal command. Losing the exchange fact can nevertheless leave the
dynamic-DOF evaluator without an entity for which to report uncertainty.

An explicit Unknown target would tell an agent that the relevant condition was
considered but unresolved. An absent finding provides no such evidence.
Acceptance and byte preservation remain useful compatibility properties, but
they cannot establish that the physical condition was checked. Future changes
should preserve uncertainty and recover facts only where command semantics
justify doing so. The frozen observations provide a regression target; changing
the checker requires a separately identified version and capture.

This audit uses purposefully selected development data, including sources
already used in rule development. Derivatives from the same discussion are
dependent observations. Independent expert review is pending, no gold defect
pairs have been admitted, and full-model validity remains outside the bounded
criteria. The sample supports diagnosis of a concrete coverage gap, not estimates
of population precision, false-positive rate, scientific accuracy, or agent
success. The [review packet](assets/defect-diagnostics/review/packet.md) presents
source intent, proposed changes, native evidence, and scope without checker
observations; its [review form](assets/defect-diagnostics/review/review.json)
retains all five decisions as pending.

## Conclusions

The source-derived audit distinguishes a detected argument restriction from a
missed temperature-normalization condition. It also preserves unassessed runs,
version-dependent non-reproduction, and reconstructed intent. This evidence
motivates improving the visibility and retention of semantic uncertainty before
using checker acceptance as feedback in further agent trials.

## References

1. matsci.org, [GCMC of confined LJ fluid](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447).
2. matsci.org, [Inconsistent kinetic energy calculation](https://matsci.org/t/inconsisten-kinetic-energy-calculation/56524).
3. matsci.org, [Lattice orientation vectors](https://matsci.org/t/lattice-orient-these-vectors-must-be-orthogonal/18394).
4. matsci.org, [Averaged per-atom output and dump timing](https://matsci.org/t/fix-ave-atom-issues-deciphering-dump-custom-and-fix-not-at-compatible-times/34056).
5. Lamlet at `3b6e05a`, [semantic state handling](https://github.com/lan496/lamlet/blob/3b6e05aede55550604ce871bd57439b7bf568911/crates/lamlet/src/semantics.rs).
6. Lamlet at `3b6e05a`, [legacy command tokenization](https://github.com/lan496/lamlet/blob/3b6e05aede55550604ce871bd57439b7bf568911/crates/lamlet/src/syntax.rs).

## Appendix: artifacts and reproduction

The public archive contains the [deterministic report](assets/defect-diagnostics/2026-11-09/report.json),
[capture receipt](assets/defect-diagnostics/2026-11-09/capture.json),
[budget](assets/defect-diagnostics/2026-11-09/budget.json), and
[inventory](assets/defect-diagnostics/2026-11-09/inventory.json). It records all
30 full CLI responses and ten emitted-file hashes. The measured first input
took 0.0258 s across its three modes, giving a prospective estimate of 0.232 s
for the remaining nine heterogeneous inputs. This estimate is a budgeting check,
not a throughput benchmark. Forty-three files, including all responses, hashes,
and the final report, were byte-identical in the second capture; only the timing
file and its inventory differed. No new LAMMPS simulation or agent trial was run.

Offline verification checks the archive inventory and recomputes the report
from archived CLI responses and native evidence. It does not rerun the checker
or verify uncached original source bytes:

```sh
python3 tools/defect_diagnostics.py verify \
  --archive docs/assets/defect-diagnostics/2026-11-09
python3 -m unittest discover -s tools -p test_defect_diagnostics.py
```

An exact live replay requires the original source cache and the recorded checker
version. The binary hash is captured, but its declared source revision is not
independently established by the runner. A different checker version requires
a new, separately identified experiment. Fresh forum retrieval may produce
different response bytes. Third-party
inputs remain in ignored caches; source availability and redistribution limits
are documented in the [collection protocol](https://github.com/lan496/lamlet/blob/main/research/defects/README.md).
The collector requires a new output directory and verifies transformed input
hashes before collecting results:

```sh
cargo build --release --locked
python3 tools/defect_diagnostics.py run \
  --cache target/defect-pilot-20261109/sources \
  --output target/diagnostic-audit-new
python3 tools/defect_diagnostics.py prepare-review \
  --output target/condition-review-new
python3 tools/defect_diagnostics.py validate-review \
  --review docs/assets/defect-diagnostics/review/review.json
```

The final command exits 1 for the published pending review. A named reviewer's
attestation and a reasoned decision for every case are required for agreement;
the validator neither verifies identity nor admits gold labels. The audit
collector exits 0 when collection and legacy controls succeed, 1 for per-mode
infrastructure or compatibility failures, and 2 for invalid inputs or global
integrity failures. A mismatch with a declared diagnostic prediction is retained
as an observation and does not turn collection success into a scientific pass.
