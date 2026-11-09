# Constraint residuals under SHAKE and RATTLE fix ordering

## Abstract

This controlled calculation tests whether fix-order diagnostics correspond to
measurable constraint errors in a rotating dimer. Six serial LAMMPS runs compare
both declaration orders for SHAKE with Langevin, SHAKE with setforce, and RATTLE
with NVT. Every run exits successfully. Reversing SHAKE increases the maximum
relative bond-length error from approximately machine precision to 0.00164 with
Langevin and 0.166 with setforce. Reversing RATTLE increases the radial relative
velocity from below 6 × 10⁻¹⁷ to 0.00293 in reduced units. The correctly ordered
RATTLE/NVT case nevertheless exceeds the prescribed bond-length threshold.
These results support the narrow ordering warnings and demonstrate why static
acceptance and process completion do not establish constraint accuracy.

## Introduction

LAMMPS documents that SHAKE should follow fixes that modify forces on constrained
atoms, while RATTLE should follow other integration fixes [1]. The constraints
involve both bond length and the velocity component along each constrained bond.
Lamlet implements conditional warnings for reviewed combinations as
`LML-PHYS-007` and `LML-PHYS-008`. This calculation asks whether reversing those
orders changes independently measured residuals, and whether a successful exit
is sufficient to detect the resulting errors.

## Methods

The model consists of two unit-mass atoms connected by one unit-length harmonic
bond with coefficient 100 in LAMMPS reduced units. Initial positions are
(−0.5, 0, 0) and (0.5, 0, 0); velocities are (0, 0.3, 0) and (0, −0.3, 0).
The periodic box extends from −5 to 5 in each direction. Pair interactions use
`pair_style zero`. Each run integrates 200 steps with timestep 0.005 and
constraint tolerance 10⁻¹⁰, with up to 100 solver iterations per step.

The SHAKE cases use NVE integration and either Langevin at temperature 0.1,
damping 0.1, seed 12345, or setforce with all components zero. The RATTLE cases use
NVT at temperature 0.1 and damping 0.1. Each pair differs only in declaration
order. Correct order means the constraint fix follows the reviewed modifier or
integrator. This designation describes the ordering condition alone.

An independent Python reader computes the maximum relative bond-length error
`abs(|r₂ − r₁| / d₀ − 1)` and maximum radial relative velocity
`abs((r₂ − r₁) · (v₂ − v₁)) / |r₂ − r₁|`. It reads unwrapped positions and
velocities from all 201 frames, including the initial state, and rejects missing
frames, duplicate IDs, malformed columns, and nonfinite values. It does not use
Lamlet findings to compute or classify the residuals. The prespecified threshold
is 10⁻⁸ for each quantity. SHAKE is assessed on distance alone; RATTLE is assessed
on both quantities. This threshold is an engineering test criterion rather than
a guarantee derived from the iterative solver tolerance.

## Results

Table 1 compares the maximum residuals and the ordering diagnostic. All six
processes returned exit code zero and produced complete trajectories. Lamlet
warned for each reversed case and for none of the corresponding correct-order
cases. Only the two correctly ordered SHAKE cases met every applicable threshold.

**Table 1.** Maximum residuals over the initial frame and 200 integration steps.
Velocity is reported in reduced units. Values are rounded from the archived
measurements; SHAKE does not impose the velocity constraint.

| Constraint and other fix | Order | Bond-length error | Radial velocity | Applicable thresholds met | Ordering warning |
| --- | --- | ---: | ---: | --- | --- |
| SHAKE / Langevin | Correct | 2.22 × 10⁻¹⁶ | 8.55 × 10⁻⁴ | Yes | No |
| SHAKE / Langevin | Reversed | 1.64 × 10⁻³ | 0.286 | No | Yes |
| SHAKE / setforce | Correct | 2.22 × 10⁻¹⁶ | 2.04 × 10⁻¹⁴ | Yes | No |
| SHAKE / setforce | Reversed | 0.166 | 0.309 | No | Yes |
| RATTLE / NVT | Correct | 6.87 × 10⁻⁷ | 5.55 × 10⁻¹⁷ | No | No |
| RATTLE / NVT | Reversed | 1.46 × 10⁻⁵ | 2.93 × 10⁻³ | No | Yes |

## Discussion

The reversed SHAKE cases illustrate errors that normal process completion does
not reveal. The RATTLE comparison separates the two constraints: correct ordering
removes the radial velocity to numerical precision, while its bond-length error
remains above the chosen threshold. This experiment does not determine the cause
of that remaining position error. Changing the threshold after observing it would
conceal this limitation, so the original criterion and all outcomes are retained.

The cases test one two-atom model, one timestep, one seed, one serial build, and
ordinary Verlet integration. They do not establish performance across molecular
topologies, integrators, parallel executions, or custom fixes. In particular, they
do not imply a general SHAKE/NVT ordering restriction. Unknown group overlap and
unreviewed fix callbacks remain outside the static rule's guarantees. No agent
trial or independent expert label review was conducted in this calculation.

## Conclusions

Fix order changes measured constraint residuals even when every simulation exits
normally. The controlled results support warnings for these specific reversed
orders. Correct ordering alone is insufficient to certify numerical accuracy.

## References

1. LAMMPS, [SHAKE and RATTLE](https://docs.lammps.org/fix_shake.html),
   documentation version 30Sep2026.
2. LAMMPS, [fix declaration and replacement order](https://docs.lammps.org/fix.html).
3. LAMMPS, [Nose–Hoover integration](https://docs.lammps.org/fix_nh.html).

## Appendix: computational details and reproduction

The native source is pinned to
[`d291d572cf3c73a8a720774b78482a7c931304ca`](https://github.com/lammps/lammps/tree/d291d572cf3c73a8a720774b78482a7c931304ca).
The build uses AppleClang 21.0.0 on macOS, MPI and OpenMP disabled, and the
MOLECULE and RIGID packages enabled. The binary and checker hashes, complete input
files, stdout, stderr, trajectories, and checker diagnostics are available in the
[report](assets/evaluation/2026-11-09-constraints/report.json) and
[file inventory](assets/evaluation/2026-11-09-constraints/inventory.json).
The independently measured first native case took 0.0118 s; its estimate for the
remaining five native runs was 0.0592 s. These durations exclude build and checker
costs and do not constitute a performance benchmark.

The fixtures and criteria are in `research/physical-semantics/`. Build a separate
engine from the pinned checkout, then collect a fresh archive:

```sh
cmake -S target/p3-runtime/source/cmake -B target/constraint-runtime/build -G Ninja \
  -DBUILD_MPI=off -DBUILD_OMP=off -DBUILD_SHARED_LIBS=off \
  -DPKG_RIGID=on -DPKG_MOLECULE=on -DCMAKE_BUILD_TYPE=Release
cmake --build target/constraint-runtime/build --parallel 2
cargo build --release --locked --jobs 2
python3 tools/constraint_validation.py \
  --source target/p3-runtime/source \
  --engine target/constraint-runtime/build/lmp --checker target/release/lamlet \
  --output target/constraint-reproduction
python3 tools/constraint_validation.py \
  --verify docs/assets/evaluation/2026-11-09-constraints
```

The source path refers to the existing pinned checkout used by
`tools/p3_runtime.py`. A different checkout may be supplied through `--source`
and the CMake source argument. The collector verifies its commit, refuses an
existing output directory, and retains unsuccessful engine outcomes. Archive
verification checks file hashes and recomputes every available trajectory
measurement without launching LAMMPS. Earlier agent-evaluation archives remain
unchanged.
