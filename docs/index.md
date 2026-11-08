---
title: Lamlet
hide:
  - navigation
  - toc
---

<div class="lamlet-home" markdown>

<div class="lamlet-intro" markdown>

# Make LAMMPS workflows *explicit and checkable*. { .lamlet-headline }

Lamlet is a Rust compiler for existing LAMMPS scripts. Add typed units and files,
managed resources, and independent stages. Get standard LAMMPS input and
structured diagnostics before execution.
{ .lamlet-lead }

<div class="lamlet-actions" markdown>

[Get started](getting-started.md){ .md-button .md-button--primary }
[Gallery](gallery.md)
[32s demo](demo.md)
[CLI reference](reference.md)
[GitHub](https://github.com/lan496/lamlet)

</div>

</div>

<div class="lamlet-code-pair" markdown>

<div markdown>

<p class="lamlet-caption">Source / compiler excerpt</p>

```lamlet
@let dt = 2 fs
units metal
timestep @{dt}
@scope {
    @fix integrate all nve
    run 100
}
```

</div>

<div markdown>

<p class="lamlet-caption">Generated / ordinary LAMMPS</p>

```lammps
units metal
timestep 0.002
fix integrate all nve
    run 100
unfix integrate
```

</div>

</div>

This excerpt converts the timestep and generates cleanup. The
[quickstart](getting-started.md) includes complete inputs. `check` and `build`
do not launch LAMMPS.
{ .lamlet-example-note }

<div class="lamlet-details" markdown>

<section markdown>

## What Lamlet checks

| Interface | Supported checks |
| --- | --- |
| [Units and expressions](reference.md) | Physical dimensions, typed arithmetic, and conversion to `metal` or `real` units. |
| [Files and outputs](resources.md) | Input existence, resource kinds, dependency paths, and output collisions. |
| [Resource lifetimes](scopes.md) | `@scope`, `@fix`, `@compute`, `@dump`, and `@drop` track ownership and generate cleanup. |
| [Modules and stages](modules-and-stages.md) | Typed calls, state contracts, explicit artifacts, and separate execution directories. |
| [LAMMPS targets](capabilities.md) | Check style availability against a selected build profile. |
| [Physical semantics](semantics.md) | Conditional temperature-consistency warnings with evidence and suggested actions. |

</section>

<section markdown>

## Use the CLI

```sh
lamlet check simulation.lamlet --format json
lamlet build simulation.lamlet
lamlet inspect dependencies simulation.lamlet --format json
```

JSON diagnostics include stable codes, source spans, and exit codes for editors
and [coding agents](agent-evaluation.md). Checking works offline without LAMMPS.

### Existing scripts

Legacy compilation preserves bytes. The recorded corpus has **998 identical
round trips and zero static rejections** at pinned revisions.
[Corpus and coverage](corpus.md).

</section>

</div>

**Validation scope.** Unsupported legacy commands remain unverified. A passing
check does not establish scientific correctness, convergence, or successful
execution. [Current limitations](design.md).
{ .lamlet-limits }

</div>
