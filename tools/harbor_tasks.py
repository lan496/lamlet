#!/usr/bin/env python3
"""Generate draft Harbor P1-P3 tasks. Does not launch agents or certify readiness."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys

from corpus import git, write_json
import evaluation


ROOT = Path(__file__).resolve().parents[1]
SPECIFICATION = ROOT / "research/harbor/tasks.json"
TOOLS = {
    "A": "Use the shell and the installed LAMMPS engine.",
    "B": "Use the shell and LAMMPS. The pinned AST skill is at /opt/agent-tools/lammps-ast-eval/SKILL.md.",
    "C": "Use the shell, LAMMPS, and `lamlet check INPUT --language lammps --format json`. Query `lamlet schema --format json` for coverage.",
    "D": "Use the shell, LAMMPS, and Lamlet. Write typed Lamlet and build it with `lamlet build SOURCE -o /app/submission/solution.in --format json`.",
}


def pinned(cache, path, expected):
    data = (cache / path).read_bytes()
    blob = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
    if blob != expected:
        raise ValueError(f"Modified upstream resource: {path}")
    return data


def image(value):
    if not re.fullmatch(r"[a-zA-Z0-9._:/-]+@sha256:[0-9a-f]{64}", value):
        raise argparse.ArgumentTypeError("image must use an immutable @sha256 digest")
    return value


def base(image_name):
    if image_name:
        return f"FROM {image_name}\n"
    return "# Draft: supply an audited image digest before building.\nARG BASE_IMAGE\nFROM ${BASE_IMAGE}\n"


def prepare(args):
    specification = json.loads(SPECIFICATION.read_text(encoding="utf-8"))
    upstream = evaluation.source()
    if git(args.cache, "rev-parse", "HEAD").decode().strip() != upstream["commit"]:
        raise ValueError("Upstream checkout does not match the pinned revision")
    # Read and verify everything before publishing any tasks.
    resources = {}
    for task_id, task in specification["tasks"].items():
        if getattr(args, "task", None) and task_id != args.task:
            continue
        resources[task_id] = (
            pinned(args.cache, task["prompt"], task["prompt_blob"]),
            pinned(args.cache, task["potential"], task["potential_blob"]),
        )
    if args.output.exists():
        raise ValueError(f"Output already exists; choose a fresh directory: {args.output}")
    records = []
    for task_id, task in sorted(specification["tasks"].items()):
        if task_id not in resources:
            continue
        prompt, potential = resources[task_id]
        directory = args.output / task["name"]
        environment = directory / "environment"
        tests = directory / "tests"
        for folder in (environment / "inputs", tests / "inputs"):
            folder.mkdir(parents=True)
            (folder / Path(task["potential"]).name).write_bytes(potential)
        instruction = prompt.decode("utf-8").strip() + (
            "\n\nSubmit a complete LAMMPS input as `/app/submission/solution.in`. "
            "The potential file is in `/app/inputs/" + Path(task["potential"]).name + "`. "
            "The verifier runs independently on the submitted input.\n\n"
            + TOOLS[args.condition] + "\n"
        )
        (directory / "instruction.md").write_text(instruction, encoding="utf-8")
        (environment / "Dockerfile").write_text(
            base(args.agent_image) + "WORKDIR /app\nCOPY inputs/ /app/inputs/\n"
            "RUN mkdir -p /app/submission\n", encoding="utf-8"
        )
        (tests / "Dockerfile").write_text(
            base(args.verifier_image) + "COPY . /tests/\nCOPY inputs/ /app/inputs/\n",
            encoding="utf-8",
        )
        shutil.copyfile(ROOT / "tools/harbor_verifier.py", tests / "verifier.py")
        (tests / "test.sh").write_text(
            "#!/usr/bin/env bash\nexec python3 /tests/verifier.py\n", encoding="utf-8"
        )
        (tests / "test.sh").chmod(0o755)
        write_json(tests / "task.json", {"id": task_id, "criteria": task["criteria"]})
        (directory / "task.toml").write_text(
            'schema_version = "1.4"\nartifacts = ["/app/submission"]\n'
            f'source = "{upstream["repository"]}@{upstream["commit"]}"\n\n'
            f'[metadata]\nrelease_status = "draft"\ntask_id = "{task_id}"\n'
            f'condition = "{args.condition}"\nsplit = "development"\n\n'
            f'[agent]\ntimeout_sec = {args.agent_seconds}\n\n'
            '[environment]\ncpus = 2\nmemory_mb = 4096\nstorage_mb = 10240\n'
            'network_mode = "public"\n\n'
            f'[verifier]\ntimeout_sec = {args.verifier_seconds}\n'
            'environment_mode = "separate"\n\n'
            '[verifier.environment]\ncpus = 2\nmemory_mb = 4096\nstorage_mb = 10240\n'
            'network_mode = "no-network"\n', encoding="utf-8",
        )
        records.append({"id": task_id, "path": task["name"],
                        "prompt_sha256": hashlib.sha256(prompt).hexdigest(),
                        "instruction_sha256": hashlib.sha256(instruction.encode("utf-8")).hexdigest(),
                        "potential_sha256": hashlib.sha256(potential).hexdigest()})
    report = {
        "schema_version": 1, "ready_for_trials": False, "condition": args.condition,
        "harbor": specification["harbor"], "upstream": upstream["commit"],
        "agent_image": args.agent_image, "verifier_image": args.verifier_image,
        "agent_seconds": args.agent_seconds, "verifier_seconds": args.verifier_seconds,
        "development_tasks": [record["id"] for record in records],
        "held_out_tasks": specification["held_out_tasks"], "tasks": records,
        "remaining": ["independent scientific oracle and audited reference solutions",
                      "validated images with pinned engines, tools, and dependencies",
                      "frozen campaign budgets, model snapshot, and held-out tasks",
                      "Docker oracle runs, deliberate-mutation tests, and isolation tests"],
    }
    write_json(args.output / "preparation.json", report)
    print(json.dumps(report, indent=2, sort_keys=True))


def positive(value):
    number = int(value)
    if number <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return number


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--condition", choices=TOOLS, required=True)
    parser.add_argument("--task", choices=("P1", "P2", "P3"))
    parser.add_argument("--cache", type=Path, default=ROOT / "target/evaluation/upstream")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--agent-image", type=image)
    parser.add_argument("--verifier-image", type=image)
    parser.add_argument("--agent-seconds", type=positive, default=600)
    parser.add_argument("--verifier-seconds", type=positive, default=600)
    args = parser.parse_args()
    try:
        prepare(args)
        return 0
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
