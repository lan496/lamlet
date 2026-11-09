#!/usr/bin/env python3
"""Measure checker output separately from the frozen independent repair oracle."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from corpus import write_json
from repair_campaign import digest, frozen, ROOT


def measure(campaign, output):
    manifest = json.loads((campaign / "manifest.json").read_text())
    contracts = json.loads((campaign / "task-contracts.json").read_text())
    frozen(campaign, manifest["files"])
    image = manifest["images"]["C"]
    actual = subprocess.check_output(["docker", "image", "inspect", image["tag"], "--format", "{{.Id}}"], text=True).strip()
    if actual != image["id"]:
        raise ValueError("Checker image changed")
    if output.exists():
        raise ValueError("Use a new checker-observation file")
    inputs = [(identity + "/" + label, identity, label, task[field].encode())
              for identity, task in sorted(contracts.items()) if task["split"] == "development"
              for label, field in (("reference", "reference"), ("original-defect", "input"))]
    results_path = campaign / "results.json"
    if results_path.is_file():
        for trial in json.loads(results_path.read_text())["trials"]:
            source = (campaign / "jobs" / trial["id"] / trial.get("trial_directory", "missing")
                      / "artifacts/app/submission/solution.in")
            if source.is_file():
                inputs.append((trial["id"], trial["task"], "submission", source.read_bytes()))
    observations = []
    with tempfile.TemporaryDirectory(dir=ROOT / "target") as temporary:
        source = Path(temporary) / "input.in"
        for identity, task_id, label, data in inputs:
            source.write_bytes(data)
            process = subprocess.run(["docker", "run", "--rm", "--network", "none",
                "--mount", f"type=bind,source={source},target=/work/input.in,readonly",
                "--entrypoint", "/opt/lamlet/lamlet", image["tag"], "check", "/work/input.in",
                "--language", "lammps", "--format", "json"], capture_output=True, text=True, timeout=30)
            if process.returncode not in (0, 1):
                raise ValueError("Checker infrastructure error: " + process.stderr)
            report = json.loads(process.stdout)
            diagnostics = report["diagnostics"]
            observations.append({"id": identity, "task": task_id, "label": label,
                "input_sha256": hashlib.sha256(data).hexdigest(),
                "target_rule": contracts[task_id]["rule"],
                "warnings": sorted(d["code"] for d in diagnostics if d["severity"] == "warning"),
                "errors": sorted(d["code"] for d in diagnostics if d["severity"] == "error"),
                "unknown": sorted(d["code"] for d in diagnostics if d["severity"] == "unknown"),
                "report": report})
    references = [r for r in observations if r["label"] == "reference"]
    defects = [r for r in observations if r["label"] == "original-defect"]
    result = {"checker_sha256": manifest["runtime"]["lamlet_sha256"], "cases": observations,
        "reference_cases": len(references), "references_with_warnings": sum(bool(r["warnings"]) for r in references),
        "defect_cases": len(defects), "target_defects_warned": sum(r["target_rule"] in r["warnings"] for r in defects),
        "independent_human_review": "pending", "used_for_oracle_grades": False}
    write_json(output, result)
    print(json.dumps({key: value for key, value in result.items() if key != "cases"}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    measure(arguments.campaign.resolve(), arguments.output.resolve())
