#!/usr/bin/env python3
"""Build a repository-local serial LAMMPS runtime for the P3 pilot."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from corpus import write_json


ROOT = Path(__file__).resolve().parents[1]


def build(directory, jobs):
    specification = json.loads((ROOT / "research/harbor/p3/runtime.json").read_text())
    source = directory / "source"
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / "build.log").open("w") as log:
        def run(argv, cwd=ROOT):
            subprocess.run(argv, cwd=cwd, stdout=log, stderr=subprocess.STDOUT, check=True)
        if not source.exists():
            run(["git", "clone", "--filter=blob:none", "--no-checkout", "--depth", "1",
                 specification["lammps_repository"], str(source)])
        remote = subprocess.check_output(["git", "config", "--get", "remote.origin.url"], cwd=source, text=True).strip()
        if remote != specification["lammps_repository"]:
            raise ValueError("Unexpected LAMMPS origin")
        if subprocess.check_output(["git", "status", "--porcelain"], cwd=source).strip():
            raise ValueError("Refusing to change a dirty LAMMPS source checkout")
        run(["git", "fetch", "--depth", "1", "origin", specification["lammps_commit"]], source)
        run(["git", "sparse-checkout", "set", "--no-cone", "/src/", "/cmake/", "/python/", "/LICENSE"], source)
        run(["git", "checkout", "--detach", specification["lammps_commit"]], source)
        run(["cmake", "-S", str(source / "cmake"), "-B", str(directory / "build"), "-G", "Ninja",
             *specification["cmake_options"]])
        run(["cmake", "--build", str(directory / "build"), "--parallel", str(jobs)])
    libraries = list((directory / "build").glob("liblammps.*"))
    module = directory / "python/lammps"
    shutil.copytree(source / "python/lammps", module, dirs_exist_ok=True,
                    ignore=shutil.ignore_patterns("__pycache__"))
    for library in libraries:
        if library.suffix in (".so", ".dylib"):
            link = module / library.name
            link.unlink(missing_ok=True)
            link.symlink_to(library)
    report = {**specification, "libraries": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
              for p in libraries if p.is_file() and not p.is_symlink()}}
    write_json(directory / "runtime.json", report)
    print(json.dumps(report, indent=2, sort_keys=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=ROOT / "target/p3-runtime")
    parser.add_argument("--jobs", type=int, choices=range(1, 9), default=4)
    args = parser.parse_args()
    build(args.directory.resolve(), args.jobs)


if __name__ == "__main__":
    main()
