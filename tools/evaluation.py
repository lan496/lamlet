#!/usr/bin/env python3
"""Replay pinned published scripts through Lamlet without running MD or agents."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

from corpus import findings, git, write_json


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "research/holbrook/source.json"
PUBLISHED = re.compile(
    r"publication/pipeline/generated_scripts/prompt([123])/([^/]+)/"
    r"prompt\1-\2-T([0-9])\.in"
)
AGENT_TASKS = {
    "al_npt_eval/al_npt.in": "P1",
    "ni_melting_eval/ni_melting.in": "P2",
    "spall_nb_eval/spall_nb.in": "P3",
    "nb_spall_eval/nb_spall.in": "P3",
}


def source():
    return json.loads(SOURCE.read_text(encoding="utf-8"))


def fetch(cache, specification):
    fresh = not cache.exists()
    if fresh:
        cache.parent.mkdir(parents=True, exist_ok=True)
        git(ROOT, "clone", "--filter=blob:none", "--no-checkout", "--depth", "1",
            specification["repository"], str(cache))
    origin = git(cache, "config", "--get", "remote.origin.url").decode().strip()
    if origin != specification["repository"]:
        raise ValueError(f"Unexpected origin in {cache}: {origin}")
    if not fresh and git(cache, "status", "--porcelain").strip():
        raise ValueError(f"Refusing to change a dirty source checkout: {cache}")
    git(cache, "fetch", "--depth", "1", "origin", specification["commit"])
    git(cache, "sparse-checkout", "set", "--no-cone", *specification["sparse"])
    git(cache, "checkout", "--detach", specification["commit"])


def identify(path):
    match = PUBLISHED.fullmatch(path)
    if match:
        task, model, trial = match.groups()
        return {"id": f"published/{model}/P{task}/{trial}", "group": "published",
                "task": f"P{task}", "model": model, "trial": int(trial)}
    prefix = "publication/agent_tests/"
    if path.startswith(prefix):
        model, _, rest = path[len(prefix):].partition("/")
        if rest in AGENT_TASKS and model in ("opus47", "sonnet46"):
            task = AGENT_TASKS[rest]
            return {"id": f"agent/{model}/{task}", "group": "agent",
                    "task": task, "model": model, "trial": None}
    return None


def verified_inputs(cache, specification):
    revision = git(cache, "rev-parse", "HEAD").decode().strip()
    if revision != specification["commit"]:
        raise ValueError(f"Expected revision {specification['commit']}, found {revision}")
    cases = []
    for entry in git(cache, "ls-tree", "-rz", "HEAD").split(b"\0"):
        if not entry:
            continue
        metadata, raw_path = entry.split(b"\t", 1)
        path = raw_path.decode("utf-8")
        case = identify(path)
        if case is None:
            continue
        mode, kind, blob = metadata.decode().split()
        if kind != "blob" or mode not in ("100644", "100755"):
            raise ValueError(f"Not a regular source file: {path}")
        data = (cache / path).read_bytes()
        digest = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        if digest != blob:
            raise ValueError(f"Modified source input: {path}")
        cases.append({**case, "path": path, "git_blob": blob,
                      "sha256": hashlib.sha256(data).hexdigest()})
    if len({case["id"] for case in cases}) != len(cases):
        raise ValueError("Duplicate source case IDs")
    for group, expected in specification["groups"].items():
        selected = [case for case in cases if case["group"] == group]
        tasks = Counter(case["task"] for case in selected)
        if (len(selected) != expected["expected_files"]
                or any(tasks[task] != expected["expected_per_task"]
                       for task in ("P1", "P2", "P3"))):
            raise ValueError(f"Incomplete selection for {group}: {dict(tasks)}")
    return sorted(cases, key=lambda case: case["id"])


def replay_case(binary, cache, case, output):
    path = cache / case["path"]
    started = time.perf_counter()
    standard = findings(binary, path)
    strict = findings(binary, path, strict=True)
    output.unlink(missing_ok=True)
    built = findings(binary, path, output=output)
    if standard != built:
        raise ValueError(f"check/build diagnostics disagree for {case['id']}")
    if built["summary"]["errors"]:
        round_trip = "build_rejected"
    else:
        round_trip = "identical" if output.read_bytes() == path.read_bytes() else "different"
    return {**case, "standard": standard, "strict": strict,
            "round_trip": round_trip, "wall_seconds": time.perf_counter() - started,
            "scientific_correctness": None}


def summarize(cases):
    result = {}
    # Preserve separate denominators for the one-shot corpus and the agent examples.
    for group in sorted({case["group"] for case in cases}):
        for task in ("P1", "P2", "P3"):
            selected = [case for case in cases
                        if case["group"] == group and case["task"] == task]
            if not selected:
                continue
            result[f"{group}/{task}"] = {
                "files": len(selected),
                "standard_accepted": sum(not case["standard"]["summary"]["errors"]
                                         for case in selected),
                "strict_accepted": sum(not case["strict"]["summary"]["errors"]
                                       for case in selected),
                "files_with_unknowns": sum(case["standard"]["summary"]["unknowns"] > 0
                                           for case in selected),
                "round_trip": dict(sorted(Counter(case["round_trip"]
                                                   for case in selected).items())),
                "scientific_assessed": 0,
                "scientific_correct": None,
            }
    return result


def replay(args, specification):
    cases = verified_inputs(args.cache, specification)
    if args.case:
        cases = [case for case in cases if case["id"] == args.case]
        if not cases:
            raise ValueError(f"Unknown case ID: {args.case}")
    elif args.limit is not None:
        cases = cases[:args.limit]
    binary = args.binary.resolve(strict=True)
    version = subprocess.run([str(binary), "--version"], check=True,
                             capture_output=True, text=True).stdout.strip()
    started = time.perf_counter()
    results = []
    with tempfile.TemporaryDirectory(prefix="lamlet-replay-") as temporary:
        for case in cases:
            results.append(replay_case(binary, args.cache, case, Path(temporary) / "output.in"))
    report = {
        "schema_version": 1,
        "experiment": "published_script_replay",
        "source": {key: specification[key] for key in ("repository", "commit", "paper")},
        "binary": {"version": version, "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()},
        "selection": {"case": args.case, "limit": args.limit,
                      "complete": args.case is None and args.limit is None},
        "commands": ["lamlet check INPUT --language lammps --format json",
                     "lamlet check INPUT --language lammps --format json --strict",
                     "lamlet build INPUT --language lammps --format json -o OUTPUT"],
        "summary": summarize(results),
        "wall_seconds": time.perf_counter() - started,
        "agent_trials_run": 0,
        "lammps_launches": 0,
        "independent_oracle": "not_implemented",
        "success_at_budget": None,
        "diagnostic_precision": None,
        "false_positive_rate": None,
        "cases": results,
    }
    write_json(args.report, report)
    print(json.dumps({key: value for key, value in report.items() if key != "cases"},
                     indent=2, sort_keys=True))
    return int(any(case["round_trip"] == "different" for case in results))


def positive(value):
    number = int(value)
    if number <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return number


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("fetch", "replay"))
    parser.add_argument("--cache", type=Path, default=ROOT / "target/evaluation/upstream")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lamlet")
    parser.add_argument("--report", type=Path, default=ROOT / "target/evaluation/replay.json")
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--case", help="Select an exact semantic case ID")
    selection.add_argument("--limit", type=positive, help="Timing subset; verifies all source inputs first")
    args = parser.parse_args()
    args.cache = args.cache.resolve()
    try:
        specification = source()
        if args.command == "fetch":
            fetch(args.cache, specification)
            cases = verified_inputs(args.cache, specification)
            print(json.dumps({"commit": specification["commit"], "verified_inputs": len(cases)},
                             sort_keys=True))
            return 0
        return replay(args, specification)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        details = getattr(error, "stderr", None)
        if isinstance(details, bytes):
            details = details.decode("utf-8", errors="replace")
        print(json.dumps({"error": str(error), "details": details,
                          "kind": "infrastructure_failure"}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
