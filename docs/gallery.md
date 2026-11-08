---
hide:
  - toc
---

# Gallery

These examples compare Lamlet inputs with their actual compiler output. Repetition provides the largest reduction. Types and contracts sometimes add code.

<div class="lamlet-gallery-summary" markdown>

| Example | Lamlet lines | LAMMPS lines | Line reduction | Characters, Lamlet / LAMMPS |
| --- | ---: | ---: | ---: | ---: |
| [Five temperatures, one recipe](#temperature-sweep) | 15 | 45 | 67% fewer | 458 / 1165 |
| [One operation, four typed calls](#reusable-phase) | 17 | 29 | 41% fewer | 514 / 620 |
| [Three resources, one scope](#resource-lifetimes) | 12 | 13 | 8% fewer | 300 / 309 |
| [Units make intent explicit](#explicit-units) | 7 | 6 | 17% more | 150 / 135 |

</div>

All setup, declarations, and braces are counted. Each example uses the included demonstration structure; generated paths assume execution from the repository root. [Comparison method and limits](#comparison-method).

## Five temperatures, one recipe { #temperature-sweep }

A finite loop calls the same typed operation at five temperatures. The loop clears LAMMPS state before each call. The function reloads the structure and releases its thermostat. These are separate simulations in one LAMMPS process; stages provide separate-process execution.

<div class="lamlet-code-pair" markdown>

<div markdown>

<p class="lamlet-caption">Lamlet · 15 lines · 458 characters</p>

```lamlet
@module experiments {
    @fn sample(target: Temperature) requires [] ensures [AtomsDefined, ModelDefined] {
        units metal
        atom_style atomic
        read_data examples/gallery/structure.data
        pair_style lj/cut 8.5
        pair_coeff * * 0.0103 3.4
        @fix bath all nvt temp @{target} @{target} @{100 fs}
        run 10000
    }
}
@for target in [300 K, 400 K, 500 K, 600 K, 700 K] {
    clear
    @call experiments.sample(target)
}
```

</div>

<div markdown>

<p class="lamlet-caption">Generated LAMMPS · 45 lines · 1165 characters</p>

```lammps
    clear
        units metal
        atom_style atomic
        read_data examples/gallery/structure.data
        pair_style lj/cut 8.5
        pair_coeff * * 0.0103 3.4
fix bath all nvt temp 300 300 0.1
        run 10000
unfix bath
    clear
        units metal
        atom_style atomic
        read_data examples/gallery/structure.data
        pair_style lj/cut 8.5
        pair_coeff * * 0.0103 3.4
fix bath all nvt temp 400 400 0.1
        run 10000
unfix bath
    clear
        units metal
        atom_style atomic
        read_data examples/gallery/structure.data
        pair_style lj/cut 8.5
        pair_coeff * * 0.0103 3.4
fix bath all nvt temp 500 500 0.1
        run 10000
unfix bath
    clear
        units metal
        atom_style atomic
        read_data examples/gallery/structure.data
        pair_style lj/cut 8.5
        pair_coeff * * 0.0103 3.4
fix bath all nvt temp 600 600 0.1
        run 10000
unfix bath
    clear
        units metal
        atom_style atomic
        read_data examples/gallery/structure.data
        pair_style lj/cut 8.5
        pair_coeff * * 0.0103 3.4
fix bath all nvt temp 700 700 0.1
        run 10000
unfix bath
```

</div>

</div>

[Source](https://github.com/lan496/lamlet/blob/main/examples/gallery/temperature-sweep.lamlet) · [Generated input](assets/gallery/temperature-sweep.in)

## One operation, four typed calls { #reusable-phase }

The function owns its compute and fix. Each call checks the temperature type and the declared state contracts, then releases both resources. The full definition, contracts, setup, and calls count toward the total.

<div class="lamlet-code-pair" markdown>

<div markdown>

<p class="lamlet-caption">Lamlet · 17 lines · 514 characters</p>

```lamlet
units metal
atom_style atomic
read_data examples/gallery/structure.data
pair_style lj/cut 8.5
pair_coeff * * 0.0103 3.4
@module dynamics {
    @fn sample(target: Temperature) requires [AtomsDefined, ModelDefined] ensures [AtomsDefined, ModelDefined] {
        @compute T all temp
        @fix bath all nvt temp @{target} @{target} @{100 fs}
        fix_modify bath temp T
        run 10000
    }
}
@call dynamics.sample(300 K)
@call dynamics.sample(400 K)
@call dynamics.sample(500 K)
@call dynamics.sample(600 K)
```

</div>

<div markdown>

<p class="lamlet-caption">Generated LAMMPS · 29 lines · 620 characters</p>

```lammps
units metal
atom_style atomic
read_data examples/gallery/structure.data
pair_style lj/cut 8.5
pair_coeff * * 0.0103 3.4
compute T all temp
fix bath all nvt temp 300 300 0.1
        fix_modify bath temp T
        run 10000
unfix bath
uncompute T
compute T all temp
fix bath all nvt temp 400 400 0.1
        fix_modify bath temp T
        run 10000
unfix bath
uncompute T
compute T all temp
fix bath all nvt temp 500 500 0.1
        fix_modify bath temp T
        run 10000
unfix bath
uncompute T
compute T all temp
fix bath all nvt temp 600 600 0.1
        fix_modify bath temp T
        run 10000
unfix bath
uncompute T
```

</div>

</div>

[Source](https://github.com/lan496/lamlet/blob/main/examples/gallery/reusable-phase.lamlet) · [Generated input](assets/gallery/reusable-phase.in)

## Three resources, one scope { #resource-lifetimes }

Acquisition order determines cleanup: undump, unfix, then uncompute. Adding another managed declaration also adds its cleanup. Plain LAMMPS fix, compute, and dump commands keep their manual lifetimes.

<div class="lamlet-code-pair" markdown>

<div markdown>

<p class="lamlet-caption">Lamlet · 12 lines · 300 characters</p>

```lamlet
units metal
atom_style atomic
read_data examples/gallery/structure.data
pair_style lj/cut 8.5
pair_coeff * * 0.0103 3.4
@scope {
    @compute T all temp
    @fix bath all nvt temp 300 300 0.1
    fix_modify bath temp T
    @dump traj all custom 100 trajectory.lammpstrj id type x y z
    run 10000
}
```

</div>

<div markdown>

<p class="lamlet-caption">Generated LAMMPS · 13 lines · 309 characters</p>

```lammps
units metal
atom_style atomic
read_data examples/gallery/structure.data
pair_style lj/cut 8.5
pair_coeff * * 0.0103 3.4
compute T all temp
fix bath all nvt temp 300 300 0.1
    fix_modify bath temp T
dump traj all custom 100 trajectory.lammpstrj id type x y z
    run 10000
undump traj
unfix bath
uncompute T
```

</div>

</div>

[Source](https://github.com/lan496/lamlet/blob/main/examples/gallery/resource-lifetimes.lamlet) · [Generated input](assets/gallery/resource-lifetimes.in)

## Units make intent explicit { #explicit-units }

In metal units, 2 fs becomes 0.002 ps. This small example needs more Lamlet lines and characters. Its benefit is dimensional checking and explicit units.

<div class="lamlet-code-pair" markdown>

<div markdown>

<p class="lamlet-caption">Lamlet · 7 lines · 150 characters</p>

```lamlet
@let dt = 2 fs
units metal
timestep @{dt}
atom_style atomic
read_data examples/gallery/structure.data
pair_style lj/cut 8.5
pair_coeff * * 0.0103 3.4
```

</div>

<div markdown>

<p class="lamlet-caption">Generated LAMMPS · 6 lines · 135 characters</p>

```lammps
units metal
timestep 0.002
atom_style atomic
read_data examples/gallery/structure.data
pair_style lj/cut 8.5
pair_coeff * * 0.0103 3.4
```

</div>

</div>

[Source](https://github.com/lan496/lamlet/blob/main/examples/gallery/explicit-units.lamlet) · [Generated input](assets/gallery/explicit-units.in)

## Comparison method { #comparison-method }

Code lines exclude blank lines and full-line comments. Characters include all whitespace and final newlines. No boilerplate is removed from either side. These are source-size comparisons, not runtime or scientific-accuracy measurements.

The baseline is Lamlet's expanded output. Handwritten LAMMPS can also reduce repetition with variables, `include`, and `jump`; this gallery does not compare against the shortest possible LAMMPS program.

Each input includes setup and reads the same eight-atom [demonstration structure](https://github.com/lan496/lamlet/blob/main/examples/gallery/structure.data) with masses and initial velocities. The model illustrates compiler behavior; no MD is run to produce this gallery, and the examples are not validated materials calculations. Unsupported operand schemas can still produce Unknown diagnostics.

## Reproduce the comparison

The gallery is generated by the release CLI. CI recompiles all four examples and checks the displayed code, counts, and downloadable files.

```sh
cargo build --release --locked
python3 tools/gallery.py --check
```

After an intentional example or compiler change, run `python3 tools/gallery.py` to refresh the page and [machine-readable measurements](assets/gallery/report.json). The generator never launches LAMMPS.

For the syntax and its guarantees, see [scoped resources](scopes.md), [modules and stages](modules-and-stages.md), and the [language reference](reference.md).
