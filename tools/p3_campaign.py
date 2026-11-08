#!/usr/bin/env python3
"""Prepare and run a serial six-trial Codex A/C development pilot in Harbor."""

import argparse
from collections import Counter
from datetime import datetime
import gzip
import hashlib
import json
import os
from pathlib import Path
import random
import re
import subprocess
from types import SimpleNamespace

from corpus import findings, write_json
import harbor_tasks


ROOT = Path(__file__).resolve().parents[1]
COMMON = """
This is a bounded protocol-generation pilot. Submit the complete requested
schedule, but use short smoke tests during development; executing the full
100 ps is unnecessary. The initial gap is 15 Angstrom between the outermost
atomic planes. Use exactly 16000 projectile atoms and 32000 target atoms.
Use metal units, atomic atom style, axis-aligned BCC, static block regions with
explicit units, and the supplied EAM/alloy potential. The independent evaluator
supports straight-line inputs using these commands: units, dimension,
atom_style, boundary, variable, lattice, region, create_box, create_atoms, mass,
group, pair_style, pair_coeff, neighbor, neigh_modify, velocity, timestep, fix,
unfix, run, thermo, thermo_style, displace_atoms. Use static arithmetic equal
variables and nvt/nve fixes. Keep model declarations before dynamics begins.
NVT at 300 K and NVE may use one all-atom fix or disjoint nonempty groups that
cover every atom exactly once. Resource limits are 100000 atoms and a box bound
of 2*product(ceil(box_length/lattice_spacing)+4) <= 5000000. Before each atom
creation, existing atoms plus the same bound on the box/creation-region
intersection must not exceed 100000. Box coordinates divided by lattice spacing
must have magnitude <= 100000000. Large unused regions are permitted.
Includes, loops, immediate expressions, dumps and other commands cannot be
assessed in this pilot. Do not add them to the final submission. Do not access
hidden verifier code or another trial's artifacts. Write only within /app.
"""


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def trial_order(seed):
    rng = random.Random(seed)
    order = []
    for block in range(1, 4):
        conditions = ["A", "C"]
        rng.shuffle(conditions)
        order.extend({"id": f"{condition}{block}", "condition": condition,
                      "block": block} for condition in conditions)
    return order


def prepare(args):
    if args.output.exists():
        raise ValueError("Use a fresh campaign directory")
    runtime = json.loads((args.container / "images.json").read_text())
    images = {}
    for condition, tag in (("A", args.agent_a), ("C", args.agent_c),
                           ("verifier", runtime["local_image_tags"]["verifier"])):
        image_id = subprocess.check_output(
            ["docker", "image", "inspect", tag, "--format", "{{.Id}}"], text=True).strip()
        stable_tag = f"lamlet-p3-campaign-{condition.lower()}:" + image_id.removeprefix("sha256:")
        subprocess.run(["docker", "tag", image_id, stable_tag], check=True)
        images[condition] = {"tag": stable_tag, "id": image_id}
    if images["verifier"]["id"] != runtime["images"]["verifier"]:
        raise ValueError("Verifier image differs from the admitted image")
    args.output.mkdir(parents=True)
    for condition in ("A", "C"):
        folder = args.output / "tasks" / condition
        harbor_tasks.prepare(SimpleNamespace(
            cache=ROOT / "target/evaluation/upstream", output=folder, task="P3",
            condition=condition, agent_image=None, verifier_image=None,
            agent_seconds=600, verifier_seconds=600))
        task = folder / "nb-impact"
        instruction = task / "instruction.md"
        instruction.write_text(instruction.read_text() + "\n" + COMMON)
        for directory, image in (("environment", condition), ("tests", "verifier")):
            file = task / directory / "Dockerfile"
            text = file.read_text()
            file.write_text(text.replace(harbor_tasks.base(None), "FROM " + images[image]["tag"] + "\n"))
        # Match the archived final instructions, rather than the unextended template.
        preparation = folder / "preparation.json"
        data = json.loads(preparation.read_text())
        data["tasks"][0]["instruction_sha256"] = digest(instruction)
        write_json(preparation, data)
    manifest = {
        "schema_version": 1, "task": "P3", "split": "development",
        "model": args.model, "reasoning_effort": args.effort,
        "service_tier": "fast", "codex_version": "0.166.0", "harbor_version": "0.25.0",
        "agent_seconds": 600, "cpus": 2, "memory_mb": 4096,
        "seed": args.seed, "trials": trial_order(args.seed), "images": images,
        "oracle_sha256": digest(ROOT / "tools/p3_oracle.py"),
        "instructions": {c: digest(args.output / "tasks" / c / "nb-impact/instruction.md")
                         for c in ("A", "C")},
        "network": "public agent network; no-network separate verifier",
        "stopping_rule": "agent completion or 600-second execution timeout",
        "model_snapshot": "provider model name; backend snapshot not exposed",
        "exclusions": "none; infrastructure failures remain in the assigned denominator",
        "full_equilibration_executed": False,
        "unmeasured": ["Pass@1", "repair iterations", "CPU time", "dollar cost"],
    }
    write_json(args.output / "manifest.json", manifest)
    (args.output / "state.md").write_text(
        "# P3 A/C pilot\n\nNext action: run tools/p3_campaign.py run.\n\n"
        "1. DONE: frozen six-trial manifest and separate-verifier tasks.\n"
        "2. TODO: serial trials. Check results.json for completed trials.\n")


def elapsed(interval):
    if not interval or not interval.get("finished_at"):
        return None
    return (datetime.fromisoformat(interval["finished_at"].replace("Z", "+00:00"))
            - datetime.fromisoformat(interval["started_at"].replace("Z", "+00:00"))).total_seconds()


def assess(job, trial):
    candidates = [p for p in job.glob("*/result.json") if p.parent.name.startswith("nb-impact__")]
    if len(candidates) != 1:
        raise ValueError(f"Missing or ambiguous trial result in {job}")
    path = candidates[0]
    data = json.loads(path.read_text())
    assessment_path = path.parent / "verifier/assessment.json"
    assessment = json.loads(assessment_path.read_text()) if assessment_path.exists() else {}
    status = assessment.get("scientific_status", "unassessed")
    reward = (data.get("verifier_result") or {}).get("rewards", {}).get("reward")
    success = (status == "correct" and reward == 1 and data.get("exception_info") is None)
    return {**trial, "scientific_status": status, "success": success,
            "reward": reward, "exception": data.get("exception_info"),
            "agent_seconds": elapsed(data.get("agent_execution")),
            "verifier_seconds": elapsed(data.get("verifier")),
            "usage": data.get("agent_result"), "assessment": assessment,
            "trial_result": str(path.relative_to(job.parent.parent))}


def run(args):
    manifest = json.loads((args.output / "manifest.json").read_text())
    if digest(ROOT / "tools/p3_oracle.py") != manifest["oracle_sha256"]:
        raise ValueError("Frozen oracle changed")
    for condition, expected in manifest["instructions"].items():
        if digest(args.output / "tasks" / condition / "nb-impact/instruction.md") != expected:
            raise ValueError("Frozen instructions changed")
    for image in manifest["images"].values():
        actual = subprocess.check_output(["docker", "image", "inspect", image["tag"],
                                          "--format", "{{.Id}}"], text=True).strip()
        if actual != image["id"]:
            raise ValueError("Frozen runtime image changed")
    auth = Path(os.environ.get("CODEX_AUTH_JSON_PATH", str(Path.home() / ".codex/auth.json")))
    if not auth.is_file():
        raise ValueError("Current Codex auth.json is unavailable")
    environment = {**os.environ, "CODEX_AUTH_JSON_PATH": str(auth)}
    records = []
    for trial in manifest["trials"]:
        job = args.output / "jobs" / trial["id"]
        if not (job / "result.json").exists():
            if job.exists():
                raise ValueError(f"Incomplete job {job}; inspect it before resuming")
            config = {
                "job_name": trial["id"], "jobs_dir": str(args.output / "jobs"),
                "n_attempts": 1, "n_concurrent_trials": 1,
                "tasks": [{"path": str(args.output / "tasks" / trial["condition"] / "nb-impact")}],
                "agents": [{"name": "codex", "model_name": "openai/" + manifest["model"],
                            "kwargs": {"version": manifest["codex_version"],
                                       "reasoning_effort": manifest["reasoning_effort"],
                                       "web_search": "disabled", "reasoning_summary": "none",
                                       "config": {"service_tier": manifest["service_tier"]}}}],
                "environment": {"type": "docker", "delete": True},
            }
            config_path = args.output / (trial["id"] + ".json")
            write_json(config_path, config)
            print("Starting " + trial["id"], flush=True)
            (args.output / "state.md").write_text(
                f"# P3 A/C pilot\n\nNext action: wait for {trial['id']}.\n\n"
                f"RUNNING: {trial['id']}. Check jobs/{trial['id']}/job.log.\n"
                f"DONE: {len(records)} trials; results.json contains their grades.\n")
            with (args.output / (trial["id"] + ".log")).open("w") as log:
                subprocess.run(["uv", "run", "--cache-dir", str(ROOT / "target/uv-cache"),
                                "--python", "3.12", "--with", "harbor==0.25.0", "harbor", "run",
                                "--config", str(config_path)], env=environment,
                               stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1800)
        record = assess(job, trial)
        records.append(record)
        write_json(args.output / "results.json", {"manifest": manifest, "trials": records,
                                                  "complete": len(records) == 6})
        print(json.dumps({k: record[k] for k in ("id", "success", "scientific_status", "agent_seconds")}), flush=True)
    (args.output / "state.md").write_text(
        "# P3 A/C pilot\n\nNext action: review results.json and archive public evidence.\n\n"
        "DONE: all six assigned trials have independent assessments or recorded infrastructure outcomes.\n")


def static(args):
    admission = json.loads((args.admission / "report.json").read_text())
    records = []
    for case in admission["cases"]:
        path = (args.admission / case["case"] / "solution.in").resolve()
        if digest(path) != case["submission_sha256"]:
            raise ValueError("Admission input changed")
        records.append({"case": case["case"], "oracle_status": case["observed"],
                        "submission_sha256": digest(path),
                        "lamlet": findings(args.binary.resolve(), path)})
    write_json(args.output, {"binary_sha256": digest(args.binary),
                             "oracle_sha256": admission["oracle_sha256"], "cases": records})


def publish(args):
    results = json.loads((args.output / "results.json").read_text())
    if not results["complete"] or len(results["trials"]) != 6:
        raise ValueError("Only complete six-trial pilots can be published")
    destination = args.public
    if destination.exists():
        raise ValueError("Use a fresh public evidence directory")
    destination.mkdir(parents=True)
    summary = {"manifest": results["manifest"], "conditions": {}, "trials": []}
    for trial in results["trials"]:
        folder = args.output / "jobs" / trial["id"]
        source = next(p for p in folder.iterdir() if p.name.startswith("nb-impact__"))
        public = destination / trial["id"]
        public.mkdir()
        assessment = public / "assessment.json"
        write_json(assessment, trial["assessment"])
        artifact = source / "artifacts/app/submission/solution.in"
        if artifact.is_file():
            (public / "solution.in").write_bytes(artifact.read_bytes())
        trajectory_path = source / "agent/trajectory.json"
        tools = []
        if trajectory_path.is_file():
            trajectory = json.loads(trajectory_path.read_text())
            tools = [{"step": s["step_id"], "tool_calls": s["tool_calls"],
                      "observation": s.get("observation")}
                     for s in trajectory["steps"] if s.get("tool_calls")]
        # Publish tool evidence, not model reasoning, session metadata, or credentials.
        write_json(public / "tools.json", tools)
        tool_text = json.dumps(tools)
        record = {key: trial[key] for key in ("id", "condition", "block", "scientific_status",
                                               "success", "reward", "agent_seconds", "verifier_seconds")}
        exception = trial["exception"]
        record["exception"] = ({key: exception[key] for key in ("exception_type", "exception_message")}
                               if exception else None)
        codex_log = source / "agent/codex.txt"
        provider_errors = []
        if codex_log.is_file():
            for line in codex_log.read_text().splitlines():
                try:
                    event = json.loads(line)
                except ValueError:
                    continue
                if event.get("type") == "error":
                    provider_errors.append(event.get("message"))
        record["provider_errors"] = provider_errors
        if provider_errors and exception:
            record["exception"]["exception_message"] = "Codex execution failed; see provider_errors"
        record["tokens"] = {key: (trial.get("usage") or {}).get(key)
                            for key in ("n_input_tokens", "n_cache_tokens", "n_output_tokens")}
        record["tool_calls"] = sum(len(s["tool_calls"]) for s in tools)
        record["lamlet_check_invoked"] = bool(re.search(r"\blamlet\s+check\b", tool_text))
        summary["trials"].append(record)
    for condition in ("A", "C"):
        selected = [t for t in summary["trials"] if t["condition"] == condition]
        counts = Counter(t["scientific_status"] for t in selected)
        successes = sum(t["success"] for t in selected)
        summary["conditions"][condition] = {
            "assigned": len(selected), "correct": counts["correct"],
            "incorrect": counts["incorrect"], "unassessed": counts["unassessed"],
            "confirmed_success_fraction": successes / len(selected),
            "success_bounds": [successes / len(selected),
                               (successes + counts["unassessed"]) / len(selected)],
        }
        instruction = args.output / "tasks" / condition / "nb-impact/instruction.md"
        (destination / (condition + "-instruction.md")).write_bytes(instruction.read_bytes())
    write_json(destination / "pilot.json", summary)
    replay = json.loads(args.replay.read_text())
    # Keep every diagnostic while making source paths independent of the local checkout.
    content = json.dumps(replay, sort_keys=True).replace(str(ROOT), "$REPO").encode()
    (destination / "replay.json.gz").write_bytes(gzip.compress(content, mtime=0))
    write_json(destination / "replay-summary.json", {k: v for k, v in replay.items() if k != "cases"})
    write_json(destination / "native-admission.json", json.loads((args.admission / "report.json").read_text()))
    write_json(destination / "harbor-admission.json", json.loads(args.harbor_admission.read_text()))
    static_report = json.loads(args.static_report.read_text())
    write_json(destination / "static-mutations.json", static_report)
    for path in destination.rglob("*"):
        if path.suffix in (".json", ".md"):
            path.write_text(path.read_text().replace(str(ROOT), "$REPO"))
    write_json(destination / "hashes.json", {
        str(p.relative_to(destination)): digest(p) for p in sorted(destination.rglob("*")) if p.is_file()})
    print(json.dumps(summary["conditions"], indent=2), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("prepare", "run", "static", "publish"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--container", type=Path, default=ROOT / "target/p3-container/v4")
    parser.add_argument("--agent-a", default="lamlet-p3-campaign-a:v1")
    parser.add_argument("--agent-c", default="lamlet-p3-campaign-c:v1")
    parser.add_argument("--model", default="gpt-6.1-sol")
    parser.add_argument("--effort", default="xhigh")
    parser.add_argument("--seed", type=int, default=496)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lamlet")
    parser.add_argument("--admission", type=Path, default=ROOT / "target/p3-admission/evaluation-20261108")
    parser.add_argument("--replay", type=Path, default=ROOT / "target/evaluation/replay-20261108.json")
    parser.add_argument("--harbor-admission", type=Path, default=ROOT / "target/p3-harbor/evaluation-20261108/report.json")
    parser.add_argument("--static-report", type=Path, default=ROOT / "target/evaluation/p3-static-comparison-20261108.json")
    parser.add_argument("--public", type=Path, default=ROOT / "docs/assets/evaluation/2026-11-08")
    args = parser.parse_args()
    args.output = args.output.resolve()
    {"prepare": prepare, "run": run, "static": static, "publish": publish}[args.command](args)
