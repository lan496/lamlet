#!/usr/bin/env python3
"""Admit the frozen P3 oracle through Harbor's separate verifier, without models."""

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from corpus import write_json
from harbor_verifier import assess
from p3_admit import cases
from p3_oracle import VERSION


ROOT = Path(__file__).resolve().parents[1]
REWARDS = {"correct": 1, "incorrect": 0, "unassessed": None}
SOURCES = ("tools/p3_oracle.py", "tools/harbor_verifier.py", "tools/p3_admit.py",
           "tools/p3_harbor_admit.py", "tools/p3_container.py", "research/harbor/tasks.json",
           "research/harbor/p3/runtime.json", "research/harbor/p3/reference.in",
           "research/harbor/p3/Dockerfile")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_files(root, hashes):
    if not hashes:
        raise ValueError("Require a nonempty frozen file manifest; rebuild the container task")
    for name, expected in hashes.items():
        path = root / name
        if not path.is_file() or digest(path) != expected:
            raise ValueError(f"Frozen admission file changed: {name}")


def assessment_checks(trial, expected, criterion, submission_hash, oracle_hash, task):
    assessment = json.loads((trial / "verifier/assessment.json").read_text())
    result = json.loads((trial / "result.json").read_text())
    oracle = assessment.get("oracle", {})
    status, reward = assess(oracle, task, submission_hash)
    reward_path = trial / "verifier/reward.txt"
    reward_json = trial / "verifier/reward.json"
    exception = result.get("exception_info")
    exception_type = exception.get("exception_type") if exception else None
    verifier = result.get("verifier_result")
    if expected == "unassessed":
        transport = (not reward_path.exists() and not reward_json.exists()
                     and verifier is None and exception_type == "RewardFileNotFoundError")
    else:
        transport = (reward_path.is_file() and reward_path.read_text().strip() == str(reward)
                     and not reward_json.exists() and exception is None
                     and verifier == {"rewards": {"reward": reward}})
    checks = {
        "expected_grade": status == expected and reward == REWARDS[expected],
        "adapter_agreement": (assessment.get("scientific_status") == status
                              and assessment.get("reward") == reward
                              and not assessment.get("infrastructure_error")),
        "submission_identity": assessment.get("submission_sha256") == submission_hash,
        "oracle_identity": (assessment.get("oracle_sha256") == oracle_hash
                            and oracle.get("oracle_version") == VERSION),
        "target_criterion": (criterion is None
                             or oracle["criteria"][criterion]["status"] == "fail"),
        "bounded_smoke": oracle.get("execution", {}).get("full_equilibration_executed") is False,
        "separate_verifier": result.get("verifier_environment_mode") == "separate",
        "agent_verifier_absence": "agent phase: verifier files absent" in
                                  (trial / "agent/oracle.txt").read_text(),
        "forged_artifacts_ignored": transport,
    }
    return {"passed": all(checks.values()), "observed": status, "reward": reward,
            "checks": checks, "harbor_exception": exception_type}


def publish(output, destination):
    if destination.exists():
        raise ValueError("Use a fresh public archive directory")
    report = json.loads((output / "report.json").read_text())
    if not report["complete"] or not report["complete_selection"] or not report["harbor_admission_passed"]:
        raise ValueError("Only a complete successful admission can be published")
    verify_files(ROOT, report["sources"])
    task = {"id": "P3", "criteria": json.loads((ROOT / "research/harbor/tasks.json").read_text())
            ["tasks"]["P3"]["criteria"]}
    for case in report["cases"]:
        submission = output / case["case"] / "tasks/nb-impact/solution/reference.in"
        if digest(submission) != case["submission_sha256"]:
            raise ValueError("Admission submission changed before publication")
        checked = assessment_checks(output / case["trial"], case["expected"], case["criterion"],
            case["submission_sha256"], report["sources"]["tools/p3_oracle.py"], task)
        if not checked["passed"] or not case["passed"]:
            raise ValueError("Admission evidence changed before publication")
    for name in report["sources"]:
        target = destination / "sources" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
    for case in report["cases"]:
        trial = output / case["trial"]
        for name in ("result.json", "agent/oracle.txt", "verifier/assessment.json",
                     "verifier/oracle.stdout", "verifier/oracle.stderr", "verifier/reward.txt"):
            source = trial / name
            if source.is_file():
                target = destination / case["case"] / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target)
        shutil.copyfile(output / case["case"] / "tasks/nb-impact/solution/reference.in",
                        destination / case["case"] / "solution.in")
        case["original_trial"] = case["trial"]
        case["trial"] = case["case"]
    write_json(destination / "report.json", report)
    write_json(destination / "plan.json", json.loads((output / "plan.json").read_text()))
    write_json(destination / "hashes.json", {str(p.relative_to(destination)): digest(p)
        for p in sorted(destination.rglob("*")) if p.is_file()})


def admit(args):
    images = json.loads((args.container / "images.json").read_text())
    verify_files(args.container, images.get("files"))
    task_source = Path(images["task_directory"])
    oracle_hash = digest(ROOT / "tools/p3_oracle.py")
    if (digest(args.container / "context/p3_oracle.py") != oracle_hash
            or digest(task_source / "tests/verifier.py") != digest(ROOT / "tools/harbor_verifier.py")
            or digest(task_source / "solution/reference.in") != digest(ROOT / "research/harbor/p3/reference.in")):
        raise ValueError("Container task does not contain the current frozen verifier and reference")
    task = json.loads((task_source / "tests/task.json").read_text())
    specification = json.loads((ROOT / "research/harbor/tasks.json").read_text())
    if task != {"id": "P3", "criteria": specification["tasks"]["P3"]["criteria"]}:
        raise ValueError("Container task criteria changed")
    docker = ["docker"]
    environment = os.environ.copy()
    if args.docker_config:
        docker += ["--config", str(args.docker_config)]
        environment["DOCKER_CONFIG"] = str(args.docker_config)
    if args.docker_host:
        docker += ["--host", args.docker_host]
        environment["DOCKER_HOST"] = args.docker_host
    for stage, tag in images["local_image_tags"].items():
        observed = subprocess.check_output([*docker, "image", "inspect", tag,
                                             "--format", "{{.Id}}"], text=True).strip()
        if observed != images["images"][stage]:
            raise ValueError("Local image tag no longer matches the admitted image")
    embedded = subprocess.check_output([*docker, "run", "--rm", "--network", "none",
        "--entrypoint", "sha256sum", images["local_image_tags"]["verifier"],
        "/opt/lamlet-bench/oracle"], text=True).split()[0]
    if embedded != oracle_hash:
        raise ValueError("Verifier image contains a different oracle")
    harbor_version = subprocess.check_output([args.harbor, "--version"], text=True).strip()
    if harbor_version != specification["harbor"]["version"]:
        raise ValueError("Require the pinned Harbor version")
    selected = list(cases((task_source / "solution/reference.in").read_text()))
    if args.case:
        selected = [case for case in selected if case[0] == args.case]
        if not selected:
            raise ValueError(f"Unknown admission case: {args.case}")
    if args.output.exists():
        raise ValueError("Use a fresh Harbor admission output directory")
    args.output.mkdir(parents=True)
    plan = {"schema_version": 2, "harbor_version": harbor_version, "oracle_version": VERSION,
            "sources": {name: digest(ROOT / name) for name in SOURCES},
            "images": images, "complete_selection": args.case is None,
            "cases": [{"case": name, "expected": expected, "criterion": criterion,
                       "submission_sha256": hashlib.sha256(text.encode()).hexdigest()}
                      for name, text, expected, criterion in selected],
            "per_case_timeout_seconds": 600, "model_trials_run": False,
            "full_equilibration_executed": False}
    write_json(args.output / "plan.json", plan)
    report = {**plan, "cases": [], "complete": False, "harbor_admission_passed": False}
    write_json(args.output / "report.json", report)
    started_all = time.perf_counter()
    for name, text, expected, criterion in selected:
        verify_files(ROOT, plan["sources"])
        verify_files(args.container, images["files"])
        dataset = args.output / name / "tasks"
        task_directory = dataset / "nb-impact"
        shutil.copytree(task_source, task_directory)
        (task_directory / "solution/reference.in").write_text(text)
        solve = task_directory / "solution/solve.sh"
        solve.write_text(
            "#!/usr/bin/env bash\nset -eu\n"
            "test ! -e /tests/test.sh\ntest ! -e /tests/verifier.py\n"
            "test ! -e /opt/lamlet-bench/oracle\n"
            "echo 'agent phase: verifier files absent'\n"
            "mkdir -p /app/submission\ncp /solution/reference.in /app/submission/solution.in\n"
            f"printf '{0 if expected == 'correct' else 1}\\n' > /app/submission/reward.txt\n"
            "printf '{\"reward\":1}\\n' > /app/submission/reward.json\n"
            "printf '{\"scientific_status\":\"forged\"}\\n' > /app/submission/assessment.json\n")
        solve.chmod(0o755)
        jobs = args.output / name / "jobs"
        record = next(dict(item) for item in plan["cases"] if item["case"] == name)
        started = time.perf_counter()
        try:
            with (args.output / name / "harbor.log").open("w") as log:
                process = subprocess.run([args.harbor, "run", "--path", str(dataset), "--agent", "oracle",
                    "--n-attempts", "1", "--n-concurrent", "1", "--jobs-dir", str(jobs),
                    "--job-name", "admission"], stdout=log, stderr=subprocess.STDOUT,
                    env=environment, timeout=600)
            record["harbor_exit"] = process.returncode
            assessments = list(jobs.rglob("verifier/assessment.json"))
            if len(assessments) != 1:
                raise ValueError("Missing or ambiguous verifier evidence; inspect harbor.log")
            trial = assessments.pop().parents[1]
            record.update(assessment_checks(trial, expected, criterion,
                          record["submission_sha256"], oracle_hash, task))
            record["passed"] = record["passed"] and process.returncode == 0
            record["trial"] = str(trial.relative_to(args.output))
        except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
            record.update(passed=False, infrastructure_error=str(error))
        record["seconds"] = time.perf_counter() - started
        report["cases"].append(record)
        report["wall_seconds"] = time.perf_counter() - started_all
        report["complete"] = len(report["cases"]) == len(selected)
        report["harbor_admission_passed"] = report["complete"] and all(r["passed"] for r in report["cases"])
        report["outcomes"] = dict(sorted(Counter(r.get("observed", "infrastructure_failure")
                                                 for r in report["cases"]).items()))
        verify_files(ROOT, plan["sources"])
        write_json(args.output / "report.json", report)
        print(json.dumps(record, sort_keys=True), flush=True)
        if name == "reference":
            print(f"Reference took {record['seconds']:.1f}s; {len(selected)} cases estimated at "
                  f"{record['seconds'] * len(selected):.1f}s", flush=True)
            if not record["passed"]:
                break
    return 0 if report["harbor_admission_passed"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--harbor", default="harbor")
    parser.add_argument("--docker-config", type=Path)
    parser.add_argument("--docker-host")
    parser.add_argument("--case", help="Exact semantic case ID for a timing run")
    parser.add_argument("--publish", type=Path, help="Archive an existing complete admission without rerunning it")
    args = parser.parse_args()
    args.output = args.output.resolve()
    if args.publish:
        publish(args.output, args.publish.resolve())
        return 0
    if args.container is None:
        parser.error("--container is required for admission")
    args.container = args.container.resolve()
    if args.docker_config:
        args.docker_config = args.docker_config.resolve()
    return admit(args)


if __name__ == "__main__":
    raise SystemExit(main())
