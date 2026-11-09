# Physical semantics

Physical rules are separate from expression types, resource ownership, and
command-state analysis. A rule returns satisfied, violated, or unknown over the
facts available at a use site. A finding describes only that rule's stated
condition. It does not establish physical correctness of the simulation.

The [lint rule reference](lint-rules.md) lists twelve physical and idiom rules and
describes the state boundaries of 286 additional manual constraints.
Their [generated catalog](manual-rule-catalog.md) contains exact applicability
and immutable manual citations. Full-manual review and implementation remain incomplete.

The initial rule, **LML-PHYS-002**, compares known nvt/npt thermostat temperature
definitions with the displayed thermo temp at a plain `run`. It models default
thermostat computes, `compute ... temp` and `temp/com`, literal fix_modify temp,
thermo_modify temp, and thermo_style resets. Equal known definitions on the same
literal group satisfy this narrow condition. Different known velocity-bias styles
on that group produce a conditional warning, even in strict mode. Different or
unknown groups, unsupported compute styles, modified DOF settings, opaque effects,
and thermo output without the generic temp field produce Unknown findings.

```lamlet
compute com all temp/com
fix thermostat all nvt temp 300 300 0.1
fix_modify thermostat temp com
run 10000
```

The rule reports the temp/com thermostat and default temp thermo definitions as
evidence. The difference may be intentional. Suggested actions are to select the
same compute explicitly or retain both after reviewing the intended bias. The
compiler never changes either selection. There is no acknowledgement directive or
autofix in this version.

`check --format json` includes `physical_findings` when a rule was evaluated;
violated conditions also produce a warning with structured `evidence` and
`suggested_actions`. Satisfied and Unknown outcomes are not compile errors.
`schema --format json` exposes rule IDs, applicability, category, confidence,
autofix policy, documentation version, references, and limitations. Deterministic
findings are emitted at each evaluated run, in thermostat-ID order.

## Physical assumptions and limits

The [controlled SHAKE/RATTLE calculation](constraint-validation.md) measures
bond-length and radial-velocity residuals for both fix orders in six short native
LAMMPS runs. All runs exit successfully; the residuals expose errors that exit
codes miss. The correctly ordered RATTLE/NVT case also exceeds the chosen distance
threshold. The report retains this limitation and all raw measurements.

The [LAMMPS temp definition](https://docs.lammps.org/compute_temp.html) subtracts
the system dimensionality and applicable fix DOFs by default. In 3D the default
translation correction is already three; Lamlet does not add another correction
when fix momentum appears. This denominator correction is distinct from
[temp/com velocity-bias removal](https://docs.lammps.org/compute_temp_com.html).
Lamlet does not equate periodic momentum removal with a holonomic constraint.
LAMMPS handles supported shake/rigid DOF reductions in its temperature computes.

The rule registry is based on the linked official documentation, identified as
30Sep2026. These conditional facts do not certify arbitrary LAMMPS versions or
custom builds. Engine capability checks describe availability separately.

The engine deliberately has no general rule language or external rule packages.
Freezing/setforce idioms, deform/flow bias, dynamic atom-count DOFs, automatic
SHAKE/rigid double-correction detection, and complete group overlap analysis remain
unimplemented. The [compute_modify documentation](https://docs.lammps.org/compute_modify.html)
describes dynamic/dof and extra/dof, but Lamlet does not infer settings for them.
High-level declarations of physical intent and generated context commands remain
future work. Ordinary integrator conflicts produce a conditional lint warning.
The ownership checks in [scopes](scopes.md) can reject conflicting managed
integrators at the stricter resource boundary.
