# LAMMPS lint rules

Lamlet has ten physical and idiom rules derived from the official LAMMPS manual.
They emit warnings with source evidence and suggested actions. Warnings remain
nonfatal in strict mode. The compiler does not reorder commands, change physical
parameters, or insert corrections in response to a warning.

## Rule catalog

<div class="lamlet-rule-catalog" markdown>

| Code | Condition | Static coverage | Manual |
| --- | --- | --- | --- |
| `LML-PHYS-001` | Multiple time integrators may update the same atoms. | Plain positive literal `run`; `nve`, `nvt`, `npt`, and `nph` on the same literal group, or `all` and another group. | [nve](https://docs.lammps.org/fix_nve.html), [nvt/npt/nph](https://docs.lammps.org/fix_nh.html) |
| `LML-PHYS-002` | Thermostat and displayed thermo temperature use different velocity-bias definitions. | Known `temp` and `temp/com` on the same group; see [physical semantics](semantics.md). | [temp](https://docs.lammps.org/compute_temp.html), [temp/com](https://docs.lammps.org/compute_temp_com.html) |
| `LML-PHYS-003` | Multiple fixes control temperature on potentially overlapping atoms. | `nvt`, `npt`, `langevin`, `temp/rescale`, and `temp/berendsen`; same overlap limits as `LML-PHYS-001`. | [nvt/npt/nph](https://docs.lammps.org/fix_nh.html), [langevin](https://docs.lammps.org/fix_langevin.html) |
| `LML-PHYS-004` | A barostatted dimension is nonperiodic. | Known boundary and `npt` or `nph` pressure keywords; tilt requires periodicity in its second axis. Two-dimensional `iso` and `aniso` omit z. | [nvt/npt/nph restrictions](https://docs.lammps.org/fix_nh.html#restrictions) |
| `LML-PHYS-005` | A two-dimensional simulation has nonperiodic z boundaries. | Known dimension and boundary at `create_box` or `read_data`. | [boundary](https://docs.lammps.org/boundary.html), [dimension](https://docs.lammps.org/dimension.html) |
| `LML-PHYS-006` | Langevin force control has no known integrator for its group. | Positive literal `run`, complete fix inventory, and reviewed fix roles; partial group coverage remains Unknown. | [langevin](https://docs.lammps.org/fix_langevin.html) |
| `LML-LINT-001` | `thermo_style` resets earlier `thermo_modify` settings. | A known modification followed by a style command. A deliberate reset is valid. | [thermo_style](https://docs.lammps.org/thermo_style.html) |
| `LML-LINT-002` | A random seed is not positive. | Integer literals in `velocity ... create` and `fix ... langevin`. | [velocity](https://docs.lammps.org/velocity.html), [langevin](https://docs.lammps.org/fix_langevin.html) |
| `LML-LINT-003` | A combined region specifies fewer than two regions. | Integer literal counts in `region ... union` and `intersect`. | [region](https://docs.lammps.org/region.html) |
| `LML-LINT-004` | `velocity ... set` replaces components after thermal initialization. | Numeric components after a known `velocity ... create`, unless `sum yes` is explicit. Replacement can be intentional. | [velocity](https://docs.lammps.org/velocity.html) |

</div>

The integrator and thermostat checks recognize the standard `gpu`, `intel`, `kk`,
`omp`, and `opt` suffixes on the listed styles. Other styles have unreviewed roles.
Distinct group names do not prove disjointness. Empty groups and specialized
integration schemes require review even when the group names match.

A deliberate velocity reset is valid. For example, freezing a subgroup after
adding random motion to `all` can trigger `LML-LINT-004`. The warning describes
the replacement and does not establish that the author's intent is wrong.

## Diagnostics and queries

```sh
lamlet check input.in --format json
lamlet check input.in --strict --format json
lamlet schema --format json
lamlet schema fix/nvt --format json
```

`schema` includes the existing temperature rule in `physical_rules`, nine
additional rules in `lint_rules`, and manual provenance in `documentation`.
A query matches a schema name, a manual index alias such as `fix/nvt`, or a source
basename such as `fix_nh`. An unmatched query returns an empty manual-page list.
The documentation index does not assert that a command has an operand schema.

Rule metadata includes applicability, category, confidence, references, the
reference version, limitations, and `autofix: false`. A matching manual page has
its source hash, section line ranges, and an immutable GitHub source URL.

This input produces `LML-PHYS-001` at the `run` command:

```lammps
fix first all nve
fix second all nvt temp 300 300 0.1
run 10
```

The JSON diagnostic identifies both fix declarations in `evidence`. The matching
entry in `physical_findings` has outcome `violated`. A build still emits the
original bytes and returns exit 0 unless another check produces an error.
Removing the first fix before declaring the second avoids this warning.

## Unknown and state boundaries

Every evaluated condition returns `satisfied`, `violated`, or `unknown`.
Unknown outcomes do not produce warning diagnostics. A condition that was never
evaluated is unverified; absence of a finding is not a satisfied result.

Opaque commands, ambiguous runtime substitutions, and unstructured control flow
invalidate tracked facts. Unknown fix styles prevent claims that an integrator
is absent. Unknown pressure options prevent the barostat rule from assigning
invented argument boundaries. `run 0` does not trigger integration checks.

Group changes, atom creation or deletion, and later velocity initialization
discard earlier thermal-initialization evidence. A numeric `velocity set` can
therefore remain unverified even when replacement occurs at runtime.

These checks do not infer the intended temperature, pressure, geometry, impact
speed, heating rate, or total simulation duration from bare numbers. They do not
establish group membership, triclinic-box compatibility, pressure-coupling
parameter equality, checkpoint fidelity, or numerical correctness. Lamlet does
not add a COM degrees-of-freedom correction: LAMMPS temperature computes already
apply their documented default correction.

## Official manual coverage

The catalog is pinned to the official LAMMPS release source at
[`0165101ce505734e8fb249694b22e295a7ab4182`](https://github.com/lammps/lammps/tree/0165101ce505734e8fb249694b22e295a7ab4182),
version **30 Sep 2026**. It contains every `.rst` file under `doc/src`:

| Indexed material | Count |
| --- | ---: |
| Source documents, including fragments | 1,001 |
| Source bytes | 8,720,158 |
| Paragraphs selected for constraint review | 6,726 |

Selection uses normative terms such as "must," "required," and "not supported."
Selected paragraphs are unreviewed candidates. They are not executable lint rules,
and their count is neither a constraint count nor a semantic coverage measure.
Historical HTML, images, and generated autodoc bodies are outside this index.
The source documents that contain autodoc directives are included.

Full text and candidate paragraphs remain in ignored `target/lammps-manual/`.
The committed catalog stores provenance and navigation metadata. Indexing all
source documents does not mean that all prose has been semantically reviewed or
implemented. Most manual constraints remain unimplemented. The ten rules above
are the current executable physical and idiom coverage; command schemas and
ownership checks are separate.

The catalog generator is `tools/lammps_docs.py`. It requires Python and a local
LAMMPS Git repository containing the pinned commit and its documentation blobs.
It does not change the checkout or fetch missing objects explicitly. Git partial
clones can fetch missing blobs during archive creation.

```sh
git clone --filter=blob:none --no-checkout https://github.com/lammps/lammps.git target/lammps-doc-source
cd target/lammps-doc-source
git fetch --depth 1 origin 0165101ce505734e8fb249694b22e295a7ab4182
cd ../..
python3 tools/lammps_docs.py --checkout target/lammps-doc-source
python3 -m unittest discover -s tools -p 'test_lammps_docs.py'
```

The generated JSON has no timestamps or machine-specific paths. Repeating the
command with the same source produces the same catalog. Updating the pin requires
reviewing rule applicability and rerunning positive cases, counterexamples, and
legacy compatibility checks. Prose extraction alone never enables a rule.
