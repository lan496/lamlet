#!/usr/bin/env python3
"""Regrade immutable P3 pilot submissions after native oracle admission."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

from corpus import write_json
from p3_oracle import VERSION


ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_archive(archive):
    hashes = json.loads((archive / "hashes.json").read_text())
    for name, expected in hashes.items():
        if digest(archive / name) != expected:
            raise ValueError(f"Frozen archive changed: {name}")


def regrade(args):
    verify_archive(args.archive)
    oracle_source = ROOT / "tools/p3_oracle.py"
    oracle_hash = digest(oracle_source)
    admission = json.loads((args.admission / "report.json").read_text())
    runtime = json.loads((args.runtime / "runtime.json").read_text())
    if (not admission["native_admission_passed"] or admission["oracle_sha256"] != oracle_hash
            or admission["runtime"] != runtime):
        raise ValueError("Require successful native admission of this oracle and runtime")
    for name, expected in runtime["libraries"].items():
        if digest(args.runtime / "build" / name) != expected:
            raise ValueError(f"Admitted runtime changed: {name}")
    if args.output.exists():
        raise ValueError("Use a fresh regrade output directory")
    args.output.mkdir(parents=True)
    pilot = json.loads((args.archive / "pilot.json").read_text())
    records = []
    with tempfile.TemporaryDirectory(prefix="lamlet-p3-regrade-") as temporary:
        folder = Path(temporary)
        oracle = folder / "oracle"
        argv = [sys.executable, str(oracle_source), "--runtime", str(args.runtime)]
        oracle.write_text("#!" + sys.executable + "\nimport os, sys\nos.execv(" + repr(sys.executable)
                          + ", " + repr(argv) + " + sys.argv[1:])\n")
        oracle.chmod(0o755)
        task = folder / "task.json"
        spec = json.loads((ROOT / "research/harbor/tasks.json").read_text())
        write_json(task, {"id": "P3", "criteria": spec["tasks"]["P3"]["criteria"]})
        for trial in pilot["trials"]:
            source = args.archive / trial["id"] / "solution.in"
            record = {"id": trial["id"], "condition": trial["condition"],
                      "original_status": trial["scientific_status"],
                      "scientific_status": "unassessed", "input_submitted": source.is_file()}
            if source.is_file():
                started = time.perf_counter()
                logs = folder / trial["id"]
                process = subprocess.run([
                    sys.executable, str(ROOT / "tools/harbor_verifier.py"), "--task", str(task),
                    "--submission", str(source), "--inputs", str(args.inputs_dir),
                    "--oracle", str(oracle), "--logs", str(logs), "--timeout", "90",
                ], capture_output=True, text=True, timeout=100)
                assessment = json.loads((logs / "assessment.json").read_text())
                if assessment.get("infrastructure_error"):
                    raise ValueError(f"Regrade infrastructure failure: {assessment['infrastructure_error']}")
                record.update(scientific_status=assessment["scientific_status"],
                              submission_sha256=digest(source), adapter_exit=process.returncode,
                              seconds=time.perf_counter() - started,
                              assessment=f"{trial['id']}/assessment.json")
                write_json(args.output / record["assessment"], assessment)
            else:
                record["reason"] = "Original trial produced no submission"
                record["provider_errors"] = trial["provider_errors"]
            records.append(record)
            print(json.dumps(record, sort_keys=True), flush=True)
    if digest(oracle_source) != oracle_hash:
        raise ValueError("Oracle changed during regrading")
    verify_archive(args.archive)
    conditions = {}
    for condition in ("A", "C"):
        trials = [r for r in records if r["condition"] == condition]
        counts = Counter(r["scientific_status"] for r in trials)
        conditions[condition] = {"assigned": len(trials), "inputs_submitted": sum(r["input_submitted"] for r in trials),
            **{key: counts[key] for key in ("correct", "incorrect", "unassessed")},
            "confirmed_success_fraction": counts["correct"] / len(trials),
            "success_bounds": [counts["correct"] / len(trials),
                               (counts["correct"] + counts["unassessed"]) / len(trials)]}
    public_admission = dict(admission)
    public_admission["cases"] = []
    for case in admission["cases"]:
        assessed = json.loads((args.admission / case["assessment"]).read_text())
        public_admission["cases"].append({**{k: v for k, v in case.items() if k != "assessment"},
                                          "oracle": assessed.get("oracle")})
    write_json(args.output / "admission.json", public_admission)
    for name in ("p3_oracle.py", "harbor_verifier.py", "p3_admit.py", "p3_regrade.py"):
        shutil.copyfile(ROOT / "tools" / name, args.output / name)
    report = {"schema_version": 1, "oracle_version": VERSION, "oracle_sha256": oracle_hash,
              "original_pilot_sha256": digest(args.archive / "pilot.json"),
              "original_archive_hashes_sha256": digest(args.archive / "hashes.json"),
              "post_hoc": True, "submissions_modified": False, "model_trials_run": False,
              "full_equilibration_executed": False, "container_validation": "not_run",
              "runtime": runtime, "conditions": conditions, "trials": records}
    write_json(args.output / "report.json", report)
    write_json(args.output / "hashes.json", {str(p.relative_to(args.output)): digest(p)
        for p in sorted(args.output.rglob("*")) if p.is_file()})
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ROOT / "docs/assets/evaluation/2026-11-08")
    parser.add_argument("--runtime", type=Path, default=ROOT / "target/p3-runtime")
    parser.add_argument("--inputs-dir", type=Path, default=ROOT / "target/evaluation/upstream/publication/agent_tests/opus47/spall_nb_eval")
    parser.add_argument("--admission", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    for key in ("archive", "runtime", "inputs_dir", "admission", "output"):
        setattr(args, key, getattr(args, key).resolve())
    return regrade(args)


if __name__ == "__main__":
    sys.exit(main())
