#!/usr/bin/env python3
"""Freeze and run a six-trial development repair pilot through isolated Harbor tasks."""

import argparse
from collections import Counter
import datetime
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import subprocess
import time

from corpus import write_json
import repair_tasks


ROOT = Path(__file__).resolve().parents[1]
SOURCES = ("tools/repair_tasks.py", "tools/repair_campaign.py", "tools/repair_instrument.py",
           "research/repair/Dockerfile", "research/repair/protocol.md", "research/harbor/p3/runtime.json")
BASELINE = "lamlet-p3-campaign-a:2662225e6a26a873fdcd72b542b543003851896d9b32c4c75e7a1bd9112b2bad"
CHECKER = "lamlet-p3-campaign-c:6f0c19d2ffd11562715fe0fa77f36f51c15d1be563998fead0cbb7ef8e025203"
COMMON = """Repair the supplied input at /app/input.in and submit /app/submission/solution.in.
Work only in /app. Use ordinary, straight-line LAMMPS commands. Whitespace and
comments may change. Keep the other commands and parameters as required above.
The hidden verifier checks the edit contract independently and executes all of
the requested short runs in a fresh LAMMPS process. It is not available during
the task. Do not search for benchmark source or external solutions. You may
consult general documentation. Engine command: lmp -log none -in INPUT.
Validation commands record the tested input revisions and engine invocations.
Finish within 180 seconds. No verifier feedback will be returned during the trial.
"""


def digest(path):
    return repair_tasks.sha(path.read_bytes())


def frozen(root, files):
    for name, expected in files.items():
        if digest(root / name) != expected:
            raise ValueError("Frozen file changed: " + name)


def order(seed):
    generator = random.Random(seed)
    identities = sorted(k for k, task in repair_tasks.tasks().items() if task["split"] == "development")
    generator.shuffle(identities)
    trials = []
    for identity in identities:
        conditions = ["A", "C"]
        generator.shuffle(conditions)
        trials.extend({"id": identity + "-" + condition, "task": identity, "condition": condition}
                      for condition in conditions)
    return trials


def native_admission(engine, output):
    results = []
    started = time.perf_counter()
    selected = {k: v for k, v in repair_tasks.tasks().items() if v["split"] == "development"}
    cases = [(identity + "/" + name, task, candidate, expected)
             for identity, task in sorted(selected.items())
             for name, candidate, expected in (("reference", task["reference"], "correct"),
                 ("original-defect", task["input"], "incorrect"),
                 ("unsupported", task["reference"] + "run ${dynamic}\n", "unassessed"))]
    orientation = selected["orientation-dev"]
    cases.append(("orientation-dev/scaled-valid", orientation,
        orientation["reference"].replace("orient y -1 1 0", "orient y -2 2 0"), "correct"))
    for identity, task, candidate, expected in cases:
        begin = time.perf_counter()
        result = repair_tasks.assess(task, candidate, engine)
        results.append({"id": identity, "expected": expected, "assessment": result,
                        "passed": result["status"] == expected,
                        "seconds": time.perf_counter() - begin})
        if identity == "integrator-dev/reference":
            print(json.dumps({"representative_seconds": results[-1]["seconds"],
                              "derived_admission_seconds": len(cases) * results[-1]["seconds"]}), flush=True)
    report = {"cases": results, "passed": all(case["passed"] for case in results),
              "seconds": time.perf_counter() - started, "engine_sha256": digest(engine),
              "held_out_tasks_executed": 0, "model_trials": 0}
    write_json(output / "admission.json", report)
    if not report["passed"]:
        raise ValueError("Native oracle admission failed")


def task_directory(output, identity, condition, images):
    task = repair_tasks.tasks()[identity]
    directory = output / "tasks" / condition / identity
    environment, tests = directory / "environment", directory / "tests"
    environment.mkdir(parents=True)
    tests.mkdir()
    (environment / "input.in").write_text(task["input"])
    instruction = task["instruction"] + "\n\n" + COMMON
    if condition == "C":
        instruction += "\nLamlet is available: lamlet check INPUT --language lammps --format json.\n"
    (directory / "instruction.md").write_text(instruction)
    (environment / "Dockerfile").write_text(f"FROM {images[condition]['tag']}\nCOPY input.in /app/input.in\nWORKDIR /app\n")
    (tests / "Dockerfile").write_text(f"FROM {images['A']['tag']}\nCOPY . /tests/\n")
    shutil.copyfile(ROOT / "tools/repair_tasks.py", tests / "repair_tasks.py")
    (tests / "test.sh").write_text("#!/usr/bin/env bash\nset -eu\n"
        f"python3 /tests/repair_tasks.py --task {identity} --source /app/submission/solution.in "
        "--output /logs/verifier/assessment.json\n")
    (tests / "test.sh").chmod(0o755)
    (directory / "task.toml").write_text(
        'schema_version = "1.4"\nartifacts = ["/app/submission"]\n'
        f'[metadata]\ntask_id = "{identity}"\nsplit = "development"\ncondition = "{condition}"\n'
        '[agent]\ntimeout_sec = 180\n'
        '[environment]\ncpus = 2\nmemory_mb = 2048\nstorage_mb = 4096\nnetwork_mode = "public"\n'
        '[verifier]\ntimeout_sec = 60\nenvironment_mode = "separate"\n'
        '[verifier.environment]\ncpus = 2\nmemory_mb = 2048\nstorage_mb = 4096\nnetwork_mode = "no-network"\n')


def prepare(args):
    output = args.output
    if (output / "manifest.json").exists():
        raise ValueError("A frozen campaign already exists; choose a new directory")
    output.mkdir(parents=True, exist_ok=True)
    native_admission(args.engine.resolve(), output)
    context = output / "context"
    context.mkdir()
    shutil.copyfile(ROOT / "research/repair/Dockerfile", context / "Dockerfile")
    shutil.copyfile(ROOT / "tools/repair_instrument.py", context / "repair_instrument.py")
    for parent in (BASELINE, CHECKER):
        actual = subprocess.check_output(["docker", "image", "inspect", parent, "--format", "{{.Id}}"], text=True).strip()
        if actual != "sha256:" + parent.split(":")[-1]:
            raise ValueError("Parent image identity changed")
    images = {}
    for condition, stage in (("A", "common"), ("C", "checker")):
        tag = "lamlet-repair-" + condition.lower() + ":20261109"
        subprocess.run(["docker", "build", "--target", stage, "--build-arg", "BASELINE_IMAGE=" + BASELINE,
            "--build-arg", "CHECKER_IMAGE=" + CHECKER, "--tag", tag, str(context)], check=True)
        identity = subprocess.check_output(["docker", "image", "inspect", tag, "--format", "{{.Id}}"], text=True).strip()
        immutable_tag = "lamlet-repair-" + condition.lower() + ":" + identity.removeprefix("sha256:")
        subprocess.run(["docker", "tag", identity, immutable_tag], check=True)
        images[condition] = {"id": identity, "tag": immutable_tag}
    for identity, task in repair_tasks.tasks().items():
        if task["split"] == "development":
            for condition in ("A", "C"):
                task_directory(output, identity, condition, images)
    write_json(output / "task-contracts.json", repair_tasks.tasks())
    files = {str(path.relative_to(output)): digest(path) for path in sorted((output / "tasks").rglob("*")) if path.is_file()}
    files.update({name: digest(output / name) for name in ("task-contracts.json", "admission.json")})
    runtime = {}
    for name, condition, path in (("lamlet_sha256", "C", "/opt/lamlet/lamlet"),
                                  ("lammps_sha256", "A", "/opt/lammps/bin/lmp")):
        runtime[name] = subprocess.check_output(["docker", "run", "--rm", "--network", "none",
            "--entrypoint", "sha256sum", images[condition]["tag"], path], text=True).split()[0]
    manifest = {"schema_version": 1, "campaign": "repair-development-20261109", "seed": 496,
        "model": args.model, "reasoning_effort": args.effort, "service_tier": "fast",
        "codex_version": "0.166.0", "harbor_version": "0.25.0", "images": images,
        "runtime": runtime,
        "sources": {name: digest(ROOT / name) for name in SOURCES}, "files": files,
        "trials": order(496), "agent_seconds": 180, "maximum_agent_seconds": 1080,
        "verifier_seconds": 60, "tool_call_ceiling": None, "token_ceiling": None,
        "stopping_rule": "completion or 180-second agent timeout; no replacements",
        "split": "development", "independent_human_review": "pending",
        "held_out": [k for k, task in repair_tasks.tasks().items() if task["split"] == "held_out"],
        "backend_snapshot": "not exposed", "potential_files": [],
        "budget_estimate": "At most 1080 agent seconds plus setup and verification; dollar cost unmeasured",
        "unmeasured": ["dollar cost", "CPU seconds", "all unobserved source edits", "full scientific correctness"]}
    write_json(output / "manifest.json", manifest)
    write_json(output / "frozen.json", {"manifest_sha256": digest(output / "manifest.json")})
    (output / "state.md").write_text("# Repair development pilot\n\nNext action: run the first assigned trial for calibration.\n\n"
        "DONE: native admission and source/task/image freeze.\nTODO: six assigned trials.\n")
    print(json.dumps({"manifest_sha256": digest(output / "manifest.json"), "trials": manifest["trials"]}), flush=True)


def admit(args):
    output = args.output
    manifest = json.loads((output / "manifest.json").read_text())
    frozen(ROOT, manifest["sources"])
    frozen(output, manifest["files"])
    report_path = output / "harbor-admission.json"
    if report_path.exists():
        raise ValueError("Admission already recorded; do not replace it")
    cases = [(identity, variant, expected) for identity, task in sorted(repair_tasks.tasks().items())
             if task["split"] == "development"
             for variant, expected in (("reference", "correct"), ("input", "incorrect"))]
    cases.append(("sampling-dev", "unsupported", "unassessed"))
    records = []
    for identity, variant, expected in cases:
        task = repair_tasks.tasks()[identity]
        case = identity + "-" + variant
        dataset = output / "admission" / case / "tasks"
        directory = dataset / identity
        shutil.copytree(output / "tasks/A" / identity, directory)
        solution = directory / "solution"
        solution.mkdir()
        candidate = task[variant] if variant != "unsupported" else task["reference"] + "run ${dynamic}\n"
        (solution / "reference.in").write_text(candidate)
        (solution / "solve.sh").write_text("#!/usr/bin/env bash\nset -eu\n"
            "test ! -e /tests/repair_tasks.py\ntest ! -e /tests/test.sh\n"
            "echo 'verifier absent in agent environment'\n"
            "cp /solution/reference.in /app/submission/solution.in\n"
            "printf '1\\n' > /app/submission/reward.txt\n")
        (solution / "solve.sh").chmod(0o755)
        jobs = output / "admission" / case / "jobs"
        begin = time.perf_counter()
        with (output / "admission" / case / "harbor.log").open("w") as log:
            process = subprocess.run(["uv", "run", "--cache-dir", str(ROOT / "target/uv-cache"),
                "--python", "3.12", "--with", "harbor==0.25.0", "harbor", "run",
                "--path", str(dataset), "--agent", "oracle", "--n-attempts", "1",
                "--n-concurrent", "1", "--jobs-dir", str(jobs), "--job-name", "admission"],
                stdout=log, stderr=subprocess.STDOUT, timeout=300)
        found = list(jobs.glob("admission/" + identity + "__*/result.json"))
        if len(found) != 1:
            raise ValueError("Admission has no unique trial result; inspect " + case)
        root = found.pop().parent
        result = json.loads((root / "result.json").read_text())
        assessment = json.loads((root / "verifier/assessment.json").read_text())
        reward = (result.get("verifier_result") or {}).get("rewards", {}).get("reward")
        expected_reward = {"correct": 1, "incorrect": 0, "unassessed": None}[expected]
        agent_log = (root / "agent/oracle.txt").read_text()
        record = {"id": case, "expected": expected, "assessment": assessment, "reward": reward,
            "seconds": time.perf_counter() - begin, "separate_verifier": result.get("verifier_environment_mode") == "separate",
            "agent_verifier_absence": "verifier absent in agent environment" in agent_log,
            "exception_type": (result.get("exception_info") or {}).get("exception_type")}
        record["passed"] = (process.returncode == 0 and assessment["status"] == expected
            and reward == expected_reward and record["separate_verifier"] and record["agent_verifier_absence"]
            and record["exception_type"] == ("RewardFileNotFoundError" if expected == "unassessed" else None))
        records.append(record)
        print(json.dumps({"admission": case, "passed": record["passed"], "seconds": record["seconds"]}), flush=True)
        if case == "integrator-dev-reference":
            print(json.dumps({"derived_admission_seconds": len(cases) * record["seconds"]}), flush=True)
        if not record["passed"]:
            raise ValueError("Separate-verifier admission failed; inspect " + case)
    write_json(report_path, {"cases": records, "passed": all(r["passed"] for r in records),
        "manifest_sha256": digest(output / "manifest.json"), "model_trials": 0})


def interval_seconds(interval):
    if not interval or not interval.get("started_at") or not interval.get("finished_at"):
        return None
    return (datetime.datetime.fromisoformat(interval["finished_at"]) -
            datetime.datetime.fromisoformat(interval["started_at"])).total_seconds()


def measure(trial, job):
    records = list(job.glob(trial["task"] + "__*/result.json"))
    if len(records) != 1:
        return {**trial, "status": "unassessed", "infrastructure_error": "Missing Harbor trial result"}
    root = records.pop().parent
    result = json.loads((root / "result.json").read_text())
    assessment_path = root / "verifier/assessment.json"
    assessment = json.loads(assessment_path.read_text()) if assessment_path.is_file() else {"status": "unassessed"}
    reward = (result.get("verifier_result") or {}).get("rewards", {}).get("reward")
    return {**trial, "status": assessment["status"], "success": assessment["status"] == "correct" and reward == 1,
        "assessment": assessment, "reward": reward, "trial_directory": str(root.relative_to(job)),
        "agent_seconds": interval_seconds(result.get("agent_execution")),
        "exception_type": (result.get("exception_info") or {}).get("exception_type")}


def run(args):
    output = args.output
    manifest = json.loads((output / "manifest.json").read_text())
    if digest(output / "manifest.json") != json.loads((output / "frozen.json").read_text())["manifest_sha256"]:
        raise ValueError("Campaign manifest changed")
    admission = json.loads((output / "harbor-admission.json").read_text())
    if not admission["passed"] or admission["manifest_sha256"] != digest(output / "manifest.json"):
        raise ValueError("Require separate-verifier admission for this frozen campaign")
    environment = os.environ.copy()
    auth = Path(environment.get("CODEX_AUTH_JSON_PATH", str(Path.home() / ".codex/auth.json")))
    if not auth.is_file():
        raise ValueError("Current Codex authentication file is unavailable")
    environment["CODEX_AUTH_JSON_PATH"] = str(auth)
    results_path = output / "results.json"
    results = json.loads(results_path.read_text()) if results_path.exists() else {"trials": [], "complete": False}
    completed = {trial["id"] for trial in results["trials"]}
    for trial in manifest["trials"]:
        if trial["id"] in completed:
            continue
        frozen(ROOT, manifest["sources"])
        frozen(output, manifest["files"])
        for image in manifest["images"].values():
            actual = subprocess.check_output(["docker", "image", "inspect", image["tag"], "--format", "{{.Id}}"], text=True).strip()
            if actual != image["id"]:
                raise ValueError("Frozen local image changed")
        job = output / "jobs" / trial["id"]
        if job.exists():
            raise ValueError("Incomplete job exists; inspect rather than replacing it")
        config = {"job_name": trial["id"], "jobs_dir": str(output / "jobs"),
            "n_attempts": 1, "n_concurrent_trials": 1, "environment": {"type": "docker", "delete": True},
            "tasks": [{"path": str(output / "tasks" / trial["condition"] / trial["task"])}],
            "agents": [{"name": "codex", "model_name": "openai/" + manifest["model"],
                "kwargs": {"version": manifest["codex_version"], "reasoning_effort": manifest["reasoning_effort"],
                    "reasoning_summary": "none", "web_search": "disabled", "config": {"service_tier": "fast"}}}]}
        config_path = output / (trial["id"] + ".json")
        write_json(config_path, config)
        (output / "state.md").write_text(f"# Repair development pilot\n\nNext action: wait for {trial['id']}.\n\n"
            f"RUNNING: {trial['id']}. Progress: python3 tools/repair_campaign.py status --output {output}\n")
        begin = time.perf_counter()
        with (output / (trial["id"] + ".log")).open("w") as log:
            subprocess.run(["uv", "run", "--cache-dir", str(ROOT / "target/uv-cache"), "--python", "3.12",
                "--with", "harbor==0.25.0", "harbor", "run", "--config", str(config_path)],
                env=environment, stdout=log, stderr=subprocess.STDOUT, timeout=900)
        record = measure(trial, job)
        record["wall_seconds"] = time.perf_counter() - begin
        results["trials"].append(record)
        results["complete"] = len(results["trials"]) == len(manifest["trials"])
        write_json(results_path, results)
        if len(results["trials"]) == 1:
            write_json(output / "calibration.json", {"first_trial": trial["id"],
                "measured_wall_seconds": record["wall_seconds"], "remaining_assigned": 5,
                "derived_remaining_wall_seconds": 5 * record["wall_seconds"],
                "maximum_remaining_agent_seconds": 900, "estimate_is_not_a_measurement": True})
        print(json.dumps({k: record.get(k) for k in ("id", "status", "success", "wall_seconds", "exception_type")}), flush=True)
        if args.first_only:
            break
    (output / "state.md").write_text("# Repair development pilot\n\nNext action: " +
        ("publish the complete results." if results["complete"] else "run the remaining frozen trials.") +
        f"\n\nDONE: {len(results['trials'])}/6 assigned trials recorded. No replacements.\n")


def publish(args):
    output, destination = args.output, args.public
    results = json.loads((output / "results.json").read_text())
    manifest = json.loads((output / "manifest.json").read_text())
    frozen(ROOT, manifest["sources"])
    frozen(output, manifest["files"])
    if not results["complete"] or destination.exists():
        raise ValueError("Require all assigned outcomes and a new public archive")
    destination.mkdir(parents=True)
    for name in ("manifest.json", "frozen.json", "admission.json", "harbor-admission.json", "task-contracts.json", "calibration.json", "provider-probe.json"):
        shutil.copyfile(output / name, destination / name)
    for name in SOURCES:
        target = destination / "sources" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
    summary = {"trials": [], "conditions": {}, "complete": True}
    for trial in results["trials"]:
        public = destination / trial["id"]
        public.mkdir()
        row = {k: v for k, v in trial.items() if k not in ("assessment", "trial_directory")}
        root = output / "jobs" / trial["id"] / trial.get("trial_directory", "missing")
        write_json(public / "assessment.json", trial.get("assessment", {"status": "unassessed"}))
        artifact = root / "artifacts/app/submission"
        solution = artifact / "solution.in"
        if solution.is_file():
            shutil.copyfile(solution, public / "solution.in")
        events_file = artifact / "validation-events.jsonl"
        events = [json.loads(line) for line in events_file.read_text().splitlines()] if events_file.is_file() else []
        write_json(public / "validation-events.json", events)
        transitions = []
        for event in events:
            identity = event["candidate_sha256"]
            if identity and (not transitions or transitions[-1] != identity):
                transitions.append(identity)
        if solution.is_file() and (not transitions or transitions[-1] != digest(solution)):
            transitions.append(digest(solution))
        row.update(engine_launches=sum(e["tool"] == "lmp" for e in events),
                   lamlet_checks=sum(e["tool"] == "lamlet" for e in events),
                   validated_candidate_transitions=max(0, len(transitions) - 1) if solution.is_file() else None)
        trajectory_path = root / "agent/trajectory.json"
        trajectory = json.loads(trajectory_path.read_text()) if trajectory_path.is_file() else {"steps": []}
        calls = [{"step": step["step_id"], "tool_calls": step["tool_calls"]}
                 for step in trajectory["steps"] if step.get("tool_calls")]
        write_json(public / "tools.json", calls)
        row["tool_calls"] = sum(len(step["tool_calls"]) for step in calls)
        log = root / "agent/codex.txt"
        native = []
        if log.is_file():
            for line in log.read_text().splitlines():
                try:
                    native.append(json.loads(line))
                except ValueError:
                    pass
        usage = [event["usage"] for event in native if event.get("type") == "turn.completed"]
        row["tokens"] = {key: sum(u.get(key, 0) for u in usage) if usage else None
                         for key in ("input_tokens", "cached_input_tokens", "output_tokens")}
        row["provider_error_count"] = sum(event.get("type") == "error" for event in native)
        summary["trials"].append(row)
    for condition in ("A", "C"):
        selected = [row for row in summary["trials"] if row["condition"] == condition]
        statuses = Counter(row["status"] for row in selected)
        successes = sum(row.get("success", False) for row in selected)
        summary["conditions"][condition] = {"assigned": len(selected), **dict(statuses),
            "success_at_180_seconds": {"numerator": successes, "denominator": len(selected)},
            "engine_launches": sum(row["engine_launches"] for row in selected),
            "tool_calls": sum(row["tool_calls"] for row in selected)}
    write_json(destination / "results.json", summary)
    write_json(destination / "hashes.json", {str(p.relative_to(destination)): digest(p)
        for p in sorted(destination.rglob("*")) if p.is_file()})
    print(json.dumps(summary["conditions"], indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("prepare", "admit", "run", "publish", "status"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--engine", type=Path, default=ROOT / "target/p3-runtime/build/lmp")
    parser.add_argument("--model", default="gpt-6.1-sol")
    parser.add_argument("--effort", default="xhigh")
    parser.add_argument("--first-only", action="store_true")
    parser.add_argument("--public", type=Path)
    args = parser.parse_args()
    args.output = args.output.resolve()
    if args.command == "status":
        path = args.output / "results.json"
        results = json.loads(path.read_text()) if path.is_file() else {"trials": []}
        print(json.dumps({"recorded": len(results["trials"]), "assigned": 6,
                          "complete": results.get("complete", False)}))
    else:
        {"prepare": prepare, "admit": admit, "run": run, "publish": publish}[args.command](args)


if __name__ == "__main__":
    main()
