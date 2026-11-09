# From public LAMMPS reports to reproducible defect cases

## Abstract

A pilot collection examined whether public LAMMPS failure reports can support
independent tests of scientific input defects. Twenty discussions and 23 unique
attachments were captured, preserving proposed fixes, unresolved reports,
counterexamples, and an engine bug. Five replay plans produced ten native
execution outcomes. In a shortened, instrumented GCMC input, the thermostat
temperature exceeded the value calculated from kinetic energy and the current
atom count by a factor of 66. The reported one-line correction restored agreement
within floating-point precision. A single-atom reconstruction reproduced a
velocity-unit energy error, whereas a historical averaging failure did not recur
on the current engine. Full GCMC runs exceeded the time limit. These observations
support narrower condition tests, but independent review remains pending and no
case is yet admitted as a gold defect pair. No checker accuracy or agent benefit
was measured.

## Introduction

Synthetic mutations can test a known diagnostic boundary without showing whether
that condition occurs in research calculations. Natural reports provide intended
calculations and observed failures, but their proposed corrections may be
incomplete, contradicted, or specific to an older executable. This pilot asks
which evidence is needed to convert such reports into reproducible defect and
correction pairs. It complements the [fixed lint-condition evaluation](lint-evaluation.md)
and the [controlled constraint calculations](constraint-validation.md).

A separate [diagnostic audit](natural-diagnostic-audit.md) compares the frozen
replays with the existing checker. Its observations and compatibility checks are
reported separately from this native collection experiment.

The criterion is a demonstrated violation of a stated condition, followed by a
correction that satisfies the same condition. Normal termination alone is
insufficient. A report of unexpected behavior also does not establish an input
defect: an engine bug or a deliberate modeling choice requires a different label.

## Methods

### Selection, capture, and screening

Twenty public matsci.org discussions were purposefully selected from existing
forum references and searches concerning units, temperature, geometry, resources,
and output. Lamlet findings were not used to select, replay, or adjudicate them.
Some sources had already informed existing Lamlet rules. All cases are therefore
development data, with no held-out evaluation subset.

The collector retrieved every post in each Discourse post stream, including
additional pages, and downloaded attachments linked through the forum. It recorded
original responses, per-post and code-block hashes, attachment hashes, and retrieval
time. External file-sharing links were recorded as dependencies rather than
treated as recovered files. Each discussion was read through its final reply.
The screening manifest records intent, a bounded condition, cited post numbers,
short evidence anchors, reported engine versions, missing dependencies, and
reasons for deferral.

The dispositions describe the evidence in a discussion. A reported fix includes
an author's confirmation; a proposed fix lacks that confirmation. Neither is an
independent validity label. Unresolved cases, engine bugs, and counterexamples are
retained in the screened denominator. Raw third-party scripts remain in ignored
local caches because public access does not establish redistribution permission.

### Native replay and independent measurements

The replay plan and a relative tolerance of 10⁻⁸ were fixed before native
execution. Five plans cover four distinct source threads. They distinguish an
original attachment, a source fragment, a full inline input, an instrumented
derivative of that input, and a newly constructed model. Each variant has a
10-second process limit. No Lamlet command is invoked by the collector or the
measurement code.

For the GCMC report [1], the original and corrected million-step inputs were
retained. A separate derivative uses 400 steps and adds outputs for atom count,
kinetic energy, and both temperature computes. The only before/after change is:

```lammps
# Before
fix_modify      NVT dynamic/dof yes
# After
compute_modify  NVT_temp dynamic/dof yes
```

The normalization check calculates
\(T_{\mathrm{kin}}=2K/[k_B(3N-3)]\) from native kinetic energy \(K\) and current
atom count \(N\), with \(k_B=0.0019872067\) kcal mol⁻¹ K⁻¹. The model is
three-dimensional, has no molecular constraints, and its fluid group includes
all atoms. This arithmetic is separate from the selected temperature compute
and from Lamlet. It tests the normalization used by the thermostat, without
requiring the transient temperature to equal its setpoint. LAMMPS documents the
default subtraction of three translational degrees of freedom and the need to
update normalization when atom counts change [5,6].

The velocity report [2] lacks the potential needed for its original irradiation
model. A new free-particle model instead uses one tungsten atom of mass
183.84, lattice spacing 3.13400196 Å, and prescribed speed 100 Å ps⁻¹. It compares
implicit lattice units with explicit `units box`. The intended kinetic energy
is computed independently as \(mv^2/2\), with the pinned metal-unit conversion
factor 0.00010364269. LAMMPS documents lattice scaling as the default [7].
This reconstruction isolates the unit mechanism; it does not reproduce the
source model or its intended 20 eV collision.

The lattice replay extracts the posted command and replaces its fractional
orientation directions with proportional integer directions [3]. It tests
argument acceptance only. The averaging replay uses the original attached
32,000-atom input and a variant that removes its final `write_dump`, retaining
the scheduled dump [4]. These two plans do not validate physical observables.

## Results

### Collection outcomes

All 20 selected discussions were captured, including all posts and 23 unique
forum-hosted attachments. No such attachment download failed. This does not
imply complete simulation dependencies: several cases require unprovided
potentials or external files. Table 1 gives the screening dispositions, which
cover 15 provisional family categories rather than 20 independent mechanisms.

**Table 1.** Discussion-level screening outcomes. Counts describe source evidence,
not independently verified negative inputs.

| Disposition | Discussions |
| --- | ---: |
| Author-reported fix | 6 |
| Proposed fix without confirmed resolution | 7 |
| Unresolved | 4 |
| Counterexample to an incorrect-input label | 2 |
| Engine bug | 1 |
| Total screened | 20 |

Complete-thread reading changed the interpretation of several reports. In the
SHAKE/NPT ordering discussion, the final test found no dynamics or pressure
difference after swapping those fixes [8]. A model initially described as solved
with `recenter` was challenged in later replies [9]. Another crash was traced
to LAMMPS and an upstream engine fix [10]. None is labeled as an invalid input
merely because an earlier message proposed a remedy.

### Native outcomes

Table 2 retains every replay outcome, including both full-input timeouts. The
GCMC rows derive from the same report and are not independent defect cases.

**Table 2.** Native outcomes on the pinned modern serial engine. “Before” and
“after” identify replay variants; they do not imply that a defect was reproduced
or a complete simulation was validated.

| Source and provenance | Before | After | Supported observation |
| --- | --- | --- | --- |
| Lattice orientation [3]; command fragment with reconstructed correction | Exit 1 | Exit 0 | Modern engine rejects a fractional integer argument |
| Averaging [4]; complete attachment and proposed edit | Exit 0 | Exit 0 | Historical failure not reproduced |
| GCMC [1]; full inline input and reported edit | 10 s timeout | 10 s timeout | Full trajectory unassessed |
| GCMC [1]; 400-step instrumented derivative | Exit 0 | Exit 0 | Temperature normalization differs before correction |
| Velocity [2]; new single-atom reconstruction | Exit 0 | Exit 0 | Implicit lattice scaling changes prescribed kinetic energy |

In the instrumented GCMC replay, the atom count increased from 2 to 67. At
step 400, the original thermostat compute returned 1058.478 K, while independent
normalization of the same state's kinetic energy gave 16.03754 K. The ratio was
66, consistent with retaining the initial three degrees of freedom instead of
the current 198. After the reported correction, the corresponding temperatures
were both 90.42591 K. Across all five sampled steps, the maximum relative
normalization error fell from 65 to 2.22 × 10⁻¹⁶, below the prespecified 10⁻⁸
tolerance. Both variants terminated normally. The corrected transient also
reached 444.7 K at step 300; agreement of the two definitions does not establish
thermal equilibration at the 77 K setpoint.

In the velocity reconstruction, the intended energy was 95.26836 eV. The default
command produced 935.7228 eV, a factor of 9.821968, equal to the square of the
lattice spacing. Explicit box units produced 95.26836 eV. This is a measured unit
interpretation effect in the reconstruction, not a replay of the original
irradiation calculation.

## Discussion

The GCMC result illustrates a useful natural negative condition: successful
execution can coexist with a large inconsistency in the physical quantity used
for feedback. Its one-line correction addresses the measured normalization
condition. The pilot does not establish that it resolves the reported
million-step freezing, since neither full trajectory completed within the cap.
The velocity reconstruction provides a second observable test while exposing
the loss of original-model provenance caused by a missing potential.

Conversely, the averaging input cannot serve as a current-engine negative on
the evidence collected here. Replaying both variants successfully is a reason
to preserve engine-version applicability, not to count the proposed edit as a
validated repair. The lattice fragment establishes a language restriction but
cannot establish the intended orientation of a complete crystal.

Selection was nonrandom, confined to one forum, and partly overlapped with rule
development sources. Repeated mechanisms and the two derivatives of one GCMC
report reduce independence. Historical binaries, production trajectories,
parallel behavior, and most source models were not reproduced. The arithmetic
checks use native kinetic energy and atom counts, so they are independent of
Lamlet's diagnostics but not an independent implementation of molecular
dynamics. Domain-expert adjudication, diagnostic precision, false-positive
rates, and effects on agent success remain unmeasured. There are zero gold
defect pairs and zero held-out cases in this pilot.

Future admission requires the original input and dependencies, stated intent,
an explicit correction, an independent condition test, applicable engine
versions, and domain review. Original inputs, derivatives, reconstructions, and
mutations must share a split; related mechanisms also require grouping when
testing generalization. This separates reproducible condition evidence from
claims about complete scientific calculations.

## Conclusions

Public reports supplied recoverable inputs and measurable defect mechanisms,
but source confirmation alone was insufficient for a benchmark label. The
instrumented GCMC case exposed a 66-fold temperature-normalization mismatch
that the reported correction removed. Version-dependent non-reproduction,
missing dependencies, and full-input timeouts materially limited pair admission.
The resulting collection is development evidence for further review, rather
than a measurement of Lamlet's scientific accuracy.

## References

1. matsci.org, [GCMC of confined LJ fluid](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447).
2. matsci.org, [Inconsistent kinetic energy calculation](https://matsci.org/t/inconsisten-kinetic-energy-calculation/56524).
3. matsci.org, [Lattice orientation vectors](https://matsci.org/t/lattice-orient-these-vectors-must-be-orthogonal/18394).
4. matsci.org, [Averaged per-atom output and dump timing](https://matsci.org/t/fix-ave-atom-issues-deciphering-dump-custom-and-fix-not-at-compatible-times/34056).
5. LAMMPS, [temperature computation](https://docs.lammps.org/compute_temp.html).
6. LAMMPS, [compute modification and dynamic degrees of freedom](https://docs.lammps.org/compute_modify.html).
7. LAMMPS, [velocity units](https://docs.lammps.org/velocity.html).
8. matsci.org, [Ordering of SHAKE and NPT](https://matsci.org/t/ordering-of-fix-shake-and-fix-npt/16297).
9. matsci.org, [Fixed groups in NEMD](https://matsci.org/t/group-virtual-fixed-problems-in-nonequilibrium-molecular-dynamics-nemd/49575).
10. matsci.org, [LAMMPS exiting without results](https://matsci.org/t/lammps-exiting-without-error-and-without-results/50684).

## Appendix: artifacts and reproduction

The [screening manifest](https://github.com/lan496/lamlet/blob/main/research/defects/cases.json)
contains all 20 source URLs, cited post numbers, and dependency limitations.
The [source receipt](https://github.com/lan496/lamlet/blob/main/research/defects/source-lock.json)
pins captured content hashes, and the
[collection report](https://github.com/lan496/lamlet/blob/main/research/defects/collection.json)
records the cache-backed audit. Native observations are published as the
[replay report](assets/defect-corpus/2026-11-09/report.json),
[frozen replay plan](assets/defect-corpus/2026-11-09/replays.json),
[runtime receipt](assets/defect-corpus/2026-11-09/runtime.json), and
[file inventory](assets/defect-corpus/2026-11-09/inventory.json).
Original downloaded input files are omitted from the public archive.

LAMMPS source revision
[`d291d572cf3c73a8a720774b78482a7c931304ca`](https://github.com/lammps/lammps/tree/d291d572cf3c73a8a720774b78482a7c931304ca)
was built with AppleClang 21.0.0 on macOS in Release mode, with MC, MOLECULE,
and RIGID enabled, and MPI, OpenMP, and shared libraries disabled. The checkout's
commit and clean status were checked separately; the replay runner records the
binary SHA-256 and declared source revision but does not inspect the checkout.
Unit constants are pinned from
[`src/update.cpp`](https://github.com/lammps/lammps/blob/d291d572cf3c73a8a720774b78482a7c931304ca/src/update.cpp).

The ten native attempts consumed about 29.3 s of measured process elapsed time,
excluding the build and collection. The first orientation command took 0.0118 s;
extrapolating it to nine remaining variants predicted 0.107 s. The heterogeneous
inputs invalidate that timing estimate: the averaging runs took about 4.6 and
4.3 s, and each full GCMC variant reached its 10 s cap. These durations describe
this run, not a performance benchmark.

```sh
python3 tools/defect_corpus.py audit
python3 tools/defect_replay.py verify --run docs/assets/defect-corpus/2026-11-09
python3 -m unittest discover -s tools -p test_defect_corpus.py
```

These checks require no network or native engine. They verify receipts and
recompute measurements from archived stdout, without reproducing the
simulations. To verify source-to-input transformations, retain the original
cache and add `--cache target/defect-pilot-20261109/sources` to replay verification.
The [collection protocol](https://github.com/lan496/lamlet/blob/main/research/defects/README.md)
documents fresh snapshots, input reconstruction, native execution, and
publication commands. A separate engine can be built from the pinned checkout:

```sh
cmake -S target/p3-runtime/source/cmake -B target/defect-runtime/build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_MPI=off -DBUILD_OMP=off \
  -DBUILD_SHARED_LIBS=off -DPKG_MC=yes -DPKG_MOLECULE=yes -DPKG_RIGID=yes
cmake --build target/defect-runtime/build --parallel 2
```

The source path refers to the existing pinned checkout used by
`tools/p3_runtime.py`; another checkout of the same revision may be supplied to
CMake. Fresh execution must use a new output directory and retains unsuccessful
outcomes. CI checks the public receipts and archived measurements offline; it
does not refetch forum posts or rerun these simulations.
