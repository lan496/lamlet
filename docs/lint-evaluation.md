# Evaluation of documented lint conditions

## Abstract

Lamlet's diagnostic behavior was evaluated using 45 command fragments covering
seven documented constraints and review conditions. The CLI detected 14 of 14
positive conditions and emitted no warnings on 16 negative controls. All 14
cases designated as statically unresolved produced a target Unknown finding.
Every fragment was accepted in normal and strict mode and emitted byte-for-byte
unchanged. These measurements characterize an engineering validation set;
scientific correctness and effects on agent performance were not assessed.

## Introduction

Compatibility tests measure whether existing inputs survive compilation. They
provide no reference labels for diagnostic accuracy. This evaluation measures
whether selected documented conditions produce warnings, whether counterexamples
avoid warnings, and whether incomplete information remains unresolved. It covers
the four forum-motivated rules together with integrator overlap, temperature
definition consistency, and averaging intervals. The [compatibility evaluation](corpus.md)
and [agent evaluation](agent-evaluation.md) address separate questions.
The [natural defect collection pilot](defect-corpus.md) examines public failure
reports and bounded native replays; it does not use this mutation set as evidence
of accuracy on research inputs.

## Methods

### Corpus and reference labels

The [corpus manifest](https://github.com/lan496/lamlet/blob/main/tests/lint/corpus.json)
contains 45 fragments with stable semantic IDs, input hashes, applicability,
per-case rationales, and expected target findings. Each positive case is a
single text replacement in a named negative control. Input-file hashes, the
complete file inventory, mutation reconstruction, and manual-page hashes are
verified before any case selection. The manual reference is LAMMPS revision
`0165101ce505734e8fb249694b22e295a7ab4182`, dated 30 September 2026 [1–6].

Labels were derived from the manual and explicit arithmetic before CLI execution.
They were authored separately from observed reports but have not received
independent human review. The implementation and evaluation share an author and
the same documentation sources. Each label concerns one stated condition under
its surrounding prerequisites. The fragments omit complete atom, force-field,
group, or box setup and are not claimed to be runnable simulations.

The [independent review packet and form](lint-label-review.md) expose every input,
mutation difference, proposed rationale, and pinned source without observed
checker output. The uncompleted form records this review as pending.

Positive labels identify either a constraint violation or a documented advisory
condition. Nonzero drag, different temperature observables, and exclusions with
long-range solvers can be intentional [2,3,5]. Detecting their review conditions
does not establish a scientific error. Negative controls include real-valued
primitive lattice vectors, final orientation overrides, a group named `drag`,
cleared neighbor exclusions, NVE with Langevin, equivalent temperature computes
with different IDs, and equality at the averaging-window bound.

Fourteen cases require Unknown because runtime values, group membership, opaque
effects, unsupported definitions, or checked-arithmetic limits prevent this
analysis from establishing the condition. One additional out-of-scope control
uses `ave/correlate`, whose sampling-window rule differs from `ave/time` [6].
Its target finding is expected to be absent. An absent finding is recorded
separately from Unknown.

### Measurements

Each fragment is passed to the release CLI with `--language lammps --format json`
using normal `check`, strict `check`, and `build`. Detection is the number of
positive cases with the expected target warning divided by all positive cases.
The false-warning rate is the number of negative controls with any warning
divided by all negative controls. The Unknown rate is the number of cases with
a target Unknown finding divided by all selected cases. Empty denominators have
a JSON rate of `null`.

Acceptance, byte identity, and equality of all three diagnostic reports are
recorded separately. Exact warning-code sets and target outcome lists must match
the independently specified expectations. Missing files, stale references, and
CLI crashes fail the evaluation. Reports contain hashes and explicit denominators
without timestamps, timing fields, or absolute host paths. Repeated runs with the
same executable must produce identical JSON. CI compares archived observations
and the hash of each complete CLI report after normalizing its source path,
while allowing the executable hash to differ across builds.

## Results

The [archived report](assets/lint-evaluation/report.json) records every expected
and observed outcome. Table 1 gives the per-rule measurements. All 14 positive
conditions were detected, and none of the 16 negative controls emitted a warning.
The target Unknown rate was 14/45, or 31.1%; all 14 intended unresolved cases were
recognized without warnings. Three cases had no target finding: the two
nonoverlapping integration controls and the out-of-scope averaging style.

*Table 1. Diagnostic measurements on the fixed engineering corpus. Entries are
observed counts divided by their relevant case counts. Detection concerns positive
review conditions, false warnings concern negative controls, and Unknown uses all
cases for that rule as its denominator.*

| Rule | Detection | False warnings | Unknown |
| --- | ---: | ---: | ---: |
| `LML-MAN-LATTICE-ORIENT-INTEGERS` | 2/2 | 0/2 | 2/6 |
| `LML-MAN-LATTICE-ORIENTATION` | 2/2 | 0/3 | 2/7 |
| `LML-MAN-NH-DRAG` | 2/2 | 0/3 | 2/7 |
| `LML-MAN-KSPACE-EXCLUSIONS` | 2/2 | 0/2 | 2/6 |
| `LML-PHYS-001` | 2/2 | 0/2 | 2/6 |
| `LML-PHYS-002` | 2/2 | 0/2 | 2/6 |
| `LML-MAN-AVE-TIME` | 2/2 | 0/2 | 2/7 |

Normal checking, strict checking, and building accepted 45/45 fragments. Build
outputs preserved 45/45 inputs byte-for-byte, and the three diagnostic reports
agreed for every fragment. This acceptance result includes the positive cases
because these legacy diagnostics are nonfatal warnings.

## Discussion

The controls establish that these selected rule boundaries behave as specified.
They do not estimate a population error rate: the cases were deliberately chosen,
the sample is small, and the rule implementation was already known. General
diagnostic precision on real LAMMPS projects remains unmeasured. Independent
expert review and a held-out corpus are required before using these observations
as evidence of scientific reliability. No agent or LAMMPS process was launched,
and no trajectory, energy, geometry, or physical result was validated.

## Conclusions

The engineering corpus provides reproducible checks for detection, false warnings,
Unknown handling, and legacy preservation. Its fixed reference labels and
counterexamples support regression testing of seven rules. The measurements
provide no estimate of improvement in agent-generated scientific calculations.

## References

1. LAMMPS, [lattice](https://docs.lammps.org/lattice.html).
2. LAMMPS, [Nose–Hoover integration and drag](https://docs.lammps.org/fix_nh.html).
3. LAMMPS, [neighbor exclusions](https://docs.lammps.org/neigh_modify.html).
4. LAMMPS, [NVE integration](https://docs.lammps.org/fix_nve.html) and
   [Langevin thermostat](https://docs.lammps.org/fix_langevin.html).
5. LAMMPS, [COM-subtracted temperature](https://docs.lammps.org/compute_temp_com.html),
   [fix temperature selection](https://docs.lammps.org/fix_modify.html), and
   [thermodynamic temperature selection](https://docs.lammps.org/thermo_modify.html).
6. LAMMPS, [time averaging](https://docs.lammps.org/fix_ave_time.html) and
   [correlation averaging](https://docs.lammps.org/fix_ave_correlate.html).

## Appendix: reproduction

The following commands build the CLI, check the corpus against the archived
observations, and verify repeated-output determinism. A single-case timing run
is available with `--case averaging-boundary`; it verifies the entire inventory
before selecting the case.

```sh
cargo build --release --locked
python3 tools/lint_evaluation.py --baseline docs/assets/lint-evaluation/report.json
python3 tools/lint_evaluation.py --report target/lint-evaluation/repeated.json
cmp target/lint-evaluation/report.json target/lint-evaluation/repeated.json
python3 -m unittest discover -s tools -p 'test_*.py'
```

Exit code 0 denotes agreement, 1 denotes an outcome or archived-observation
mismatch, and 2 denotes an infrastructure or input-integrity failure. The
binary version and SHA-256 are stored in each report. Previous pilot and
published-script archives remain unchanged.
