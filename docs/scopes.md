# Scoped resources

`@scope` gives explicitly declared resources a lexical lifetime. It emits no
LAMMPS scope command. The compiler emits acquisition commands and inserts cleanup
in reverse acquisition order:

```lamlet
@scope {
    @compute T all temp
    @fix thermostat all nvt temp 300 300 0.1
    @dump traj all custom 100 "traj.lammpstrj" id x y z
    run 10000
}
```

The generated tail is `undump traj`, `unfix thermostat`, `uncompute T`.
Only `@fix`, `@compute`, and `@dump` are managed. An ordinary `fix` inside the
same block remains live after the block until an ordinary `unfix` destroys it.
LAMMPS state lost on unfix is not restored by Lamlet.

Declarations require a surrounding scope and a literal alphanumeric/underscore
ID. They use ordinary LAMMPS arguments, including `@{...}` interpolation. Bare
high-level function calls are not interpreted. Blocks can nest, contain lexical
bindings, and contain static `@for` loops. A declaration belongs to the surrounding
`@scope`, not to an implicit per-iteration scope. Put a nested `@scope` inside a
loop to acquire and release the same ID once per iteration.

```lamlet
@scope {
    @fix thermostat all nvt temp 300 300 0.1
    run 10000
    @drop thermostat
    @fix production all nve
    run 100000
}
```

`@drop ID` emits the matching unfix/uncompute/undump immediately and removes it
from later cleanup. It can release a resource acquired earlier than other live
resources, provided none depends on it. An inner block can explicitly drop an
outer resource. An ID shared by several live managed resource kinds is ambiguous
and cannot be dropped by this syntax. A released ID can be acquired again.

## Ownership and dependencies

The compiler rejects manual destruction or replacement of a live managed ID,
a duplicate managed acquisition of a known live ID, a second drop, and known
references to a resource after its managed lifetime. It tracks c_ID / f_ID in
covered numeric expressions and attributes, direct modify targets, and temperature
or pressure compute references in fix_modify / thermo_modify. Dependency tracking
is partial; arbitrary style-specific argument meanings are not inferred.

Known live fix/compute/dump consumers prevent early destruction of their
providers. A legacy consumer that outlives a managed provider is also rejected;
manage the consumer in the same or an inner scope, or remove it before the
provider's scope ends. Known equal/atom/vector variable formulas and persistent thermo temp/press
settings also keep their referenced resources live; delete the variable or reset
the thermo setting before cleanup. Transitive variable dependencies and arbitrary
style-specific callbacks do not yet form a complete dependency graph.

Two known `nve`, `nvt`, or `npt` integrators on the same literal group conflict
when at least one is managed. Legacy-only combinations retain existing behavior.
Unknown group overlap and accelerator/style variants are not inferred. Constraint
and diagnostic fixes are not classified as time integrators merely because they
are fixes. This is a narrow effect check, not physical validation of all fixes.

## Structured execution requirement

These are compile-time insertion scopes, not runtime RAII. `jump`, `quit`, and
`clear` are rejected inside a scope, even before its first declaration. Direct
embedded exits in if/run command strings are also rejected. The current compiler
does not prove intra-scope jump targets. Nested runtime conditionals require
control-flow analysis and are rejected in a scope.

Include, Python, ambiguous commands, opaque commands, and runtime run callbacks
warn with W_SCOPE_UNVERIFIED. Strict mode rejects these with E_SCOPE_UNVERIFIED;
ordinary opaque legacy commands outside scopes remain accepted. This explicitly
marks the limit of static ownership guarantees. The compiler does not rewrite
jump targets or synthesize cleanup on unstructured exits.

Generated cleanup source maps point to the closing brace or explicit `@drop`,
with the enclosing static loop expansion context. Acquisition maps retain the
original command span. Resource expansion participates in the line limit.

The earlier `@with` proposal was replaced by this flat scope design. `@simulation`,
`@resources`, variable/group lifetimes, temporary setting restoration, generated
IDs, high-level nvt/trajectory APIs, and user-defined contexts are not implemented.
They can build on these ownership operations later.
