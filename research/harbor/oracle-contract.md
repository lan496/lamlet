# Independent oracle interface

`tools/harbor_verifier.py` adapts a trusted oracle's evidence to Harbor rewards.
It does not implement molecular dynamics validation. Generated tasks bundle it
as `/tests/verifier.py` and run it through `/tests/test.sh` in a separate verifier
container. The required criteria come from [tasks.json](tasks.json).

## Inputs and process result

The adapter reads the submitted LAMMPS file at
`/app/submission/solution.in`, computes its SHA-256, then calls:

```text
/opt/lamlet-bench/oracle --task P3 --input /app/submission/solution.in --inputs-dir /app/inputs
```

The executable belongs to the trusted verifier image. No oracle implementation
is supplied by this prototype. The agent cannot select the executable through
its submission. The adapter records the executable's SHA-256 and captures its
stdout and stderr in verifier logs.

Exit code 0 means that the oracle produced an assessment. It does not mean that
the submission is correct. Any other exit code, timeout, or malformed output is
an infrastructure failure with no scientific label and no reward. The default
oracle timeout is 300 seconds. `--timeout` changes that subprocess limit.

## Output schema

The oracle writes one JSON object to stdout. This partial example illustrates
the structure; an actual P3 assessment must contain every required criterion:

```json
{
  "schema_version": 1,
  "task": "P3",
  "submission_sha256": "SHA256_OF_THE_SUBMITTED_LAMMPS_BYTES",
  "oracle_version": "AUDITED_ORACLE_REVISION",
  "criteria": {
    "gap": {
      "status": "unknown",
      "evidence": ["Required engine observation was unavailable"]
    }
  }
}
```

`criteria` must contain exactly the required names, with no missing or extra
names. Each finding has `status` equal to `pass`, `fail`, or `unknown`, and a
nonempty list of evidence strings. A version string identifies the audited
oracle implementation. Task ID and submission hash must match the adapter's
inputs. The adapter checks that the submitted bytes remain unchanged during
assessment.

Evidence must point to observations or independently checked properties. The
adapter checks the transport structure, not the truth of evidence strings.
Oracle review, adversarial tests, and independent engine observations establish
that trust. An agent-authored claim file is not an oracle output and is ignored.

## Reward mapping

| Situation | Scientific status | Reward | Adapter exit |
| --- | --- | --- | --- |
| Every required criterion passes | `correct` | 1 | 0 |
| At least one required criterion fails | `incorrect` | 0 | 0 |
| Unknown criteria, with no failure | `unassessed` | Absent | 2 |
| Oracle unavailable, invalid, failed, or timed out | `unassessed` | Absent | 2 |
| Required submission file missing | `unassessed` | 0 for submission failure | 0 |

The adapter removes existing `reward.txt` and `reward.json` before each call.
It writes `assessment.json` for every completed adapter call and writes
`reward.txt` only for a resolved task outcome. Unknown and infrastructure
outcomes intentionally leave Harbor without a reward. They must be retained as
unassessed or failed verification in campaign analysis, not dropped or counted
as scientific success. Parser acceptance and zero-force fallback execution have
no special reward path.

## Scope of the current criteria

The criterion names cover the structures, potential identities, ensembles,
physical durations, and task-specific requirements listed in the
[evaluation protocol](../../docs/agent-evaluation.md). P3 additionally requires
groups, boundaries, gap, equilibration, impact velocity, retained thermal
velocity, and adiabatic integration.

The manifest is a list of required questions, not an implemented physics test.
Tolerances, observation procedures, alternative valid implementations, and
reviewed expected outcomes remain to be defined. Public P1–P3 tasks are listed
as development tasks; `held_out_tasks` is empty.
