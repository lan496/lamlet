#!/usr/bin/env python3
"""Adapt an independent oracle's evidence to Harbor rewards. No physics inference."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def assess(payload, task, submission_hash):
    required = task.get("criteria")
    if (not isinstance(required, list) or not required
            or not all(isinstance(item, str) and item.strip() for item in required)
            or len(set(required)) != len(required)):
        raise ValueError("Task criteria must be a nonempty list of unique names")
    if (type(payload.get("schema_version")) is not int or payload["schema_version"] != 1
            or payload.get("task") != task["id"]
            or payload.get("submission_sha256") != submission_hash
            or not isinstance(payload.get("oracle_version"), str)
            or not payload["oracle_version"].strip()):
        raise ValueError("Oracle identity, version, task, or submission hash is invalid")
    criteria = payload.get("criteria")
    if not isinstance(criteria, dict) or set(criteria) != set(required):
        raise ValueError("Oracle criteria do not match the task contract")
    for finding in criteria.values():
        if (not isinstance(finding, dict) or finding.get("status") not in ("pass", "fail", "unknown")
                or not isinstance(finding.get("evidence"), list) or not finding["evidence"]
                or not all(isinstance(item, str) and item.strip() for item in finding["evidence"])):
            raise ValueError("Each oracle criterion needs a status and nonempty evidence")
    statuses = {finding["status"] for finding in criteria.values()}
    if "fail" in statuses:
        return "incorrect", 0
    if "unknown" in statuses:
        return "unassessed", None
    return "correct", 1


def verify(args):
    args.logs.mkdir(parents=True, exist_ok=True)
    # Harbor retries must never inherit a reward from a previous verifier call.
    for name in ("reward.txt", "reward.json"):
        (args.logs / name).unlink(missing_ok=True)
    report = {"schema_version": 1, "scientific_status": "unassessed", "reward": None}
    try:
        task = json.loads(args.task.read_text(encoding="utf-8"))
        report["task"] = task["id"]
        if not args.submission.exists():
            report.update(submission_status="missing", reward=0)
        else:
            if args.submission.is_symlink() or not args.submission.is_file():
                raise ValueError("Submission must be a regular file")
            submission_hash = hashlib.sha256(args.submission.read_bytes()).hexdigest()
            report["submission_sha256"] = submission_hash
            report["oracle_sha256"] = hashlib.sha256(args.oracle.read_bytes()).hexdigest()
            process = subprocess.run(
                [str(args.oracle), "--task", task["id"], "--input", str(args.submission),
                 "--inputs-dir", str(args.inputs)],
                capture_output=True, text=True, timeout=args.timeout,
            )
            (args.logs / "oracle.stdout").write_text(process.stdout, encoding="utf-8")
            (args.logs / "oracle.stderr").write_text(process.stderr, encoding="utf-8")
            if process.returncode != 0:
                raise ValueError(f"Oracle process failed with exit code {process.returncode}")
            payload = json.loads(process.stdout)
            if not isinstance(payload, dict):
                raise ValueError("Oracle output must be one JSON object")
            if hashlib.sha256(args.submission.read_bytes()).hexdigest() != submission_hash:
                raise ValueError("Submission changed during verification")
            status, reward = assess(payload, task, submission_hash)
            report.update(scientific_status=status, reward=reward, oracle=payload)
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        report["infrastructure_error"] = str(error)
    (args.logs / "assessment.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    if report["reward"] is not None:
        (args.logs / "reward.txt").write_text(str(report["reward"]) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    return 0 if report["reward"] is not None else 2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", type=Path, default=Path("/tests/task.json"))
    parser.add_argument("--submission", type=Path, default=Path("/app/submission/solution.in"))
    parser.add_argument("--inputs", type=Path, default=Path("/app/inputs"))
    parser.add_argument("--oracle", type=Path, default=Path("/opt/lamlet-bench/oracle"))
    parser.add_argument("--logs", type=Path, default=Path("/logs/verifier"))
    parser.add_argument("--timeout", type=float, default=300)
    return verify(parser.parse_args())


if __name__ == "__main__":
    sys.exit(main())
