#!/usr/bin/env python3
"""Measure dimer constraints independently of Lamlet's findings.

Run one case before estimating the remaining serial sweep. Keep every engine
outcome, including errors and missing trajectories, in a fresh output directory.
"""

import argparse
import hashlib
import json
import math
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "research/physical-semantics"


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def measure_trajectory(path, steps, bond_length):
    """Require a complete, finite two-atom trajectory and return both residuals."""
    lines = iter(path.read_text().splitlines())
    max_distance = max_radial = 0.0
    for step in range(steps + 1):
        try:
            if next(lines) != "ITEM: TIMESTEP" or int(next(lines)) != step:
                raise ValueError("Missing or unexpected timestep")
            if next(lines) != "ITEM: NUMBER OF ATOMS" or int(next(lines)) != 2:
                raise ValueError("Expected exactly two atoms")
            if not next(lines).startswith("ITEM: BOX BOUNDS"):
                raise ValueError("Missing box bounds")
            for _ in range(3):
                bounds = [float(x) for x in next(lines).split()]
                if len(bounds) != 2 or not all(map(math.isfinite, bounds)):
                    raise ValueError("Invalid orthogonal box bounds")
            if next(lines) != "ITEM: ATOMS id xu yu zu vx vy vz":
                raise ValueError("Unexpected trajectory columns")
            atoms = {}
            for _ in range(2):
                row = next(lines).split()
                if len(row) != 7:
                    raise ValueError("Unexpected atom columns")
                atom_id, values = int(row[0]), [float(x) for x in row[1:]]
                if atom_id in atoms or not all(map(math.isfinite, values)):
                    raise ValueError("Duplicate atom ID or nonfinite trajectory")
                atoms[atom_id] = values
            if set(atoms) != {1, 2}:
                raise ValueError("Unexpected atom IDs")
        except StopIteration as exc:
            raise ValueError("Incomplete trajectory") from exc
        displacement = [atoms[2][i] - atoms[1][i] for i in range(3)]
        velocity = [atoms[2][i] - atoms[1][i] for i in range(3, 6)]
        distance = math.sqrt(sum(x * x for x in displacement))
        if distance == 0:
            raise ValueError("Coincident atoms")
        radial = abs(sum(r * v for r, v in zip(displacement, velocity))) / distance
        max_distance = max(max_distance, abs(distance / bond_length - 1))
        max_radial = max(max_radial, radial)
    if next(lines, None) is not None:
        raise ValueError("Unexpected extra trajectory frames")
    return {
        "frames": steps + 1,
        "max_relative_bond_error": max_distance,
        "max_radial_relative_velocity": max_radial,
    }


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, sort_keys=True, allow_nan=False) + "\n")


def verify_archive(directory):
    inventory = json.loads((directory / "inventory.json").read_text())
    actual = {str(p.relative_to(directory)) for p in directory.rglob("*")
              if p.is_file() and p.name != "inventory.json"}
    if set(inventory) != actual:
        raise ValueError("Archive inventory differs from its files")
    for name, digest in inventory.items():
        path = directory / name
        if not path.resolve().is_relative_to(directory.resolve()) or sha256(path) != digest:
            raise ValueError(f"Archive hash mismatch: {name}")
    config = json.loads((directory / "cases.json").read_text())
    report = json.loads((directory / "report.json").read_text())
    cases = {case["id"]: case for case in config["cases"]}
    if len(report["cases"]) != len(cases) or {c["id"] for c in report["cases"]} != set(cases):
        raise ValueError("Missing or duplicate case results")
    for result in report["cases"]:
        if result["measurement"] is not None:
            measured = measure_trajectory(directory / result["id"] / "trajectory.dump",
                                          config["steps"], config["bond_length"])
            if measured != result["measurement"]:
                raise ValueError(f"Measurement mismatch: {result['id']}")
    print(f"Verified {len(cases)} cases and {len(inventory)} file hashes")


def run_case(case, config, engine, checker, output):
    directory = output / case["id"]
    directory.mkdir()
    shutil.copyfile(FIXTURES / "dimer.data", directory / "dimer.data")
    script = (FIXTURES / "dimer.in.template").read_text().format(
        fixes="\n".join(case["fixes"])
    )
    (directory / "in.lammps").write_text(script)
    started = time.perf_counter()
    try:
        process = subprocess.run(
            [str(engine), "-log", "none", "-in", "in.lammps"],
            cwd=directory, capture_output=True, text=True, timeout=10, check=False,
        )
        returncode, stdout, stderr = process.returncode, process.stdout, process.stderr
    except subprocess.TimeoutExpired as exc:
        returncode = None
        stdout = (exc.stdout or b"").decode()
        stderr = (exc.stderr or b"").decode() + "\nTimed out after 10 seconds\n"
    elapsed = time.perf_counter() - started
    (directory / "engine.stdout").write_text(stdout)
    (directory / "engine.stderr").write_text(stderr)
    metrics, reason = None, None
    try:
        if returncode != 0:
            raise ValueError("Engine did not exit successfully")
        metrics = measure_trajectory(
            directory / "trajectory.dump", config["steps"], config["bond_length"]
        )
    except (OSError, ValueError) as exc:
        reason = str(exc)
    result = subprocess.run(
        [str(checker), "check", "in.lammps", "--format", "json"],
        cwd=directory, capture_output=True, text=True, timeout=10, check=False,
    )
    (directory / "checker.json").write_text(result.stdout)
    (directory / "checker.stderr").write_text(result.stderr)
    diagnostic = json.loads(result.stdout)
    warning = any(d["code"] == case["rule"] for d in diagnostic["diagnostics"])
    satisfies = None if metrics is None else (
        metrics["max_relative_bond_error"] <= config["distance_threshold"]
        and (case["constraint"] != "rattle" or
             metrics["max_radial_relative_velocity"] <= config["radial_velocity_threshold"])
    )
    return {
        "id": case["id"], "rule": case["rule"], "order": case["order"],
        "engine_exit_code": returncode, "engine_seconds": elapsed,
        "measurement": metrics, "unassessed_reason": reason,
        "constraints_within_threshold": satisfies,
        "checker_exit_code": result.returncode, "ordering_warning": warning,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path)
    parser.add_argument("--checker", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--source", type=Path, default=ROOT / "target/p3-runtime/source",
                        help="LAMMPS checkout used to build the selected engine")
    parser.add_argument("--verify", type=Path, help="Verify archived hashes and remeasure trajectories")
    args = parser.parse_args()
    if args.verify:
        verify_archive(args.verify)
        return
    if not all((args.engine, args.checker, args.output)):
        parser.error("--engine, --checker and --output are required for a new run")
    engine, checker, output = args.engine.resolve(), args.checker.resolve(), args.output
    config = json.loads((FIXTURES / "cases.json").read_text())
    output.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(FIXTURES / "cases.json", output / "cases.json")
    help_result = subprocess.run([str(engine), "-h"], capture_output=True, text=True, check=True)
    (output / "engine-help.txt").write_text(help_result.stdout)
    source = args.source
    source_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
    if source_commit != config["lammps_commit"]:
        raise ValueError("Native source does not match the runtime pin")
    results = []
    for case in config["cases"]:
        result = run_case(case, config, engine, checker, output)
        results.append(result)
        if len(results) == 1:
            estimate = result["engine_seconds"] * (len(config["cases"]) - 1)
            print(f"First native case: {result['engine_seconds']:.3f}s; remaining estimate {estimate:.3f}s", flush=True)
        print(f"{case['id']}: exit={result['engine_exit_code']} constraints={result['constraints_within_threshold']} warning={result['ordering_warning']}", flush=True)
    write_json(output / "report.json", {
        "schema_version": 1, "lammps_source_commit": source_commit,
        "engine_sha256": sha256(engine), "checker_sha256": sha256(checker),
        "cases": results,
    })
    write_json(output / "inventory.json", {
        str(path.relative_to(output)): sha256(path)
        for path in sorted(output.rglob("*")) if path.is_file()
    })


if __name__ == "__main__":
    main()
