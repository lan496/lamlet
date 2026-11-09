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

## Bounded candidate discovery

`discovery-plan.json` defines a separate activity-ordered cohort: three category
pages and the first 12 unique non-pinned topics not in the frozen pilot manifest.
The plan was committed before collection. API-shape inspection is disclosed;
these sources are development data. Selection uses no keyword or Lamlet finding.
The listing includes subcategories and topics without input failure reports.
Live pagination is not atomic, so the captured pages describe an observed cohort,
not every topic active during the interval.

```sh
python3 tools/defect_discovery.py fetch --output target/defect-discovery-new
python3 tools/defect_discovery.py publish --cache target/defect-discovery-new \
  --output target/defect-discovery-public-new
python3 tools/defect_discovery.py verify --archive docs/assets/defect-discovery/2026-11-09
python3 tools/defect_discovery.py verify --archive docs/assets/defect-discovery/2026-11-09 \
  --cache target/defect-discovery/2026-11-09
```

Use a fresh output directory for `fetch` and `publish`. An optional `--plan`
selects another committed plan for a new cohort. Requests run serially, using
the existing collector's 30-second timeout and 16-MiB per-response limit. The
first listing and first complete-thread capture are timed before estimating
the remaining work. No simulation, attachment code, or checker is executed.

Every requested page and selected capture has an outcome. Duplicate listing
occurrences, pinned and previously screened topics, budget deferrals, missing
attachments, and capture failures remain visible. Failures do not trigger
replacement. `fetch` exits 1 when any listing page or selected thread cannot be
captured, retaining its report and queue. Missing attachments are recorded
separately and do not by themselves make that exit nonzero. Exit 0 indicates
successful collection, publication, or integrity verification; exit 2 indicates
invalid arguments or integrity/processing errors. None establishes a valid input
or a defect.

The dated public archive contains nine metadata files, including a hash inventory,
all listing identities, source receipts, structural counts, and a pending
screening form. Raw threads and attachments stay in ignored `target/`.
`verify` without a cache checks the published inventory and recomputes selection,
counts, and the pending packet. It does not verify uncached source bytes.
`--cache` additionally checks the exact original listing responses, complete post
streams, and downloaded files. Hash agreement establishes artifact integrity,
not forum authorship or scientific truth. CI verifies the public archive offline.

Follow `packet.md` and fill in a **copy** of `screening.json`; keep the frozen
archive unchanged. Read the complete discussions, including retractions, and
record intent, bounded condition, evidence post numbers, correction, and missing
dependencies. Code blocks include quotations and non-LAMMPS material. Neither
code-block counts, attachments, titles, nor accepted-answer flags assign a label.
All 12 captured topics initially await screening. Screening and later independent
pair admission are separate stages; discovery produces zero gold defect pairs.

## Screen the complete discovery cohort

`screening-plan.json` pins the discovery inventory and decision criteria. It was
committed before complete-thread reading, after titles and structural counts
were visible. `screened-cohort.json` records the subsequent model-assisted
screening of all twelve selected topics and 67 captured posts. The original
discovery archive and its pending form remain unchanged. The separate screening
record preserves the evidence available at that snapshot, not later forum edits.

The classifications describe each discussion. An `input_report` can become a
`replay_candidate` when its intent, condition, correction, input, and dependencies
support preparation. `needs_evidence` records an input report with a concrete
gap. `excluded` preserves engine reports, build/environment issues, usage
questions, announcements, and feature requests outside input-defect admission.
Exclusion does not establish that an input is valid. None of these decisions
admits a gold pair or establishes scientific correctness.

```sh
python3 tools/defect_screening.py verify \
  --report docs/assets/defect-screening/2026-11-09/report.json
python3 tools/defect_screening.py verify \
  --report docs/assets/defect-screening/2026-11-09/report.json \
  --cache target/defect-discovery/2026-11-09
python3 tools/defect_screening.py report
```

`report` recomputes deterministic JSON. `verify` compares it with `--report` and
exits 2 on an integrity or form error. Both exit 0 on successful verification;
that exit code does not grade a defect claim. Optional `--manifest` and `--plan`
paths select a separate version. Preserve existing evidence when extending it.

The validator requires every selected topic, exact captured post-number coverage,
the final captured reply among cited evidence, attachment inspection records,
source receipts, and reasons for each decision. It rejects replay promotion with
declared evidence gaps. This protocol also requires zero native replays, agent
trials, gold pairs, and independent adjudications. Without `--cache`, anchors
are unchecked text tied to known source post numbers. The exact cache adds
post/attachment byte checks and literal anchor matching. Neither mode proves
that reading occurred, authenticates a reviewer, or assesses an interpretation.
All screening is identified as Codex model-assisted development work, with no
independent human adjudication. Two input reports need evidence, ten topics are
excluded, and none is ready for replay preparation. The
[results](../../docs/defect-corpus.md#discovery-screening) explain the retained
conditions and gaps.

## Recover missing public dependencies

`dependency-plan.json` bounds follow-up to six public URLs for the two deferred
discovery reports. It discloses prior source exposure and a failed preliminary
Drive request. `defect_dependencies.py` captures each URL serially, preserving
HTTP error responses as well as successful responses. No login, recursive folder
crawl, attachment execution, or simulation is performed.

```sh
python3 tools/defect_dependencies.py fetch --output target/dependencies-new
python3 tools/defect_dependencies.py verify \
  --snapshot docs/assets/defect-dependencies/2026-11-09/snapshot.json \
  --report docs/assets/defect-dependencies/2026-11-09/report.json
python3 tools/defect_dependencies.py verify \
  --snapshot docs/assets/defect-dependencies/2026-11-09/snapshot.json \
  --report docs/assets/defect-dependencies/2026-11-09/report.json \
  --cache target/dependency-recovery/capture
```

`fetch` requires a new directory. It exits 0 when every request returns a 2xx
response, 1 when any request fails, and 2 on invalid arguments or processing
errors. An HTTP 200 folder or web page does not establish recovery of its linked
files. The archived fetch exited 1 because the anonymous Drive request returned
404. Five other requests succeeded, including the current NIST potential and
library and a commit-pinned reference table. Their three file receipts do not
establish the reporter's original bytes.

`dependency-assessment.json` records short file anchors, remaining gaps, and a
next action for both reports. Neither advances to replay preparation. The Ga-In
case still lacks the original inputs and structures, confirmation of potential
hashes, and exact correction. The tempering case needs a replacement dataset,
includes, PLUMED input, original restarts, and partition launch mapping.

`verify` exits 0 on metadata/report agreement, including recorded source failures,
and 2 on an integrity error. Without `--cache`, it cannot verify source bytes or
literal anchors. The exact cache adds those checks. Neither mode validates a
physical interpretation, confirms original-file provenance, or authorizes gold
admission. A fresh fetch cannot substitute for the original snapshot; publish
new receipts and a separate assessment after reviewing the new responses.
Raw third-party files remain in ignored `target/`. Historical source, native,
diagnostic, discovery, and screening archives remain unchanged.

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

## Versioned checker regrades

A changed checker uses the frozen inputs, native criteria, targets, and original
diagnostic audit plan. Its capture declares the new full source commit separately
from the plan's baseline revision. Build from a clean checkout at that commit
before collecting. Use a new output directory and preserve the original archive:

```sh
cargo build --release --locked
python3 tools/defect_diagnostics.py run \
  --checker-source-commit 0c658e13bf00770086fe8ca11679555046f4f00c \
  --cache target/defect-pilot-20261109/sources \
  --output target/diagnostic-audit-gcmc-new
python3 tools/defect_diagnostics.py verify \
  --archive docs/assets/defect-diagnostics/2026-11-09-gcmc
```

The [GCMC regrade](../../docs/gcmc-diagnostic-recovery.md) recovers the previously
absent temperature-normalization finding. Its independent native measurements
are unchanged. These inputs are development regressions; their reuse cannot
measure generalization to unseen reports or agent improvement. The review packet
and its five pending decisions remain unchanged.

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
