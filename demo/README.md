# Present the Lamlet demo

The demo verifies six scenes through the real CLI and produces a standalone HTML
presentation. It covers legacy preservation, quantities, diagnostic JSON,
modules, scoped resources, and independently built stages. It does not run a
LAMMPS simulation, probe an engine, or fetch the upstream corpus.

The [32-second video and animated preview](../docs/demo.md) summarize four scenes.
Regenerate their assets with `python3 demo/render_media.py` after running the
demo. The renderer requires FFmpeg and librsvg and records the evidence hash.

## Prepare the presentation

Use a stable Rust toolchain and Python 3.9 or newer. Python uses only its standard
library. Run these commands from the repository root:

```sh
cargo build --release --locked
python3 demo/run.py
```

Expect six `PASS` lines and exit code 0. The dimensional error and missing-state
error are intentional rejections; the runner verifies their codes and exit
statuses. An unexpected result stops the demo with exit code 1.

Open [the generated presentation](../target/demo/index.html) in a browser. Use
Next, Previous, the scene list, or the left and right arrow keys. Home and End
select the first and last scenes. Each scene offers generated code where
applicable, JSON evidence, and the actual CLI commands.

The presentation needs no server, network, external fonts, or JavaScript
packages. Share `target/demo/index.html` as one file. It shows captured CLI
results; it does not execute a compiler inside the browser. Use **Download
evidence** to save the complete report.

The runner also creates:

| Artifact | Contents |
| --- | --- |
| `target/demo/report.json` | Commands, exit codes, CLI reports, assertions' evidence, compiler version and binary digest |
| `target/demo/workspace/` | Copies of the demo sources and all generated scripts and manifests |
| `target/demo/workspace/.lamlet/build/stages/cold/` | The 85 K instance and its generated cleanup |
| `target/demo/workspace/.lamlet/build/stages/warm/` | The 110 K instance with a separate output path |

The runner writes only under the selected directory inside `target/`. To use a
different built binary or a separate output directory:

```sh
python3 demo/run.py --binary target/debug/lamlet --output target/demo-debug
```

The debug binary must already exist. Rebuilding the compiler and rerunning the
demo refreshes the presentation. No generated outputs need to be committed.
The corpus CI job is configured to run the demo and upload its HTML and JSON as
the `lamlet-demo` artifact. Hosted CI has not been run for this change locally.

## Use this five-minute sequence

Read the [30-second pitch](pitch.md#the-30-second-pitch) on the opening
scene, then advance through the six verified scenes.

| Approximate time | Scene | Show | Say |
| --- | --- | --- | --- |
| 0:30 | Start with the script you have | Identical input and output | "Adoption starts with this existing file. Its bytes are preserved." |
| 1:00 | Put the unit next to the value | `2 fs` becoming `0.002`; finite loop expansion | "The source carries the physical unit. The emitted value follows the active LAMMPS units." |
| 1:40 | Catch the mistake | Error JSON with expected Time and actual Temperature | "This is an expected rejection. An agent gets the code and source location; a failed build keeps the previous output." |
| 2:20 | Make one calculation a task | Stage source and emitted `undump` / `unfix` | "The module establishes the model. The stage binds files and owns these managed resources." |
| 3:20 | Separate the conditions | Cold/warm directories and dependency JSON | "Same stage definition, different parameters and output ownership. These are builds; no trajectory has been produced." |
| 4:20 | Reject hidden state | Missing-state diagnostic | "The stage must establish its own required system. It cannot inherit one from a previous process." |
| 4:45 | Where to start | Current features and boundaries | "Start with one timestep or file dependency. Checkpoint restoration and workflow graphs remain future work." |

These times are a speaking guide, not measured execution times. The runner
verifies all six scenes before the presentation opens.

## Explain the evidence

The live assertions compare emitted bytes and literal expected code, verify
diagnostic identity on repeated checks, and inspect the actual manifests. They
also check that compiling the second instance preserves the first script.

The 998-file compatibility result on the opening scene comes from the committed
[baseline](../tests/corpus/baseline.json). The demo does not rerun that evaluation.
The HTML and JSON identify it as recorded evidence and retain its compiler
fingerprint separately from the binary used for the live demo. Reproduction and
exclusions are documented in [the corpus evaluation](../docs/corpus.md).

Compiler warnings and Unknown findings are retained in the downloadable reports.
The presentation's JSON tab shows selected evidence rather than every diagnostic.
The tiny atomic structure and LJ model are demonstration fixtures, not a
validated materials calculation. The unit-only scene demonstrates compilation
and does not construct a runnable atom system.

Fresh-process stage execution is covered by the CLI's synthetic-engine tests.
This demo verifies compilation and manifests; it makes no claim about an actual
LAMMPS run or numerical equivalence.
