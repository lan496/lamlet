#!/usr/bin/env python3
"""Exercise the real P3 oracle and Harbor adapter on reference and mutation inputs."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

from corpus import write_json


ROOT = Path(__file__).resolve().parents[1]


def cases(reference):
    yield "reference", reference, "correct", None
    yield "renamed-groups", reference.replace("projectile", "flyer").replace("target", "sample"), "correct", None
    yield "split-equilibration", reference.replace("run 100000\nunfix", "run 50000\nrun 50000\nunfix"), "correct", None
    for name, old, new, criterion in (
        ("wrong-gap", "39.5*v_a+15-60*v_a", "39.5*v_a+1.5-60*v_a", "gap"),
        ("wrong-speed", "0 0 -20 sum", "0 0 -2 sum", "impact_velocity"),
        ("wrong-direction", "0 0 -20 sum", "0 0 20 sum", "impact_velocity"),
        ("overwritten-thermal-velocity", "sum yes", "sum no", "thermal_velocity_retained"),
        ("wrong-projectile-size", "60 79.999999", "60 78.999999", "groups"),
        ("periodic-impact-axis", "boundary p p s", "boundary p p p", "boundaries"),
        ("short-equilibration", "run 100000\nunfix", "run 10000\nunfix", "equilibration"),
        ("overlapping-integrators", "unfix equilibrate", "# unfix omitted", "adiabatic_integration"),
        ("substituted-potential", "pair_style eam/alloy", "pair_style zero 5.0", "potential"),
    ):
        if reference.count(old) != 1:
            raise ValueError(f"Mutation {name} no longer matches exactly one source location")
        yield name, reference.replace(old, new), "incorrect", criterion
    yield "unsupported-shell", reference + "\nshell touch /logs/verifier/reward.txt\n", "unassessed", None


def admit(args):
    if args.output.exists():
        raise ValueError("Use a fresh admission output directory")
    args.output.mkdir(parents=True)
    oracle = args.output / "oracle"
    argv = [sys.executable, str(ROOT / "tools/p3_oracle.py"), "--runtime", str(args.runtime)]
    oracle.write_text("#!" + sys.executable + "\nimport os, sys\nos.execv(" + repr(sys.executable)
                      + ", " + repr(argv) + " + sys.argv[1:])\n")
    oracle.chmod(0o755)
    task = args.output / "task.json"
    spec = json.loads((ROOT / "research/harbor/tasks.json").read_text())
    write_json(task, {"id": "P3", "criteria": spec["tasks"]["P3"]["criteria"]})
    reference = (ROOT / "research/harbor/p3/reference.in").read_text()
    selected = list(cases(reference))
    records = []
    oracle_hash = hashlib.sha256((ROOT / "tools/p3_oracle.py").read_bytes()).hexdigest()
    for name, text, expected, criterion in selected:
        folder = args.output / name
        folder.mkdir()
        submission = folder / "solution.in"
        submission.write_text(text)
        started = time.perf_counter()
        process = subprocess.run([
            sys.executable, str(ROOT / "tools/harbor_verifier.py"), "--task", str(task),
            "--submission", str(submission), "--inputs", str(args.inputs_dir),
            "--oracle", str(oracle), "--logs", str(folder / "logs"), "--timeout", "90",
        ], capture_output=True, text=True, timeout=100)
        elapsed = time.perf_counter() - started
        if hashlib.sha256((ROOT / "tools/p3_oracle.py").read_bytes()).hexdigest() != oracle_hash:
            raise ValueError("Oracle source changed during admission")
        (folder / "stdout.json").write_text(process.stdout)
        (folder / "stderr.txt").write_text(process.stderr)
        assessment = json.loads(process.stdout)
        correct_criterion = (criterion is None or assessment.get("oracle", {}).get("criteria", {})
                             .get(criterion, {}).get("status") == "fail")
        passed = (assessment["scientific_status"] == expected and correct_criterion
                  and process.returncode == (2 if expected == "unassessed" else 0)
                  and assessment["reward"] == {"correct": 1, "incorrect": 0, "unassessed": None}[expected])
        records.append({"case": name, "expected": expected, "observed": assessment["scientific_status"],
                        "criterion": criterion, "passed": passed, "seconds": elapsed,
                        "submission_sha256": hashlib.sha256(text.encode()).hexdigest()})
        print(json.dumps(records[-1], sort_keys=True), flush=True)
        if name == "reference":
            print(f"Reference took {elapsed:.3f}s; {len(selected)} cases estimated at {elapsed * len(selected):.1f}s", file=sys.stderr)
            if not passed:
                break
    report = {"schema_version": 1, "native_admission_passed": len(records) == len(selected)
              and all(r["passed"] for r in records), "cases": records,
              "oracle_sha256": oracle_hash,
              "runtime": json.loads((args.runtime / "runtime.json").read_text()),
              "full_equilibration_executed": False, "model_trials_run": False,
              "container_validation": "not_run"}
    write_json(args.output / "report.json", report)
    return 0 if report["native_admission_passed"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", type=Path, default=ROOT / "target/p3-runtime")
    parser.add_argument("--inputs-dir", type=Path, default=ROOT / "target/evaluation/upstream/publication/agent_tests/opus47/spall_nb_eval")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.runtime = args.runtime.resolve()
    args.inputs_dir = args.inputs_dir.resolve()
    args.output = args.output.resolve()
    return admit(args)


if __name__ == "__main__":
    sys.exit(main())
