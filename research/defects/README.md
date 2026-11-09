# Collecting LAMMPS defect reports

This development collection preserves natural reports, proposed corrections,
unresolved claims, engine defects, and counterexamples. A source report is not a
gold defect label. Lamlet output is excluded from selection and adjudication.

## Selection and capture

The first pilot screens 20 public matsci.org discussions. The sources come from
the existing reviewed forum references and searches for reported unit,
temperature, geometry, resource, and output problems. This is a purposeful
feasibility sample. It cannot estimate defect prevalence in research inputs.
All sources are development data, including sources previously used to design
Lamlet rules. No case in this pilot is held out.

`cases.json` records the intended calculation, a bounded condition, discussion
post numbers, text anchors, proposed correction evidence, reported engine
version, dependencies, and reasons for deferral. The disposition describes the
discussion's evidence. `reported_fix` means that a source author reports a fix,
not that Lamlet's authors have verified it. A thread can contain several defects
and superseded corrections. Later stages must separate those claims.

The collector retrieves all post IDs in the Discourse stream, including posts
beyond the first response page. It saves the original responses and attachments,
and records SHA-256 hashes. Missing posts fail capture. Unavailable attachments
remain explicit outcomes. Attachment counts use unique URLs, since quotations
can repeat the same link.

```sh
python3 tools/defect_corpus.py fetch --output target/defect-sources-new
python3 tools/defect_corpus.py audit --snapshot target/defect-sources-new/snapshot.json --cache target/defect-sources-new
python3 tools/defect_corpus.py audit
```

The last command checks the committed manifest and receipts without network
access. Adding `--cache target/defect-pilot-20261109/sources` also checks the
original downloaded bytes, post completeness, and cited text anchors against
the pilot snapshot. A live refetch creates a new snapshot because posts and
attachments can change. It does not overwrite the pinned receipts.

Raw source content stays in ignored `target/`. Public availability does not
establish permission to redistribute scripts or potential files. The committed
collection contains bibliographic metadata, short anchors, hashes, and original
Lamlet-authored reconstructions. A future redistribution requires a recorded
license or permission for each source artifact.

## Pair admission

A proposed negative condition needs a recoverable original input, all relevant
dependencies, a specific computational intent, and evidence of that condition's
violation. A proposed correction needs an explicit change and an independent
test of the same condition. Native execution success alone does not establish
scientific validity. The test must address the intended observable or invariant.

Original full-input replays, extracted source fragments, instrumented or
shortened inputs, and new model reconstructions are different provenance kinds.
Preserve the original bytes and an exact change list for every derived input.
A historical report reproduced only on a modern engine is a modern replay,
not a reproduction of the original executable's behavior. Timeouts, unsupported
packages, missing files, and changed modern behavior remain in the denominator.

An independent domain reviewer must assess the intent, correction, and physical
test before a pair becomes a gold example. Every pilot review is pending.
The current collection does not measure diagnostic precision, scientific
correctness, or agent improvement.

Valid counterexamples accompany each proposed condition. Deliberate modeling
choices, insufficient evidence, and engine bugs must not become incorrect-input
labels. An author-confirmed resolution can still be contradicted later in the
same thread.

## Native replay contract

The initial replay plans are fixed in `replays.json` before engine execution.
The native collector does not invoke Lamlet. It records all variant outcomes,
engine and input hashes, the selected engine source revision, and time limits.
The first representative case is timed before estimating the serial sweep.
Each process has a 10-second cap. Expensive production trajectories are excluded.

The velocity reconstruction tests the initial kinetic energy of a single
free tungsten atom. With lattice spacing \(a=3.13400196\), implicit lattice units
predict an energy ratio of \(a^2\) relative to explicit box units. The intended
box-unit energy is computed from \(m v^2/2\) with the pinned LAMMPS metal-unit
conversion factor. Relative tolerance is \(10^{-8}\).

The GCMC replay preserves both full posted inputs with a one-line thermostat
correction. A separate instrumented pair replaces the million-step run by
400 steps and requests atom count, kinetic energy, and thermostat temperature.
For real units, an independent normalization check uses
\(T_{\mathrm{kin}}=2K/[k_B(3N-3)]\), with
\(k_B=0.0019872067\) kcal mol\(^{-1}\) K\(^{-1}\).
It checks normalization only. It cannot establish temperature control or
reproduce the reported freezing over a million steps.

The orientation replay extracts the original lattice command and scales its
direction vectors to integers in a separately identified reconstruction.
It checks command acceptance, not the geometry of a complete crystal.
The averaging replay uses the posted attachment and a correction that omits
its final `write_dump`, retaining the scheduled `dump`. A modern success of
both variants is recorded as failure to reproduce the historical error.

The dated observations and their limitations are reported in
[`docs/defect-corpus.md`](../../docs/defect-corpus.md). Reproduce the input
transformations and collect new native observations with the pinned engine:

```sh
python3 tools/defect_replay.py run --engine target/defect-runtime/build/lmp \
  --cache target/defect-pilot-20261109/sources --output target/defect-replay-new
python3 tools/defect_replay.py publish --run target/defect-replay-new \
  --output target/defect-public-new
python3 tools/defect_replay.py verify --run docs/assets/defect-corpus/2026-11-09
python3 tools/defect_replay.py verify --run docs/assets/defect-corpus/2026-11-09 \
  --cache target/defect-pilot-20261109/sources
```

Offline verification checks the complete file inventory, hashes, provenance,
execution outcomes, and measurements recomputed from native stdout. Supplying
the original cache also audits its bytes and regenerates every before/after
input to verify the recorded input hashes. A live refetch may differ from that
cache and cannot substitute silently for the original snapshot. Publishing
omits source inputs and generated simulation files; the engine was launched
with `-echo none`. Its executable hash is recorded, but the runner does not
verify the declared engine source revision automatically.

## Frozen diagnostic audit

The checker audit plan in `diagnostic-audit.json` was committed before collecting
Lamlet observations. It pins the native evidence, checker source revision,
target rules and subjects, and conditions excluded from native comparisons.
The CLI is unchanged. The audit retains absent targets, Unknown findings,
infrastructure failures, and compatibility outcomes separately. The velocity
reconstruction has external intent and no assumed static detector.

```sh
python3 tools/defect_diagnostics.py run \
  --cache target/defect-pilot-20261109/sources --output target/diagnostic-audit-new
python3 tools/defect_diagnostics.py verify \
  --archive docs/assets/defect-diagnostics/2026-11-09
python3 tools/defect_diagnostics.py prepare-review --output target/condition-review-new
python3 tools/defect_diagnostics.py validate-review \
  --review docs/assets/defect-diagnostics/review/review.json
```

`run` requires the exact original cache and a new output directory. It reproduces
the input hashes, records the binary hash and declared source revision, and
captures normal, strict, and build CLI JSON plus emitted-file hashes. The runner
does not independently establish the binary's declared source revision. Its
exit 0 indicates collection and legacy controls succeeded; failed diagnostic
predictions remain observations. Exit 1 records per-mode infrastructure or
compatibility failures, and exit 2 records invalid inputs or integrity failures.

Offline `verify` checks the full inventory and recomputes the report, without
rerunning the binary or verifying uncached source bytes. The
[results page](../../docs/natural-diagnostic-audit.md) reports a missed GCMC
normalization condition and preserved legacy compatibility. Historical native
evidence remains unchanged.

The review packet omits checker observations. The published review is pending,
so `validate-review` exits 1. Named, independence-attested reviewers must give a
reason for every non-pending decision. The validator checks evidence hashes and
form completeness, not reviewer identity or scientific truth. Even an all-agree
form does not automatically admit gold defect pairs.

## Future evaluation splits

Freeze the case selection, labels, independent tests, and executable profile
before inspecting checker findings. Group each original source, minimized
input, reconstruction, and mutation in the same split. Related threads with
the same failure mechanism also need grouping when measuring generalization
to unseen defect families. Development data and new prospective evaluation
sources must remain separate. Public source exposure can contaminate language
model evaluations and must be disclosed.

Synthetic mutations can test rule coverage, but their results require separate
denominators from natural reports. A later random sample of research inputs,
with blinded review, is required for a prevalence estimate.
