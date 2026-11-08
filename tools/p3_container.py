#!/usr/bin/env python3
"""Build isolated P3 engine images and prepare one local Harbor reference task."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

from corpus import write_json


ROOT = Path(__file__).resolve().parents[1]


def build(args):
    if args.output.exists():
        raise ValueError("Use a fresh container output directory")
    args.output.mkdir(parents=True)
    context = args.output / "context"
    source = args.runtime / "source"
    pin = json.loads((ROOT / "research/harbor/p3/runtime.json").read_text())["lammps_commit"]
    if subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip() != pin:
        raise ValueError("Runtime source does not match the P3 pin")
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=source).strip():
        raise ValueError("Modified runtime source")
    for directory in ("src", "cmake", "python/lammps"):
        shutil.copytree(source / directory, context / "source" / directory,
                        ignore=shutil.ignore_patterns("__pycache__"))
    shutil.copyfile(source / "LICENSE", context / "source/LICENSE")
    shutil.copyfile(ROOT / "research/harbor/p3/Dockerfile", context / "Dockerfile")
    shutil.copyfile(ROOT / "tools/p3_oracle.py", context / "p3_oracle.py")
    docker = ["docker"]
    if args.docker_config:
        docker += ["--config", str(args.docker_config)]
    if args.docker_host:
        docker += ["--host", args.docker_host]
    images = {}
    local_tags = {}
    for stage in ("runtime", "verifier"):
        tag = f"lamlet-p3-{stage}:v1"
        with (args.output / f"{stage}-build.log").open("w") as log:
            subprocess.run([*docker, "build", "--target", stage, "-t", tag, str(context)],
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        images[stage] = subprocess.check_output([*docker, "image", "inspect", tag, "--format", "{{.Id}}"], text=True).strip()
        local_tags[stage] = f"lamlet-p3-{stage}:" + images[stage].removeprefix("sha256:")
        subprocess.run([*docker, "tag", images[stage], local_tags[stage]], check=True)
    dataset = args.output / "tasks"
    subprocess.run([sys.executable, str(ROOT / "tools/harbor_tasks.py"), "--condition", "A",
                    "--task", "P3", "--output", str(dataset)], check=True, stdout=subprocess.DEVNULL)
    task = dataset / "nb-impact"
    # Local image configuration IDs pin the tested images on this Docker daemon.
    # They are not registry distribution digests and are not portable image names.
    for folder, stage in (("environment", "runtime"), ("tests", "verifier")):
        path = task / folder / "Dockerfile"
        text = path.read_text()
        text = text.replace("# Draft: supply an audited image digest before building.\nARG BASE_IMAGE\nFROM ${BASE_IMAGE}", "FROM " + local_tags[stage])
        path.write_text(text)
    instruction = task / "instruction.md"
    instruction.write_text(instruction.read_text() + "\nPilot convention: the initial gap is 15 Angstrom between the outermost atomic planes. Use exactly 16000 projectile atoms and 32000 target atoms.\n")
    preparation_path = dataset / "preparation.json"
    preparation = json.loads(preparation_path.read_text())
    preparation["tasks"][0]["instruction_sha256"] = hashlib.sha256(instruction.read_bytes()).hexdigest()
    preparation["local_image_ids"] = images
    preparation["local_image_tags"] = local_tags
    write_json(preparation_path, preparation)
    solution = task / "solution"
    solution.mkdir()
    shutil.copyfile(ROOT / "research/harbor/p3/reference.in", solution / "reference.in")
    (solution / "solve.sh").write_text("#!/usr/bin/env bash\nset -eu\nmkdir -p /app/submission\ncp /solution/reference.in /app/submission/solution.in\n")
    (solution / "solve.sh").chmod(0o755)
    report = {"schema_version": 1, "condition": "A", "task": "P3", "images": images,
              "local_image_tags": local_tags, "lammps_commit": pin, "task_directory": str(task),
              "ready_for_model_trials": False, "scope": "local reference admission"}
    report["files"] = {str(path.relative_to(args.output)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(task.rglob("*")) if path.is_file()}
    for name in ("Dockerfile", "p3_oracle.py"):
        report["files"]["context/" + name] = hashlib.sha256((context / name).read_bytes()).hexdigest()
    write_json(args.output / "images.json", report)
    print(json.dumps(report, indent=2, sort_keys=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", type=Path, default=ROOT / "target/p3-runtime")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--docker-config", type=Path)
    parser.add_argument("--docker-host")
    args = parser.parse_args()
    args.output, args.runtime = args.output.resolve(), args.runtime.resolve()
    if args.docker_config:
        args.docker_config = args.docker_config.resolve()
    build(args)


if __name__ == "__main__":
    main()
