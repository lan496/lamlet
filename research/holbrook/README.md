# Replay the published LAMMPS artifacts

This pilot runs the real Lamlet CLI on the public artifacts associated with
[Holbrook et al.](https://arxiv.org/abs/2603.20630). It checks acceptance,
diagnostics, and byte preservation. It does not run an agent, execute LAMMPS,
or label scientific correctness. See the [evaluation protocol](../../docs/agent-evaluation.md)
for the proposed controlled comparison.

## Fetch the pinned inputs

Build the release CLI, then fetch the source revision in
[source.json](source.json):

```sh
cargo build --release --locked
python3 tools/evaluation.py fetch
```

The fetch uses a sparse Git checkout in `target/evaluation/upstream`. It includes
240 one-shot scripts, six agent example scripts, the original method
descriptions, three pinned potential files for Harbor task preparation, and
baseline implementation files. Upstream source is research
data. The tool does not load the skill as instructions, execute upstream code,
install Python packages, or run simulations.

Git and network access are needed for fetch. Python 3.9 or later and the built
Lamlet binary suffice for offline replay. The fetched artifact revision is a
public release pin, not proof of the exact historical experimental environment.

## Time one case before the full replay

Run a named P3 agent artifact first:

```sh
python3 tools/evaluation.py replay --case agent/sonnet46/P3 --report target/evaluation/timing.json
```

The JSON includes `wall_seconds` for one case and a separate whole-replay time.
Multiply the per-case time by 246 to estimate the serial full-replay budget.
Allow for differences in script length and diagnostics. No MD work is included.

Then replay all artifacts:

```sh
python3 tools/evaluation.py replay
```

The full report is `target/evaluation/replay.json`. Each input gets a normal
check, a strict check, and a build in explicit LAMMPS mode. The tool compares
check and build diagnostics and compares emitted bytes with the original.
It records each source path, Git blob ID, SHA-256, full diagnostics, elapsed
time, and the compiler binary hash. One-shot and agent examples have separate
summary denominators, split by task.

All 246 selected source files must match the pinned Git tree before any replay,
including a limited replay. Missing files, modified inputs, and incomplete
selections fail instead of silently shrinking the denominator. `--limit N`
supports timing subsets and marks the report incomplete. Use `--case` to select
a specific semantic ID.

## Interpret the report

`standard_accepted` and `strict_accepted` count scripts with no error diagnostic.
`files_with_unknowns` records partial coverage. `round_trip` distinguishes
`identical`, `different`, and `build_rejected`. A rejected input has no emitted
artifact to compare. A checker crash, timeout, or I/O error stops the replay as
an infrastructure failure.

Scientific outcomes, Success@budget, diagnostic precision, and false-positive
rate are `null`. The report records zero agent trials and zero LAMMPS launches.
Do not infer scientific success from static acceptance, including acceptance of
a script with known physical mistakes. Lamlet currently has no general geometry
checker.

Replay exits with 0 after a completed evaluation, including any recorded static
rejections. It exits with 1 if an emitted legacy script differs, or 2 for an
infrastructure failure. An earlier report is not updated on infrastructure
failure and must not be mistaken for a newly completed evaluation.

Run the offline tool checks with:

```sh
python3 -m unittest discover -s tools -p 'test_*.py'
```

The controlled AST-skill comparison, independent scientific oracle, and paid
model trials are not implemented by this runner. The original prompts and
baseline files remain in the ignored source checkout for auditing. Generated
reports are also ignored and can be archived with a future campaign.
