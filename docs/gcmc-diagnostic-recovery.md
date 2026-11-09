# Semantic fact retention and GCMC diagnostic coverage

## Abstract

This study tests whether preserving command facts through runtime parameters
recovers a missing temperature-normalization diagnostic. The ten inputs and
native reference conditions of the source-derived audit were checked with a
revised semantic engine. Both full and instrumented GCMC inputs changed from an
absent target to a warning before correction and a satisfied finding after
correction. Native assessment remains restricted to the instrumented derivative;
both full-input runs retain their original timeout outcomes. All ten inputs
were accepted in normal, strict, and build modes and emitted byte for byte.
The result establishes a development regression fix. Independent expert review
and evaluation on unseen reports remain pending.

## Introduction

Temperature normalization in an exchanging atom population requires the
temperature compute to recalculate its atom count and degrees of freedom [1].
The [initial diagnostic audit](natural-diagnostic-audit.md) found that
`LML-PHYS-010` produced no target finding for a source-derived GCMC input,
including its shortened derivative with independently measured normalization
error. This study examines whether command-state retention recovers that
finding while preserving legacy compatibility and conservative uncertainty.

## Methods

### Fixed inputs and independent reference conditions

The [frozen audit plan](assets/defect-diagnostics/2026-11-09-gcmc/plan.json)
and [native replay methods](defect-corpus.md#native-replay-and-independent-measurements)
define ten before/after inputs from four source discussions. Input hashes,
transformations, target rules and subjects, and the relative tolerance of
10⁻⁸ are unchanged. The GCMC target is `LML-PHYS-010` with the exact evidence
subject `fix NVT temperature selection`. Outcomes are recomputed from the
previously archived native stdout; no new native simulation is included.

The instrumented derivative contains 400 steps. Its reference temperature is
<var>T</var><sub>kin</sub> = 2<var>K</var> /
[<var>k</var><sub>B</sub>(3<var>N</var> − 3)], using current atom count and
native kinetic energy. The original full-input variants both exceeded their
10 s limits and remain unassessed. The averaging case retains an execution-only
outcome. The velocity reconstruction retains its external speed contract and
has no mapped static detector. All inputs remain development data.

### Semantic state and diagnostic capture

The revised checker is pinned at
[`0c658e1`](https://github.com/lan496/lamlet/commit/0c658e13bf00770086fe8ca11679555046f4f00c).
It records fix identity, group, and style when those arguments precede the first
substitution. Later substitutions remain unevaluated and may produce multiple
words under LAMMPS parsing rules [2]. A literal `nvt` or `npt` prefix therefore
establishes the default temperature compute without determining the numeric
temperature or damping parameter [3]. Substitutions in the prefix remain opaque.

Region definitions and restart output scheduling preserve the tracked resource
and temperature-selection facts [4, 5]. Equal/index variable definitions also
preserve these facts. A single literal `fix_modify ... dynamic/dof yes` leaves
the selected compute unchanged. It does not enable that compute's dynamic
normalization; the relevant correction is `compute_modify ... dynamic/dof yes`
[1, 6]. Other unsupported options retain conservative treatment.

Opaque effects retain previously observed thermostat and atom-exchange roles
as Unknown. Compute updates alone cannot establish that these fixes remain
active. New fix definitions can establish known roles, while unfix and clear
remove them. Compiler and CLI tests cover these cases, parameterized prefixes,
multiword tails, deterministic JSON, strict acceptance, and byte preservation.

Each frozen input is checked in normal, strict, and build modes. The new capture
records the revised source commit separately from the plan's baseline revision.
Its report distinguishes an absent target from Unknown and tests the exact
target subject. A second capture compares every deterministic archive file.
Historical checker observations and native evidence remain unchanged.

## Results

The revised checker emits the declared GCMC finding for both variants of each
GCMC plan. Table 1 compares these observations with the frozen baseline and
native reference conditions.

*Table 1. Before/after findings for all five replay plans. Each target column
describes the input before correction followed by its corrected variant.
"Absent" denotes no finding for the declared rule and subject. "Unmapped"
denotes no assumed detector. Native conditions apply only to each replay's
bounded measurement; the two GCMC plans share one source discussion.*

| Replay plan | Native before → after | Baseline target | Revised target |
| --- | --- | --- | --- |
| Lattice orientation | Violated → satisfied | Violated, warning → satisfied | Violated, warning → satisfied |
| Averaging | Execution only → execution only | Unmapped → unmapped | Unmapped → unmapped |
| Full GCMC | Unassessed → unassessed | Absent → absent | Violated, warning → satisfied |
| Instrumented GCMC | Violated → satisfied | Absent → absent | Violated, warning → satisfied |
| Velocity | Violated → satisfied | Unmapped → unmapped | Unmapped → unmapped |

The instrumented GCMC warning now corresponds to the previously measured
66-fold temperature-normalization mismatch at step 400. The corrected compute
policy produces a satisfied finding, consistent with the archived native
agreement within 10⁻⁸. The full-input static findings have no completed native
reference assessment and therefore provide no additional independently
assessed negative/corrected pair.

All ten inputs are accepted in normal, strict, and build modes. All ten emitted
hashes match their frozen input hashes, and diagnostics agree across modes.
There are no CLI infrastructure failures. The second capture matches all 43
deterministic files, including the 30 full CLI responses. Nine inputs still
contain at least one Unknown finding; these do not substitute for the exact
GCMC target. The [report](assets/defect-diagnostics/2026-11-09-gcmc/report.json)
retains all findings and original native outcome categories.

## Discussion

The recovered diagnostic depends on separating resource identity from numeric
parameter evaluation. A runtime temperature argument does not erase a literal
thermostat style or its default compute definition. Conversely, an opaque
command may remove or replace that fix, so retaining the old role as known
would permit unsupported warnings or satisfied findings. Explicit Unknown
outcomes preserve this distinction.

The improvement is measured on the development inputs that motivated the fix.
The full and instrumented GCMC plans are dependent observations, and neither
their duplication nor the lattice result estimates population accuracy. The
[review form](assets/defect-diagnostics/review/review.json) still contains five
pending decisions and zero admitted gold pairs. No agent trial, independent
scientific review, or prospective evaluation on unseen reports was performed.
The checker still does not evaluate arbitrary substitutions, establish actual
atom insertion, verify the complete physical model, or prove long-time
temperature control. An external-intent velocity discrepancy remains unmapped.

## Conclusions

Preserving literal resource facts recovers the source-derived GCMC diagnostic
without changing the native criteria or legacy text. Retaining uncertain roles
as Unknown prevents unsupported certainty after opaque effects. These changes
provide a tested regression boundary for subsequent evaluation with independent
labels and previously unseen inputs.

## References

1. LAMMPS, [compute_modify](https://docs.lammps.org/compute_modify.html), 30Sep2026.
2. LAMMPS, [input parsing rules](https://docs.lammps.org/Commands_parse.html), 30Sep2026.
3. LAMMPS, [fix nvt/npt](https://docs.lammps.org/fix_nh.html), 30Sep2026.
4. LAMMPS, [region](https://docs.lammps.org/region.html), 30Sep2026.
5. LAMMPS, [restart output](https://docs.lammps.org/restart.html), 30Sep2026.
6. LAMMPS, [fix_modify](https://docs.lammps.org/fix_modify.html), 30Sep2026.

## Appendix: artifacts and reproduction

The [capture receipt](assets/defect-diagnostics/2026-11-09-gcmc/capture.json)
records binary SHA-256
`a9004de69ea905dadc1f6656a67ab7aee7f0c35601eb98b186131c8ff099c1df`.
The [inventory](assets/defect-diagnostics/2026-11-09-gcmc/inventory.json) covers
44 files. The [budget record](assets/defect-diagnostics/2026-11-09-gcmc/budget.json)
measures the first three-mode input at 0.0245 s and estimates the remaining
nine at 0.2208 s. These are collection costs, not throughput measurements.
The runner records the declared source revision but does not independently
establish the binary's source provenance. The original cache is required to
reproduce transformed input hashes. Public artifacts omit third-party scripts.

From a clean checkout of the recorded checker commit:

```sh
cargo build --release --locked
python3 tools/defect_diagnostics.py run \
  --checker-source-commit 0c658e13bf00770086fe8ca11679555046f4f00c \
  --cache target/defect-pilot-20261109/sources \
  --output target/diagnostic-gcmc-new
python3 tools/defect_diagnostics.py verify \
  --archive docs/assets/defect-diagnostics/2026-11-09-gcmc
python3 -m unittest discover -s tools -p test_defect_diagnostics.py
```

Offline verification recomputes the report from archived CLI responses and
native evidence. It does not invoke the checker or establish uncached source
bytes. The original checker archive and its observations are still available
in the [baseline study](natural-diagnostic-audit.md).
