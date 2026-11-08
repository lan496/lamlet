---
title: Lamlet
hide:
  - navigation
  - toc
---

<div class="lamlet-masthead">
  <span><strong>Lamlet</strong> / v0.1</span>
  <span>A compiler for LAMMPS inputs</span>
</div>

<div class="lamlet-hero" markdown>

<div markdown>

<span class="lamlet-eyebrow">Physical quantities. Explicit resources.</span>

# LAMMPS inputs,<br>with *units attached*. { .lamlet-headline }

Put `2 fs` in your source. Lamlet checks the dimension and emits `0.002` for
LAMMPS metal units. Add typed paths, managed resources, and independent stages
to the scripts you already use.
{ .lamlet-lead }

<div class="lamlet-actions" markdown>

[Get started](getting-started.md){ .md-button .md-button--primary }
[Watch the 32s demo](demo.md)
[GitHub](https://github.com/lan496/lamlet)

</div>

</div>

<aside class="lamlet-measure" aria-label="A time quantity of 2 femtoseconds compiles to 0.002 picoseconds in metal units">
  <p class="lamlet-caption">Source / Time</p>
  <p class="lamlet-measure-value"><span>2</span> fs</p>
  <hr>
  <p class="lamlet-caption">LAMMPS / units metal</p>
  <p class="lamlet-measure-value"><span>0.002</span> ps</p>
  <p class="lamlet-measure-note">A checked conversion.<br>No engine process required.</p>
</aside>

</div>

<section class="lamlet-section" markdown>

<span class="lamlet-index">01 / Source &amp; output</span>

## Small additions. Ordinary LAMMPS.

<div class="lamlet-code-pair" markdown>

<div markdown>

<p class="lamlet-caption">You write / simulation.lamlet</p>

```lamlet
@let dt = 2 fs
units metal
timestep @{dt}
@for steps in [0, 100] {
    run @{steps}
}
```

</div>

<div markdown>

<p class="lamlet-caption">LAMMPS receives / simulation.in</p>

```lammps
units metal
timestep 0.002
    run 0
    run 100
```

`check` and `build` work without LAMMPS.
Use `run` to launch the engine explicitly.

</div>

</div>

</section>

<section class="lamlet-section" markdown>

<span class="lamlet-index">02 / What becomes explicit</span>

## Check the assumptions in the input.

<div class="lamlet-feature" markdown>

<span class="lamlet-index" aria-hidden="true">a.</span>

### Physical quantities

<div markdown>

A timestep accepts a time quantity. `timestep @{300 K}` produces
`E_DIMENSION_MISMATCH`, with a source location and a suggested correction.

[Units and expressions](reference.md)

</div>
</div>

<div class="lamlet-feature" markdown>

<span class="lamlet-index" aria-hidden="true">b.</span>

### Files and resources

<div markdown>

Resolve typed input files before execution. Enumerate dependencies and distinguish
an input file from a directory or an output destination.

[Paths and projects](resources.md)

</div>
</div>

<div class="lamlet-feature" markdown>

<span class="lamlet-index" aria-hidden="true">c.</span>

### Resource lifetimes

<div markdown>

Declare `@fix`, `@compute`, and `@dump` inside `@scope`. Lamlet checks managed
references and emits cleanup in reverse order.

[Scoped resources](scopes.md)

</div>
</div>

<div class="lamlet-feature" markdown>

<span class="lamlet-index" aria-hidden="true">d.</span>

### Independent stages

<div markdown>

Build a selected stage with explicit arguments, a dedicated working directory,
and an execution manifest.

[Modules and stages](modules-and-stages.md)

</div>
</div>

</section>

<section class="lamlet-section" markdown>

<span class="lamlet-index">03 / Gradual adoption</span>

## Start with the script you have.

Lamlet preserves a legacy file's bytes and accepts commands outside its partial
schemas as unverified. Add units, typed paths, or structured scopes to the parts
you want checked. The compiler is written in Rust. Checking and compilation
need no LAMMPS installation, Python, or network.

<div class="lamlet-corpus" markdown>

<span class="lamlet-corpus-number">998</span>

<div markdown>

**Selected upstream inputs. Every output byte preserved.**

The recorded corpus has zero standard or strict rejections at pinned revisions.
This measures preservation and false rejections on that corpus.
[Scope and reproduction procedure](corpus.md).

</div>

</div>

</section>

<section class="lamlet-section" markdown>

<span class="lamlet-index">04 / Feedback for agents</span>

## Check. Inspect. Repair.

```sh
lamlet check simulation.lamlet --format json
```

One JSON object, stable diagnostic codes, source spans, and explicit exit codes
let an agent inspect a problem and check its repair. Dependencies, command
schemas, stage manifests, and target capabilities are also available as JSON.
[Read the CLI reference](reference.md).

!!! note "What a passing check means"
    Lamlet reports errors within its implemented checks. Unsupported legacy
    commands remain unverified. A passing check does not establish scientific
    correctness, convergence, or successful execution. See the
    [current limitations](design.md) and [independent evaluation plan](agent-evaluation.md).

</section>
