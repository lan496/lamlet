#!/usr/bin/env python3
"""Run reference, mutation, and artifact-forgery admission through real Harbor."""

import argparse
import json
from pathlib import Path
import shutil
import subprocess

from corpus import write_json


def admit(args):
    if args.output.exists():
        raise ValueError("Use a fresh Harbor admission output directory")
    args.output.mkdir(parents=True)
    images = json.loads((args.container / "images.json").read_text())
    docker = ["docker"]
    if args.docker_config:
        docker += ["--config", str(args.docker_config)]
    if args.docker_host:
        docker += ["--host", args.docker_host]
    for stage, tag in images["local_image_tags"].items():
        observed = subprocess.check_output([*docker, "image", "inspect", tag,
                                             "--format", "{{.Id}}"], text=True).strip()
        if observed != images["images"][stage]:
            raise ValueError("Local image tag no longer matches the admitted image")
    records = []
    for name, expected, reward in (("reference", "correct", 1), ("wrong-gap", "incorrect", 0)):
        dataset = args.output / name / "tasks"
        task = dataset / "nb-impact"
        shutil.copytree(Path(images["task_directory"]), task)
        reference = task / "solution/reference.in"
        if name == "wrong-gap":
            text = reference.read_text()
            if text.count("39.5*v_a+15-60*v_a") != 1:
                raise ValueError("Wrong-gap mutation no longer matches the reference")
            reference.write_text(text.replace("39.5*v_a+15-60*v_a", "39.5*v_a+1.5-60*v_a"))
        solve = task / "solution/solve.sh"
        solve.write_text(
            "#!/usr/bin/env bash\nset -eu\n"
            "test ! -e /tests/test.sh\ntest ! -e /tests/verifier.py\n"
            "test ! -e /opt/lamlet-bench/oracle\n"
            "echo 'agent phase: verifier files absent'\n"
            "mkdir -p /app/submission\ncp /solution/reference.in /app/submission/solution.in\n"
            # Both cases submit the opposite reward. The verifier must ignore it.
            f"printf '{1 - reward}\\n' > /app/submission/reward.txt\n"
            "printf '{\"scientific_status\":\"forged\"}\\n' > /app/submission/assessment.json\n")
        solve.chmod(0o755)
        jobs = args.output / name / "jobs"
        with (args.output / name / "harbor.log").open("w") as log:
            subprocess.run([args.harbor, "run", "--path", str(dataset), "--agent", "oracle",
                            "--n-attempts", "1", "--n-concurrent", "1", "--jobs-dir", str(jobs),
                            "--job-name", "admission"], stdout=log, stderr=subprocess.STDOUT,
                           check=True, timeout=600)
        assessments = list(jobs.rglob("verifier/assessment.json"))
        if len(assessments) != 1:
            raise ValueError(f"{name}: missing or ambiguous verifier evidence; inspect harbor.log")
        assessment = json.loads(assessments[0].read_text())
        trial = assessments[0].parents[1]
        agent_log = (trial / "agent/oracle.txt").read_text()
        passed = (assessment["scientific_status"] == expected and assessment["reward"] == reward
                  and "agent phase: verifier files absent" in agent_log
                  and (trial / "verifier/reward.txt").read_text().strip() == str(reward))
        records.append({"case": name, "passed": passed, "expected": expected,
                        "observed": assessment["scientific_status"], "reward": assessment["reward"],
                        "assessment": str(assessments[0]), "agent_verifier_absence_checked": True,
                        "forged_artifact_reward_ignored": passed})
    report = {"schema_version": 1, "harbor_admission_passed": all(r["passed"] for r in records),
              "cases": records, "images": images["images"], "model_trials_run": False,
              "full_equilibration_executed": False}
    write_json(args.output / "report.json", report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["harbor_admission_passed"] else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--harbor", default="harbor")
    parser.add_argument("--docker-config", type=Path)
    parser.add_argument("--docker-host")
    raise SystemExit(admit(parser.parse_args()))
