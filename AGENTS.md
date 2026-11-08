# Working on Lamlet

Lamlet is a stable Rust compiler and static checker for LAMMPS inputs.
It emits ordinary LAMMPS scripts and deterministic JSON diagnostics.

## Code and specifications

- `LAMLET_SPEC.md`, Sections 1–8, defines the original v0.1 requirements;
  Section 9 orders its implementation milestones.
- `docs/design.md` and the feature guides document subsequent extensions and
  current limitations. Check the relevant guide before changing behavior.
- `crates/lamlet/` contains the compiler, analysis, units, schemas, and lint rules.
- `crates/lamlet-cli/` contains commands, project handling, and engine invocation.
- Rust integration tests live under each crate's `tests/` directory.
- `tools/` contains Python corpus, manual-rule, and evaluation tooling.
- `tests/corpus/` and `tests/lint/` contain pinned compatibility and lint inputs.
- `docs/`, `zensical.toml`, and `tools/pygments_lamlet/` own documentation and
  syntax highlighting. `demo/` contains reproducible demonstrations.
- `research/` contains task contracts and runtime pins for optional evaluations.

## Compatibility and analysis

- Preserve unmodified legacy LAMMPS inputs byte for byte. Do not reject valid
  commands because their styles, arguments, or effects are unsupported.
- Apply stricter guarantees at explicit typed Lamlet boundaries. Keep an
  unresolved analysis result distinct from an error and from no finding.
- Keep CLI JSON deterministic, with one object on stdout and stable exit codes.
  Keep diagnostic evidence and spans accurate. Test commands through the CLI
  when their observable behavior changes.
- Derive lint conditions from cited documentation or a reviewed physical
  argument. Include valid counterexamples and unresolved cases. Never silently
  modify a physical model or label static acceptance as scientific correctness.
- Manual data is pinned and generated. Update it through `tools/lammps_docs.py`
  and `tools/manual_rules.py`; review provenance and generated changes together.

## Required checks

Run formatting, Clippy, and Rust tests after each implementation milestone:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

For Python tooling, run `python3 -m unittest discover -s tools -p 'test_*.py'`.
For compiler or lint changes, also build the release CLI and run the fixed corpus:

```sh
cargo build --release --locked
python3 tools/lint_evaluation.py --baseline docs/assets/lint-evaluation/report.json
python3 tools/manual_rules.py --check
```

For documentation or highlighting changes, use the pinned environment:

```sh
uv run --python 3.12 --with-requirements docs/requirements.txt python -m unittest discover -s tools/pygments_lamlet -p 'test_*.py'
uv run --python 3.12 --with-requirements docs/requirements.txt zensical build --strict
python3 tools/check_docs.py target/docs-site
```

Network-backed corpus fetches and native LAMMPS or Docker evaluations are optional
unless the task requires them. Time one representative evaluation before a full
sweep. Record skipped checks and infrastructure failures explicitly.

## Evaluation and delivery

- Preserve historical evaluation archives. Publish a new dated archive for a
  regrade or new run, retaining missing outcomes and original denominators.
- Do not change an independent verifier after observing evaluation submissions.
  Admit verifier changes separately before prospective trials. Track label
  review and scientific validation separately from software checks.
- Keep generated builds, downloaded corpora, and local runs in ignored `target/`.
  Commit only deliberate, reviewed public evidence and never credentials.
- Track work with a GitHub issue and a focused PR. Commit verified units with
  explicit file staging. Open PRs as drafts, mark them ready after all CI passes,
  and leave merging to the user.
- `.github/workflows/ci.yml` validates Rust on three platforms, documentation,
  and corpora. Its Pages job publishes only after those checks pass. Automatic
  publishing follows `main`; the `github-pages` environment restricts the branch.
- Use English for repository documentation, issues, and PR descriptions. Report
  measured results with evidence and state what remains untested.
