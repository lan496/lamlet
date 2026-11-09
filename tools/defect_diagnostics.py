#!/usr/bin/env python3
"""Audit existing checker diagnostics against frozen natural-report evidence."""

import argparse
from collections import Counter
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

from corpus import findings, write_json
import defect_replay as replay
from defect_corpus import MANIFEST, sha256

ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "research/defects/diagnostic-audit.json"
MODES = ("check", "strict", "build")


def digest(path):
    return sha256(path.read_bytes())


def context(plan_path=PLAN, cache=None):
    if digest(plan_path) != digest(PLAN):
        raise ValueError("Changed frozen diagnostic audit plan")
    plan = json.loads(plan_path.read_bytes())
    if plan["schema_version"] != 1 or plan["split"] != "development":
        raise ValueError("Unsupported diagnostic audit plan")
    archive = ROOT / plan["native_archive"]
    for path, key in ((archive / "inventory.json", "native_inventory_sha256"),
                      (archive / "report.json", "native_report_sha256"),
                      (MANIFEST, "source_manifest_sha256"),
                      (replay.FIXTURES / "source-lock.json", "source_lock_sha256")):
        if digest(path) != plan[key]:
            raise ValueError(f"Changed frozen evidence: {path.name}")
    replay.verify(archive, cache)
    native = json.loads((archive / "report.json").read_bytes())
    config = json.loads((archive / "replays.json").read_bytes())
    ids = [case["id"] for case in plan["cases"]]
    if len(set(ids)) != len(ids) or set(ids) != {case["id"] for case in native["cases"]}:
        raise ValueError("Every native replay needs exactly one audit entry")
    for case in plan["cases"]:
        if set(case["expected_target"]) != {"before", "after"}:
            raise ValueError("Missing target variant")
        if case["basis"] not in {"command_acceptance", "execution_only", "unassessed",
                                  "temperature_normalization", "kinetic_energy"}:
            raise ValueError("Unknown native-condition basis")
        for expected in case["expected_target"].values():
            if ((case["target_rule"] is None and expected is not None)
                    or (case["target_rule"] is not None and expected not in {"violated", "satisfied"})):
                raise ValueError("Invalid declared target outcome")
    return plan, native, config


def reference(case, variant, config, stdout):
    """Apply only the condition criteria of the existing native experiment."""
    if variant["timed_out"] or case["basis"] == "unassessed":
        return "unassessed"
    if case["basis"] == "command_acceptance":
        if variant["engine_exit_code"] == 0:
            return "satisfied"
        return "violated" if "Expected integer parameter" in stdout else "unassessed"
    if variant["engine_exit_code"] != 0:
        return "unassessed"
    if case["basis"] == "execution_only":
        return "execution_only"
    measured = variant["measurement"]
    if measured is None or variant["measurement_error"] is not None:
        return "unassessed"
    if case["basis"] == "temperature_normalization":
        if not measured["atom_count_changed"]:
            return "unassessed"
        error = measured["max_relative_error"]
    else:
        error = measured["relative_error"]
    return "satisfied" if error <= config["relative_tolerance"] else "violated"


def normalize(value, path):
    if isinstance(value, dict):
        return {key: normalize(item, path) for key, item in value.items()}
    if isinstance(value, list):
        return [normalize(item, path) for item in value]
    if isinstance(value, str):
        return value.replace(str(path), "inputs/" + path.name)
    return value


def observation(case, modes, output_hash, input_hash):
    standard = modes["check"]
    assessed = standard["status"] == "collected"
    report = standard.get("report", {})
    selected = lambda item: (item.get("rule_id", item.get("code")) == case["target_rule"]
        and (case["target_subject"] is None or any(
            evidence["subject"] == case["target_subject"] for evidence in item.get("evidence", []))))
    target = sorted(f["outcome"] for f in report.get("physical_findings", []) if selected(f))
    diagnostics = report.get("diagnostics", [])
    warned = any(d["severity"] == "warning" and selected(d) for d in diagnostics)
    accepted = {mode + "_accepted": result["status"] == "collected"
                and result["report"]["summary"]["errors"] == 0 for mode, result in modes.items()}
    complete = all(result["status"] == "collected" for result in modes.values())
    target_state = ("unmapped" if case["target_rule"] is None else "unavailable" if not assessed
                    else "absent" if not target else target[0] if len(set(target)) == 1 else "mixed")
    return {"assessed": assessed, "target_state": target_state,
            "target_outcomes": target, "target_warned": warned,
            "warnings": sorted(d["code"] for d in diagnostics if d["severity"] == "warning"),
            "unknown_rules": sorted({f["rule_id"] for f in report.get("physical_findings", [])
                                     if f["outcome"] == "unknown"}),
            "legacy": {**accepted, "byte_identical": output_hash == input_hash,
                       "diagnostics_agree": complete and modes["check"] == modes["strict"] == modes["build"]}}


def summarize(results):
    mapped = [r for r in results if r["target_rule"] is not None
              and r["reference_status"] in {"violated", "satisfied"}]
    return {"inputs": len(results), "source_threads": len({r["source"] for r in results}),
            "reference_statuses": dict(sorted(Counter(r["reference_status"] for r in results).items())),
            "mapped_violated": sum(r["reference_status"] == "violated" for r in mapped),
            "mapped_violated_warned": sum(r["reference_status"] == "violated"
                                          and r["observed"]["target_warned"] for r in mapped),
            "mapped_satisfied": sum(r["reference_status"] == "satisfied" for r in mapped),
            "mapped_satisfied_warned": sum(r["reference_status"] == "satisfied"
                                           and r["observed"]["target_warned"] for r in mapped),
            "mapped_cli_unavailable": sum(not r["observed"]["assessed"] for r in mapped),
            "mapped_target_absent": sum(r["observed"]["target_state"] == "absent" for r in mapped),
            "mapped_target_unknown": sum(r["observed"]["target_state"] == "unknown" for r in mapped),
            "declared_targets_matched": sum(r["declared_target_matched"] is True for r in results),
            "declared_targets": sum(r["target_rule"] is not None for r in results),
            "inputs_with_any_warning": sum(bool(r["observed"]["warnings"]) for r in results),
            "inputs_with_unknown": sum(bool(r["observed"]["unknown_rules"]) for r in results),
            "infrastructure_failures": sum(r["infrastructure_failures"] for r in results),
            "legacy": {key: sum(r["observed"]["legacy"][key] for r in results)
                       for key in ("check_accepted", "strict_accepted", "build_accepted",
                                   "byte_identical", "diagnostics_agree")}}


def assemble(directory, binary):
    plan, native, config = context(directory / "plan.json")
    archive = ROOT / plan["native_archive"]
    result = []
    for case in plan["cases"]:
        source = next(c for c in native["cases"] if c["id"] == case["id"])
        for variant in source["variants"]:
            identity = case["id"] + "/" + variant["variant"]
            at = directory / "diagnostics" / identity
            modes = {mode: json.loads((at / (mode + ".json")).read_bytes()) for mode in MODES}
            receipt = json.loads((at / "emission.json").read_bytes())
            observed = observation(case, modes, receipt["sha256"], variant["input_sha256"])
            expected = case["expected_target"][variant["variant"]]
            result.append({"id": identity, "source": source["source"], "kind": source["kind"],
                           "input_sha256": variant["input_sha256"], "target_rule": case["target_rule"],
                           "diagnostics_sha256": {mode: digest(at / (mode + ".json")) for mode in MODES},
                           "reference_status": reference(case, variant, config,
                               (archive / identity / "engine.stdout").read_text()),
                           "observed": observed,
                           "infrastructure_failures": sum(m["status"] != "collected" for m in modes.values()),
                           "declared_target_matched": (observed["assessed"]
                               and observed["target_outcomes"] == [expected]
                               and observed["target_warned"] == (expected == "violated"))
                               if expected is not None else None})
    return {"schema_version": 1, "experiment": "natural_report_diagnostic_audit",
            "plan_sha256": digest(directory / "plan.json"), "binary": binary,
            "summary": summarize(result), "cases": result,
            "split": "development", "independent_review": "pending", "gold_defect_pairs": 0,
            "checker_accuracy": None, "scientific_correctness": None,
            "new_lammps_launches": 0, "agent_trials": 0}


def run(binary, cache, output, checker_source_commit=None):
    plan, native, config = context(cache=cache)
    revision = checker_source_commit or plan["checker_source_commit"]
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision):
        raise ValueError("Checker source revision must be a full lowercase commit hash")
    lock = json.loads((replay.FIXTURES / "source-lock.json").read_bytes())
    inputs = {case["id"]: replay.scripts(case, cache, lock) for case in config["cases"]}
    for case in native["cases"]:
        for variant in case["variants"]:
            if sha256(inputs[case["id"]][variant["variant"]].encode()) != variant["input_sha256"]:
                raise ValueError("Source transformation differs from the frozen native input")
    output.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(PLAN, output / "plan.json")
    version = subprocess.run([str(binary), "--version"], capture_output=True, text=True,
                             check=True, timeout=30).stdout.strip()
    receipt = {"version": version, "sha256": digest(binary),
               "declared_source_commit": revision}
    write_json(output / "capture.json", {"binary": receipt, "plan_sha256": digest(PLAN),
        "source_transformations_verified": True,
        "tool_sha256": {name: digest(ROOT / "tools" / name)
                        for name in ("defect_diagnostics.py", "defect_replay.py", "defect_corpus.py", "corpus.py")}})
    first = None
    with tempfile.TemporaryDirectory(prefix="lamlet-natural-") as temporary:
        for case in plan["cases"]:
            for variant, script in inputs[case["id"]].items():
                path = Path(temporary) / (case["id"] + "-" + variant + ".in")
                path.write_bytes(script.encode())
                destination = output / "diagnostics" / case["id"] / variant
                emission = path.with_suffix(".output.in")
                started = time.perf_counter()
                for mode in MODES:
                    try:
                        report = findings(binary, path, output=emission if mode == "build" else None,
                                          strict=mode == "strict")
                        result = {"status": "collected", "report": normalize(report, path)}
                    except (OSError, ValueError, subprocess.SubprocessError) as error:
                        result = {"status": "infrastructure_failure", "error": str(error).replace(temporary, "workspace")}
                    write_json(destination / (mode + ".json"), result)
                write_json(destination / "emission.json", {"sha256": digest(emission) if emission.is_file() else None})
                if first is None:
                    first = time.perf_counter() - started
                    write_json(output / "budget.json", {"first_input_three_modes_seconds": first,
                        "remaining_inputs": 9, "estimated_remaining_seconds": 9 * first,
                        "per_command_timeout_seconds": 30, "heterogeneous_inputs": True})
                    print(f"First three-mode input: {first:.6f}s; remaining estimate: {9 * first:.6f}s", file=sys.stderr)
    report = assemble(output, receipt)
    write_json(output / "report.json", report)
    write_json(output / "inventory.json", {str(p.relative_to(output)): digest(p)
        for p in sorted(output.rglob("*")) if p.is_file() and p.name != "inventory.json"})
    return report


def verify(directory):
    inventory = json.loads((directory / "inventory.json").read_bytes())
    actual = {str(p.relative_to(directory)) for p in directory.rglob("*")
              if p.is_file() and p.name != "inventory.json"}
    if actual != set(inventory):
        raise ValueError("Diagnostic archive inventory differs from its files")
    for name, expected in inventory.items():
        path = directory / name
        if not path.resolve().is_relative_to(directory.resolve()) or digest(path) != expected:
            raise ValueError(f"Changed diagnostic archive: {name}")
    capture = json.loads((directory / "capture.json").read_bytes())
    if capture["plan_sha256"] != digest(directory / "plan.json"):
        raise ValueError("Capture refers to a different audit plan")
    actual_report = json.loads((directory / "report.json").read_bytes())
    expected_report = assemble(directory, capture["binary"])
    if actual_report != expected_report:
        raise ValueError("Diagnostic summary differs from the archived CLI observations")
    return {"verified_inputs": len(expected_report["cases"]), "verified_files": len(inventory),
            "checker_rerun": False, "source_inputs_verified_offline": False}


def prepare_review(output):
    plan, native, config = context()
    sources = {case["id"]: case for case in json.loads(MANIFEST.read_bytes())["cases"]}
    output.mkdir(parents=True, exist_ok=False)
    form = {"schema_version": 1, "plan_sha256": digest(PLAN),
            "native_report_sha256": plan["native_report_sha256"],
            "reviewer": {"name": "", "affiliation": "", "date": "", "independent": False}, "cases": []}
    text = ["# Review of bounded natural-report conditions", "",
            "Assess the condition evidence and provenance scope. No checker observations are included.",
            "Agreement concerns the bounded assessment, not a gold label or complete simulation validity.",
            "Return review.json with agree, disagree, or unsure and a reason for every case.", "",
            "The native criteria use relative tolerance 1e-8. The GCMC comparison uses",
            "T = 2 K / [k_B (3 N - 3)], with k_B = 0.0019872067 kcal/mol/K, no molecular",
            "constraints, and a fluid group containing all atoms. K and N come from native stdout.",
            "The velocity reconstruction prescribes 100 Angstrom/ps for one atom of mass 183.84.",
            "Its expected energy is m v^2 / 2 times the metal-unit conversion 0.00010364269.",
            "This external speed contract is not a reproduction of the original 20 eV collision.", "",
            "[Frozen native plan](https://github.com/lan496/lamlet/blob/main/research/defects/replays.json)",
            "and [velocity template](https://github.com/lan496/lamlet/blob/main/research/defects/velocity.in.template).", ""]
    for case in plan["cases"]:
        observed = next(c for c in native["cases"] if c["id"] == case["id"])
        source = sources[observed["source"]]
        hashes = {v["variant"]: v["input_sha256"] for v in observed["variants"]}
        form["cases"].append({"id": case["id"], "input_sha256": hashes, "decision": "pending", "reason": ""})
        text.extend(["## " + case["id"], "", case["scope"], "", "Intent: " + source["intent"], "",
                     f"Source: [{source['id']}]({source['url']}). Provenance: `{observed['kind']}`.", "",
                     "Source evidence:", ""])
        text.extend(f"- [Post {e['post']}]({source['url']}/{e['post']}): {e['role']}. {e['anchor']}"
                    for e in source["evidence"])
        native_case = next(c for c in config["cases"] if c["id"] == case["id"])
        if "replace" in native_case:
            before, after = native_case["replace"]
            text.extend(["", "Proposed change:", "", "```diff", "-" + before, "+" + after, "```"])
        elif "before" in native_case:
            text.extend(["", "Proposed change:", "", "```diff", "-" + native_case["before"],
                         "+" + native_case["after"], "```"])
        elif "remove" in native_case:
            text.extend(["", "Proposed removal:", "", "```diff", "-" + native_case["remove"], "```"])
        else:
            text.extend(["", "The reconstruction adds `units box` to the velocity command."])
        for variant in observed["variants"]:
            stdout = (ROOT / plan["native_archive"] / case["id"] / variant["variant"] / "engine.stdout").read_text()
            text.extend(["", "### " + variant["variant"], "",
                         f"Input SHA-256: `{variant['input_sha256']}`.", "",
                         "[Native stdout](https://github.com/lan496/lamlet/blob/main/"
                         f"{plan['native_archive']}/{case['id']}/{variant['variant']}/engine.stdout).", "",
                         "```json", json.dumps({"engine_exit_code": variant["engine_exit_code"],
                             "timed_out": variant["timed_out"], "measurement": variant["measurement"],
                             "proposed_assessment": reference(case, variant, config, stdout)}, indent=2), "```"])
        text.extend(["", "Dependencies and limitations:", ""])
        text.extend("- " + reason for reason in source["blockers"])
        text.append("")
    (output / "packet.md").write_text("\n".join(text), encoding="utf-8")
    form["packet_sha256"] = digest(output / "packet.md")
    write_json(output / "review.json", form)


def validate_review(path, template):
    review = json.loads(path.read_bytes())
    for key in ("schema_version", "plan_sha256", "native_report_sha256", "packet_sha256"):
        if review.get(key) != template[key]:
            raise ValueError("Review refers to different evidence")
    expected = {case["id"]: case["input_sha256"] for case in template["cases"]}
    entries = review["cases"]
    if len(entries) != len(expected) or {c["id"] for c in entries} != set(expected):
        raise ValueError("Every replay needs exactly one review decision")
    counts = Counter()
    for entry in entries:
        if entry["input_sha256"] != expected[entry["id"]]:
            raise ValueError("Stale reviewed input")
        decision = entry["decision"]
        if decision not in {"pending", "agree", "disagree", "unsure"}:
            raise ValueError("Unknown review decision")
        if decision != "pending" and not entry["reason"].strip():
            raise ValueError("Review decision requires a reason")
        counts[decision] += 1
    reviewer = review["reviewer"]
    independent = reviewer["independent"] is True and all(
        isinstance(reviewer.get(key), str) and reviewer[key].strip() for key in ("name", "affiliation", "date"))
    return {"decisions": dict(sorted(counts.items())), "independence_attested": bool(independent),
            "all_conditions_agreed": bool(independent and counts["agree"] == len(expected)),
            "identity_verified_by_tool": False, "gold_pairs_admitted_by_tool": 0}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("run", "verify", "prepare-review", "validate-review"))
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lamlet")
    parser.add_argument("--checker-source-commit", help="Declare a new checker revision; frozen conditions are unchanged")
    parser.add_argument("--cache", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--review", type=Path)
    args = parser.parse_args()
    try:
        exit_code = 0
        if args.command == "run":
            if args.cache is None or args.output is None:
                parser.error("run requires --cache and a new --output directory")
            result = run(args.binary.resolve(strict=True), args.cache, args.output,
                         args.checker_source_commit)["summary"]
            if result["infrastructure_failures"] or any(n != result["inputs"] for n in result["legacy"].values()):
                exit_code = 1
        elif args.command == "verify":
            if args.archive is None:
                parser.error("verify requires --archive")
            result = verify(args.archive)
        elif args.command == "prepare-review":
            if args.output is None:
                parser.error("prepare-review requires a new --output directory")
            prepare_review(args.output)
            result = {"review_cases": 5, "independent_review": "pending"}
        else:
            if args.review is None:
                parser.error("validate-review requires --review")
            with tempfile.TemporaryDirectory() as temporary:
                at = Path(temporary) / "review"
                prepare_review(at)
                result = validate_review(args.review, json.loads((at / "review.json").read_bytes()))
            print(json.dumps(result, sort_keys=True))
            return 0 if result["all_conditions_agreed"] else 1
        print(json.dumps(result, sort_keys=True))
        return exit_code
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(json.dumps({"kind": "infrastructure_failure", "error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
