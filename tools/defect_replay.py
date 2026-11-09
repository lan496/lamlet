#!/usr/bin/env python3
"""Replay a frozen natural-report pilot without invoking Lamlet."""

import argparse
import json
import math
from pathlib import Path
import shutil
import subprocess
import sys
import time

from corpus import write_json
from defect_corpus import MANIFEST, Post, audit, sha256

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "research/defects"


def replace_once(text, before, after):
    if text.count(before) != 1:
        raise ValueError(f"Expected exactly one source anchor: {before}")
    return text.replace(before, after, 1)


def scripts(case, cache, lock):
    source = next(source for source in lock["sources"] if source["id"] == case["source"])
    directory = cache / case["source"]
    posts = json.loads((directory / "complete.json").read_text())["posts"]
    if case["id"] == "orientation":
        text = " ".join("".join(Post(p["cooked"]).text) for p in posts)
        if case["before"] not in " ".join(text.split()):
            raise ValueError("Original orientation command is missing")
        return {variant: case[variant] + "\n" for variant in ("before", "after")}
    if case["id"] == "averaging":
        attachments = {a["url"]: a for a in source["attachments"]}
        original = (directory / attachments[case["attachment_url"]]["file"]).read_bytes().decode()
        return {"before": original, "after": replace_once(original, case["remove"], "")}
    if case["id"].startswith("gcmc-"):
        blocks = [block for post in posts if post["post_number"] == 1
                  for block in Post(post["cooked"]).code if case["code_anchor"] in block]
        if len(blocks) != 1:
            raise ValueError("Ambiguous or missing original GCMC input")
        original, = blocks
        if "steps" in case:
            original = replace_once(original, "run             1000000", f"run {case['steps']}")
            lines = original.splitlines(keepends=True)
            selected = [line for line in lines if line.split()[:1] == ["thermo_style"]]
            if len(selected) != 1:
                raise ValueError("Ambiguous thermo instrumentation")
            original = replace_once(original, selected.pop(),
                                    f"thermo_style custom {case['thermo_columns']}\n"
                                    "thermo_modify format float %.17g\n")
        return {"before": original, "after": replace_once(original, *case["replace"])}
    if case["id"] == "velocity":
        template = (FIXTURES / "velocity.in.template").read_text()
        return {variant: template.format(**case, units_option=option)
                for variant, option in (("before", ""), ("after", "units box"))}
    raise ValueError(f"Unknown replay: {case['id']}")


def thermo_rows(text, columns):
    """Read only complete, finite rows from the requested native thermo table."""
    active = False
    result = []
    for line in text.splitlines():
        tokens = line.split()
        if tokens == columns:
            active = True
            continue
        if active:
            try:
                values = [float(token) for token in tokens]
            except ValueError:
                active = False
                continue
            if len(values) != len(columns) or not all(map(math.isfinite, values)):
                raise ValueError("Incomplete or nonfinite native thermo row")
            result.append(dict(zip(columns, values)))
    if not result:
        raise ValueError("Requested native thermo table is missing")
    return result


def measurement(case, config, text):
    if case["id"] == "velocity":
        rows = thermo_rows(text, ["Step", "c_measured"])
        if len(rows) != 1 or rows[0]["Step"] != 0:
            raise ValueError("Expected one initial kinetic-energy sample")
        expected = 0.5 * case["mass"] * case["velocity"] ** 2 * config["metal_mvv2e"]
        return {"kinetic_energy_eV": rows[0]["c_measured"], "intended_energy_eV": expected,
                "relative_error": abs(rows[0]["c_measured"] / expected - 1)}
    if case["id"] == "gcmc-instrumented":
        rows = thermo_rows(text, ["Step", "Atoms", "KinEng", "c_NVT_temp", "c_Tfluid"])
        if [row["Step"] for row in rows] != list(range(0, case["steps"] + 1, 100)):
            raise ValueError("Missing instrumented sampling steps")
        comparisons = []
        for row in rows:
            atoms, energy = row["Atoms"], row["KinEng"]
            if atoms <= 1 or atoms != int(atoms) or energy <= 0:
                raise ValueError("Temperature normalization is undefined for this sample")
            independent = 2 * energy / (config["real_boltz"] * (3 * atoms - 3))
            comparisons.append({"step": int(row["Step"]), "atoms": int(atoms),
                                "kinetic_temperature_K": independent,
                                "thermostat_temperature_K": row["c_NVT_temp"],
                                "relative_error": abs(row["c_NVT_temp"] / independent - 1)})
        return {"samples": comparisons,
                "atom_count_changed": len({row["atoms"] for row in comparisons}) > 1,
                "max_relative_error": max(row["relative_error"] for row in comparisons)}
    return None


def run(engine, cache, output):
    lock = json.loads((FIXTURES / "source-lock.json").read_text())
    audit(MANIFEST, lock, cache)
    config = json.loads((FIXTURES / "replays.json").read_text())
    output.mkdir(parents=True, exist_ok=False)
    write_json(output / "replays.json", config)
    runtime = {"engine_sha256": sha256(engine.read_bytes()),
               "declared_source_commit": config["source_commit"],
               "source_commit_verified_by_runner": False,
               "checker_invoked": False}
    help_result = subprocess.run([str(engine), "-h"], capture_output=True, timeout=10)
    if help_result.returncode != 0:
        raise ValueError("Engine help failed")
    (output / "engine-help.txt").write_bytes(help_result.stdout)
    write_json(output / "runtime.json", runtime)
    report = {"schema_version": 1, "gold_defect_pairs": 0, "independent_review": "pending",
              "cases": [], "manifest_sha256": sha256(MANIFEST.read_bytes()),
              "source_lock_sha256": sha256((FIXTURES / "source-lock.json").read_bytes())}
    first_seconds = None
    variants = sum(len(scripts(case, cache, lock)) for case in config["cases"])
    for case in config["cases"]:
        result = {"id": case["id"], "source": case["source"], "kind": case["kind"], "variants": []}
        for variant, script in scripts(case, cache, lock).items():
            directory = output / case["id"] / variant
            directory.mkdir(parents=True)
            data = script.encode()
            (directory / "in.lammps").write_bytes(data)
            started = time.perf_counter()
            try:
                process = subprocess.run([str(engine), "-log", "none", "-echo", "none", "-in", "in.lammps"],
                                         cwd=directory, capture_output=True, timeout=config["timeout_seconds"])
                code, stdout, stderr = process.returncode, process.stdout, process.stderr
            except subprocess.TimeoutExpired as error:
                code, stdout, stderr = None, error.stdout or b"", error.stderr or b""
            seconds = time.perf_counter() - started
            (directory / "engine.stdout").write_bytes(stdout)
            (directory / "engine.stderr").write_bytes(stderr)
            measured, error = None, None
            if code == 0:
                try:
                    measured = measurement(case, config, stdout.decode())
                except ValueError as exc:
                    error = str(exc)
            result["variants"].append({"variant": variant, "input_sha256": sha256(data),
                                       "engine_exit_code": code, "seconds": seconds,
                                       "timed_out": code is None, "measurement": measured,
                                       "measurement_error": error})
            if first_seconds is None:
                first_seconds = seconds
                report["budget"] = {"first_case_seconds": seconds,
                                    "derived_remaining_seconds_from_first": seconds * (variants - 1),
                                    "remaining_timeout_ceiling_seconds": config["timeout_seconds"] * (variants - 1),
                                    "heterogeneous_inputs_limit_timing_extrapolation": True}
            print(f"{case['id']}/{variant}: exit={code}, {seconds:.3f}s", file=sys.stderr)
        report["cases"].append(result)
        write_json(output / "report.json", report)
    return report


def publish(run_directory, output):
    output.mkdir(parents=True, exist_ok=False)
    for name in ("replays.json", "runtime.json", "engine-help.txt", "report.json"):
        shutil.copyfile(run_directory / name, output / name)
    report = json.loads((run_directory / "report.json").read_text())
    # Source inputs and their generated output files can have unknown licenses.
    # Publish receipts and engine observations only. -echo none suppresses input echo.
    for case in report["cases"]:
        for variant in case["variants"]:
            relative = Path(case["id"]) / variant["variant"]
            (output / relative).mkdir(parents=True)
            for name in ("engine.stdout", "engine.stderr"):
                shutil.copyfile(run_directory / relative / name, output / relative / name)
    write_json(output / "inventory.json", {
        str(path.relative_to(output)): sha256(path.read_bytes())
        for path in sorted(output.rglob("*")) if path.is_file()})


def verify(directory, cache=None):
    inventory = json.loads((directory / "inventory.json").read_text())
    actual = {str(p.relative_to(directory)) for p in directory.rglob("*")
              if p.is_file() and p.name != "inventory.json"}
    if set(inventory) != actual:
        raise ValueError("Replay archive inventory differs from actual files")
    for name, digest in inventory.items():
        path = directory / name
        if not path.resolve().is_relative_to(directory.resolve()) or sha256(path.read_bytes()) != digest:
            raise ValueError(f"Changed replay archive: {name}")
    config = json.loads((directory / "replays.json").read_text())
    report = json.loads((directory / "report.json").read_text())
    if report["manifest_sha256"] != sha256(MANIFEST.read_bytes()):
        raise ValueError("Replay refers to a different screening manifest")
    lock_path = FIXTURES / "source-lock.json"
    if report["source_lock_sha256"] != sha256(lock_path.read_bytes()):
        raise ValueError("Replay refers to a different source snapshot")
    lock = json.loads(lock_path.read_text())
    if cache:
        audit(MANIFEST, lock, cache)
    cases = {case["id"]: case for case in config["cases"]}
    if (len(cases) != len(config["cases"]) or len(report["cases"]) != len(cases)
            or {case["id"] for case in report["cases"]} != set(cases)):
        raise ValueError("Missing or duplicate replay cases")
    for result in report["cases"]:
        case = cases[result["id"]]
        if (result["source"], result["kind"]) != (case["source"], case["kind"]):
            raise ValueError("Changed replay provenance")
        if len(result["variants"]) != 2 or {v["variant"] for v in result["variants"]} != {"before", "after"}:
            raise ValueError("Missing or duplicate replay variant")
        inputs = scripts(case, cache, lock) if cache else None
        for variant in result["variants"]:
            if variant["timed_out"] != (variant["engine_exit_code"] is None):
                raise ValueError("Inconsistent timeout outcome")
            if not math.isfinite(variant["seconds"]) or variant["seconds"] < 0:
                raise ValueError("Invalid elapsed time")
            if inputs and sha256(inputs[variant["variant"]].encode()) != variant["input_sha256"]:
                raise ValueError("Replay input differs from source transformation")
            if variant["engine_exit_code"] == 0:
                text = (directory / result["id"] / variant["variant"] / "engine.stdout").read_text()
                try:
                    observed = measurement(cases[result["id"]], config, text)
                except ValueError as error:
                    if variant["measurement"] is not None or variant["measurement_error"] != str(error):
                        raise ValueError("Changed unassessed measurement") from error
                else:
                    if observed != variant["measurement"] or variant["measurement_error"] is not None:
                        raise ValueError("Changed independent measurement")
            elif variant["measurement"] is not None or variant["measurement_error"] is not None:
                raise ValueError("Unsuccessful execution cannot supply a completed measurement")
    return {"verified_cases": len(cases), "verified_files": len(inventory),
            "source_transformations_verified": cache is not None}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("run", "publish", "verify"))
    parser.add_argument("--engine", type=Path)
    parser.add_argument("--cache", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--run", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "verify":
            if args.run is None:
                parser.error("verify requires --run")
            print(json.dumps(verify(args.run, args.cache), sort_keys=True))
        elif args.command == "publish":
            if not all((args.run, args.output)):
                parser.error("publish requires --run and --output")
            publish(args.run, args.output)
            print(json.dumps(verify(args.output), sort_keys=True))
        else:
            if not all((args.engine, args.cache, args.output)):
                parser.error("run requires --engine, --cache, and --output")
            report = run(args.engine.resolve(), args.cache, args.output)
            print(json.dumps({"cases": len(report["cases"]), "gold_defect_pairs": 0}, sort_keys=True))
        return 0
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
