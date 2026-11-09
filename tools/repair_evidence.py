#!/usr/bin/env python3
"""Publish and audit repair-pilot evidence without changing the frozen grader."""

import argparse
from collections import Counter
import json
from pathlib import Path
import shutil

from corpus import write_json
from repair_campaign import digest, frozen, interval_seconds, publish, ROOT
from repair_diagnostics import measure


def verify(archive):
    hashes = json.loads((archive / "hashes.json").read_text())
    actual = {str(p.relative_to(archive)) for p in archive.rglob("*") if p.is_file()
              and p != archive / "hashes.json"}
    if actual != set(hashes):
        raise ValueError("Archive inventory changed")
    frozen(archive, hashes)
    manifest = json.loads((archive / "manifest.json").read_text())
    if digest(archive / "manifest.json") != json.loads((archive / "frozen.json").read_text())["manifest_sha256"]:
        raise ValueError("Frozen manifest changed")
    frozen(archive, manifest["files"])
    frozen(archive / "sources", manifest["sources"])
    summary = json.loads((archive / "results.json").read_text())
    assigned = {(t["id"], t["task"], t["condition"]) for t in manifest["trials"]}
    observed = {(t["id"], t["task"], t["condition"]) for t in summary["trials"]}
    if observed != assigned or len(summary["trials"]) != len(assigned):
        raise ValueError("Assigned trials are missing or duplicated")
    for row in summary["trials"]:
        path = archive / row["id"]
        assessment = json.loads((path / "assessment.json").read_text())
        if assessment["status"] != row["status"]:
            raise ValueError("Summary disagrees with independent assessment")
        if (path / "solution.in").is_file() and digest(path / "solution.in") != assessment.get("submission_sha256"):
            raise ValueError("Assessment refers to another submission")
        events = json.loads((path / "validation-events.json").read_text())
        tools = json.loads((path / "tools.json").read_text())
        if (row["engine_launches"] != sum(e["tool"] == "lmp" for e in events)
                or row["lamlet_checks"] != sum(e["tool"] == "lamlet" for e in events)
                or row["tool_calls"] != sum(len(step["tool_calls"]) for step in tools)):
            raise ValueError("Usage counts disagree with instrumented events")
        usage = json.loads((path / "usage.json").read_text())
        expected_tokens = {key: sum(u.get(key, 0) for u in usage) if usage else None
                           for key in ("input_tokens", "cached_input_tokens", "output_tokens")}
        if row["tokens"] != expected_tokens:
            raise ValueError("Token totals disagree with provider usage")
        timing = json.loads((path / "harbor-result.json").read_text())
        if (row.get("harbor_timestamp_interval_seconds") != interval_seconds(timing.get("agent_execution"))
                or timing.get("verifier_environment_mode") != "separate"):
            raise ValueError("Agent timing or verifier separation disagrees with Harbor")
        if row["agent_seconds"] is not None or row["agent_seconds_reliable"]:
            raise ValueError("Discontinuous wall timestamps must not be reported as execution time")
        reward = (timing.get("verifier_result") or {}).get("rewards", {}).get("reward")
        if row.get("reward") != reward or row.get("success", False) != (assessment["status"] == "correct" and reward == 1):
            raise ValueError("Success disagrees with independent reward")
    if set(summary["conditions"]) != {trial["condition"] for trial in manifest["trials"]}:
        raise ValueError("Condition inventory disagrees with assignments")
    for condition, expected in summary["conditions"].items():
        selected = [row for row in summary["trials"] if row["condition"] == condition]
        statuses = Counter(row["status"] for row in selected)
        success = sum(row.get("success", False) for row in selected)
        aggregate = {"assigned": len(selected), **dict(statuses),
            "success_at_180_seconds": {"numerator": success, "denominator": len(selected)},
            "engine_launches": sum(row["engine_launches"] for row in selected),
            "tool_calls": sum(row["tool_calls"] for row in selected)}
        if expected != aggregate:
            raise ValueError("Condition denominators disagree with assigned outcomes")
    return {"files": len(hashes), "assigned": len(assigned), "integrity_verified": True}


def archive(args):
    publish(args)
    destination = args.public
    measure(args.output, destination / "checker-report.json")
    shutil.copyfile(args.output / "reference-checker.json", destination / "pretrial-checker.json")
    shutil.copytree(args.output / "tasks", destination / "tasks")
    results = json.loads((args.output / "results.json").read_text())
    failures = {}
    for trial in results["trials"]:
        root = args.output / "jobs" / trial["id"] / trial.get("trial_directory", "missing")
        directory = root / "artifacts/app/submission/validated-candidates"
        if directory.is_dir():
            shutil.copytree(directory, destination / trial["id"] / "validated-candidates")
        result_path = root / "result.json"
        result = json.loads(result_path.read_text()) if result_path.is_file() else {}
        exception = result.get("exception_info") or {}
        if exception:
            message = exception.get("exception_message", "")
            failures[trial["id"]] = {"type": exception.get("exception_type"),
                "agent_execution_started": result.get("agent_execution") is not None,
                "node_download_failure": "download from https://nodejs.org/dist/" in message,
                "diagnostic_excerpt": [line for line in message.splitlines() if
                    line.startswith("download from https://nodejs.org/dist/") or "curl: (18)" in line],
                "replacement_trial": False}
        write_json(destination / trial["id"] / "harbor-result.json", {key: result.get(key) for key in (
            "agent_execution", "agent_setup", "environment_setup", "verifier", "verifier_environment_mode", "verifier_result")})
        log = root / "agent/codex.txt"
        usage = []
        if log.is_file():
            for line in log.read_text().splitlines():
                try:
                    event = json.loads(line)
                except ValueError:
                    continue
                if event.get("type") == "turn.completed":
                    usage.append(event["usage"])
        write_json(destination / trial["id"] / "usage.json", usage)
    summary = json.loads((destination / "results.json").read_text())
    inconsistent = []
    for row in summary["trials"]:
        interval = row.pop("agent_seconds", None)
        row["harbor_timestamp_interval_seconds"] = interval
        row["agent_seconds"] = None
        row["agent_seconds_reliable"] = False
        if interval is not None and interval > row["wall_seconds"]:
            inconsistent.append(row["id"])
    write_json(destination / "results.json", summary)
    write_json(destination / "timing-quality.json", {
        "agent_execution_seconds_measured": False,
        "reason": "Harbor wall timestamp intervals exceed monotonic whole-trial durations; retain raw intervals but do not use them as execution times",
        "inconsistent_trials": inconsistent,
        "whole_trial_seconds_clock": "host time.perf_counter, including setup and verification",
        "all_successes_whole_trial_within_180_seconds": all(row["wall_seconds"] <= 180
            for row in summary["trials"] if row.get("success"))})
    write_json(destination / "infrastructure.json", failures)
    for name in ("tools/repair_diagnostics.py", "tools/repair_evidence.py"):
        target = destination / "sources" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
    write_json(destination / "postprocessing.json", {"sources": {
        name: digest(ROOT / name) for name in ("tools/repair_diagnostics.py", "tools/repair_evidence.py")},
        "does_not_change_frozen_oracle": True,
        "precollection_commit": "11b5c7d",
        "superseded_preparation": "An earlier preflight reused a stale checker; no model trials were run there."})
    write_json(destination / "hashes.json", {str(p.relative_to(destination)): digest(p)
        for p in sorted(destination.rglob("*")) if p.is_file() and p != destination / "hashes.json"})
    print(json.dumps(verify(destination), indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("publish", "verify"))
    parser.add_argument("--output", type=Path)
    parser.add_argument("--public", type=Path, required=True)
    args = parser.parse_args()
    args.public = args.public.resolve()
    if args.command == "publish":
        if args.output is None:
            parser.error("publish requires --output")
        args.output = args.output.resolve()
        archive(args)
    else:
        print(json.dumps(verify(args.public), sort_keys=True))
