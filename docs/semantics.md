# Physical semantics

Physical rules are separate from expression types, resource ownership, and
command-state analysis. A rule returns satisfied, violated, or unknown over the
facts available at a use site. A finding describes only that rule's stated
condition. It does not establish physical correctness of the simulation.

The [lint rule reference](lint-rules.md) lists fourteen physical and idiom rules and
describes the state boundaries of 286 additional manual constraints.
Their [generated catalog](manual-rule-catalog.md) contains exact applicability
and immutable manual citations. Full-manual review and implementation remain incomplete.

## What the checker tracks

The checker follows active fix definitions, their effective declaration order,
selected temperature computes, explicit box-control components, and known
boundary settings. It evaluates command combinations at their use sites rather
than treating each line independently.

| Physical condition | Rule | Known facts needed |
| --- | --- | --- |
| Overlapping integration or temperature control | `LML-PHYS-001`, `003`, `006` | Reviewed fix roles and group overlap or coverage |
| Temperature velocity bias | `LML-PHYS-002` | Thermostat and thermo selections on the same known group |
| Periodicity of controlled box components | `LML-PHYS-004`, `005` | Known dimension, boundary, and pressure options |
| SHAKE/RATTLE fix order | `LML-PHYS-007`, `008` | Complete reviewed fix inventory and effective declaration order |
| Competing box controllers | `LML-PHYS-009` | Explicit cell components in reviewed barostat/deform grammar |
| Temperature normalization during atom exchange | `LML-PHYS-010` | Selected temperature definition, dynamic DOF policy, active exchange fix |

The remaining four rules cover command idioms and narrow argument constraints.
The [rule reference](lint-rules.md) records each rule's exact scope. A satisfied
finding establishes only its stated condition. A missing warning may also mean
the condition was outside the modeled subset.

## Temperature definitions

`LML-PHYS-002` compares known nvt/npt thermostat temperature
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
findings follow deterministic rule and ID ordering at each evaluated run.

## Competing box controllers

The simulation box is shared by all atom groups. Two fixes that control its
same component conflict even when their atom groups are disjoint. For example,
after system setup, this combination produces `LML-PHYS-009`:

```lammps
fix pressure all nph x 0 0 1
fix stretch all deform 1 x erate 0.001 units box
run 100
```

Lamlet tracks `x`, `y`, `z`, `xy`, `xz`, and `yz` as separate components.
`xy` is not the same component as `y`, although both require a periodic y axis
for pressure control. Known `iso` and `aniso` expand according to dimension;
`tri` names all six components. The diagnostic cites both fix definitions and
their controlled components.

Different explicit components satisfy this narrow conflict check. They do not
establish full compatibility of tilt scaling, integration, or the ensemble.
Non-default isochoric coupling, `deform/pressure`, `press/berendsen`, unknown
options, and opaque state changes remain unverified. The check runs at positive
literal `run` commands and uses the final active definitions. Removing a fix
before the run removes its contribution. Lamlet does not choose which controller
to remove or change.

## Temperature normalization when atom counts change

Temperature normalization depends on the population of the compute's group.
LAMMPS assumes constant population by default. Its
[`dynamic/dof` option](https://docs.lammps.org/compute_modify.html) requests
recomputation of atom count and degrees of freedom whenever temperature is
evaluated.

After system setup and definition of an `incoming` region, this example produces
`LML-PHYS-010` because `gas` uses the default `dynamic/dof no`:

```lammps
compute gas all temp
thermo_style custom step atoms c_gas
fix arrivals all deposit 100 1 10 12345 region incoming
run 1000
```

An explicit `compute_modify gas dynamic/dof yes` satisfies the normalization
policy check. The compiler suggests reviewing this setting and does not insert
it. An atom-exchange fix may perform no exchange during a particular run, so the
warning remains conditional.

The rule follows active `deposit`, `pour`, `gcmc`, and `evaporate` fixes at positive
literal runs. It checks known `temp` and `temp/com` computes selected by displayed
thermo `temp`, direct scalar `c_ID` thermo fields, and ordinary `nvt`/`npt`
temperature selections. Known internal default computes are included. Unused
explicit computes do not trigger this check. Unfix, uncompute, clear, selection
changes, and thermo-style resets affect subsequent findings.

Literal fix IDs, groups, and styles establish these roles even when later
arguments contain LAMMPS substitutions. For example,
`fix bath all nvt temp ${T} ${T} ${damping}` selects the known default
`bath_temp` compute. The checker neither evaluates the temperature nor assumes
that a substitution expands to one word. A substitution in the command name,
ID, group, or style makes the effect opaque.
LAMMPS performs substitution before splitting arguments into words
([parsing rules](https://docs.lammps.org/Commands_parse.html)).

[Region definitions](https://docs.lammps.org/region.html),
[restart output scheduling](https://docs.lammps.org/restart.html), and
parameterized equal/index variable definitions preserve these resource and
temperature-selection facts. This limited preservation makes no assertion about
the numerical trajectory. A literal `fix_modify bath dynamic/dof yes` preserves
the temperature selection but does not change the selected compute's
normalization policy. That policy requires `compute_modify bath_temp dynamic/dof
yes` ([fix_modify](https://docs.lammps.org/fix_modify.html)).

After an opaque effect such as `include`, previously observed thermostat and
exchange roles remain as Unknown. Subsequent compute definitions and selection
changes alone cannot establish that those fixes are still active. Explicit fix
definitions can restore known roles; unfix and clear remove them. The checker
does not invent identities for resources that have never been observed. Unknown
findings preserve uncertainty without warning or rejection.

Same-group and `all` relationships support a conditional overlap warning.
Distinct group names do not prove overlap; an unresolved DOF policy or compute
definition yields Unknown. Literal repeated `dynamic/dof` options use the final
value. Numeric `extra/dof`, including negative values, does not change that policy
or cause an extra COM correction. Modified computes remain Unknown to the separate
velocity-bias comparison rule. Arbitrary dynamic groups, lost atoms at boundaries,
other temperature styles, accelerated exchange styles, and indexed thermo fields
remain outside this check.

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
Freezing/setforce idioms, deform/flow bias, automatic
SHAKE/rigid double-correction detection, and complete group overlap analysis remain
unimplemented. The dynamic DOF check covers the stated atom-exchange subset;
Lamlet does not infer or apply numerical DOF corrections.
High-level declarations of physical intent and generated context commands remain
future work. Ordinary integrator conflicts produce a conditional lint warning.
The ownership checks in [scopes](scopes.md) can reject conflicting managed
integrators at the stricter resource boundary.
