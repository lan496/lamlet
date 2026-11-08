#!/usr/bin/env python3
"""Measure reviewed lint conditions through the CLI without running simulations."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

from corpus import findings, write_json


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "tests/lint/corpus.json"
MANUAL = ROOT / "crates/lamlet/data/lammps_docs.json"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def load_cases(manifest, manual=MANUAL):
    """Verify the whole corpus before selecting a timing subset."""
    spec = json.loads(manifest.read_bytes())
    index = json.loads(manual.read_bytes())
    if spec["schema_version"] != 1 or spec["manual_revision"] != index["revision"]:
        raise ValueError("Stale corpus schema or manual revision")
    pages = {page["path"]: page for page in index["pages"]}
    for group in spec["rules"].values():
        for reference in group["sources"]:
            page = pages.get(reference["path"])
            if page is None or any(reference[key] != page[key]
                                   for key in ("sha256", "source_url")):
                raise ValueError(f"Stale manual source: {reference['path']}")
    cases = spec["cases"]
    by_id = {case["id"]: case for case in cases}
    if (not cases or not spec["rules"] or len(by_id) != len(cases)
            or len({(case["rule_id"], case["sha256"]) for case in cases}) != len(cases)
            or len(cases) != spec["case_count"]):
        raise ValueError("Duplicate IDs, duplicate rule inputs, or incomplete corpus")
    paths = set()
    payloads = {}
    for case in cases:
        identity = case["id"]
        if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", identity):
            raise ValueError(f"Invalid case ID: {identity}")
        path = manifest.parent / "inputs" / (identity + ".in")
        paths.add(path)
        data = path.read_bytes()
        if sha256(data) != case["sha256"]:
            raise ValueError(f"Modified input: {identity}")
        payloads[identity] = data
        if case["rule_id"] not in spec["rules"] or not case["rationale"].strip():
            raise ValueError(f"Missing rule or rationale: {identity}")
        expected = case["expected"]
        label = case["label"]
        allowed = {"positive": ["violated"], "negative": ["satisfied"],
                   "unknown": ["unknown"], "out_of_scope": []}
        if label not in allowed or (expected["outcomes"] != allowed[label]
                                    and not (label == "negative" and expected["outcomes"] == [])):
            raise ValueError(f"Invalid independent label: {identity}")
        warnings = [case["rule_id"]] if label == "positive" else []
        if expected["warnings"] != warnings:
            raise ValueError(f"Invalid warning expectation: {identity}")
    if paths != set((manifest.parent / "inputs").glob("*.in")):
        raise ValueError("Unlisted corpus inputs")
    for case in cases:
        identity = case["id"]
        mutation = case["mutation"]
        if case["label"] == "positive":
            base = by_id.get(mutation["base"]) if mutation else None
            if (base is None or base["label"] != "negative"
                    or base["rule_id"] != case["rule_id"]):
                raise ValueError(f"Missing negative mutation base: {identity}")
            original = payloads[base["id"]]
            before, after = (mutation[key].encode("utf-8") for key in ("before", "after"))
            if (not before or before == after or original.count(before) != 1
                    or original.replace(before, after, 1) != payloads[identity]):
                raise ValueError(f"Stale single-edit mutation: {identity}")
    for rule_id in spec["rules"]:
        labels = Counter(case["label"] for case in cases if case["rule_id"] == rule_id)
        if any(labels[label] < 2 for label in ("positive", "negative", "unknown")):
            raise ValueError(f"Incomplete controls for {rule_id}")
    return spec, sorted(cases, key=lambda case: case["id"])


def observe(report, rule_id, path):
    canonical = json.dumps(report, sort_keys=True, separators=(",", ":"))
    canonical = canonical.replace(json.dumps(str(path)), json.dumps("inputs/" + path.name))
    return {
        "diagnostics_sha256": sha256(canonical.encode("utf-8")),
        "outcomes": sorted(finding["outcome"] for finding in report.get("physical_findings", [])
                           if finding["rule_id"] == rule_id),
        "warnings": sorted(d["code"] for d in report["diagnostics"]
                           if d["severity"] == "warning"),
        "errors": sorted(d["code"] for d in report["diagnostics"]
                         if d["severity"] == "error"),
    }


def evaluate_case(binary, directory, case, output):
    path = directory / "inputs" / (case["id"] + ".in")
    standard = findings(binary, path)
    strict = findings(binary, path, strict=True)
    output.unlink(missing_ok=True)
    built = findings(binary, path, output=output)
    observed = observe(standard, case["rule_id"], path)
    legacy = {
        "normal_accepted": not standard["summary"]["errors"],
        "strict_accepted": not strict["summary"]["errors"],
        "build_accepted": not built["summary"]["errors"],
        "byte_identical": output.is_file() and output.read_bytes() == path.read_bytes(),
        "diagnostics_agree": standard == strict == built,
    }
    matched = (observed["outcomes"] == case["expected"]["outcomes"]
               and observed["warnings"] == case["expected"]["warnings"]
               and not observed["errors"])
    return {"id": case["id"], "rule_id": case["rule_id"], "label": case["label"],
            "expected": case["expected"], "observed": observed, "legacy": legacy,
            "passed": matched and all(legacy.values())}


def ratio(numerator, denominator):
    return {"numerator": numerator, "denominator": denominator,
            "rate": numerator / denominator if denominator else None}


def summarize(cases):
    positives = [case for case in cases if case["label"] == "positive"]
    negatives = [case for case in cases if case["label"] == "negative"]
    unknowns = [case for case in cases if case["label"] == "unknown"]
    warned = lambda case: case["rule_id"] in case["observed"]["warnings"]
    return {
        "cases": len(cases),
        "passed": sum(case["passed"] for case in cases),
        "detection": ratio(sum(warned(case) for case in positives), len(positives)),
        "false_warning": ratio(sum(bool(case["observed"]["warnings"]) for case in negatives),
                               len(negatives)),
        "unknown": ratio(sum("unknown" in case["observed"]["outcomes"] for case in cases),
                         len(cases)),
        "unknown_recognition": ratio(sum(case["observed"]["outcomes"] == ["unknown"]
                                         for case in unknowns), len(unknowns)),
        "unknown_warned": sum(bool(case["observed"]["warnings"]) for case in unknowns),
        "not_evaluated": sum(not case["observed"]["outcomes"] for case in cases),
        "legacy": {key: sum(case["legacy"][key] for case in cases)
                   for key in ("normal_accepted", "strict_accepted", "build_accepted",
                               "byte_identical", "diagnostics_agree")},
    }


def evaluate(binary, manifest, selected=None):
    spec, cases = load_cases(manifest)
    if selected is not None:
        cases = [case for case in cases if case["id"] == selected]
        if not cases:
            raise ValueError(f"Unknown case ID: {selected}")
    version = subprocess.run([str(binary), "--version"], capture_output=True,
                             check=True, text=True, timeout=30).stdout.strip()
    with tempfile.TemporaryDirectory(prefix="lamlet-lint-") as temporary:
        results = [evaluate_case(binary, manifest.parent, case, Path(temporary) / "output.in")
                   for case in cases]
    return {
        "schema_version": 1,
        "experiment": "lint_condition_mutations",
        "corpus_sha256": sha256(manifest.read_bytes()),
        "manual_revision": spec["manual_revision"],
        "binary": {"version": version, "sha256": sha256(binary.read_bytes())},
        "selection": {"case": selected, "complete": selected is None},
        "summary": summarize(results),
        "by_rule": {rule_id: summarize([case for case in results if case["rule_id"] == rule_id])
                    for rule_id in sorted({case["rule_id"] for case in results})},
        "cases": results,
        "lammps_launches": 0,
        "agent_trials": 0,
        "scientific_correctness": None,
    }


def matches_baseline(report, baseline):
    # Executable bytes vary across platforms. All observations and denominators must agree.
    return ({key: value for key, value in report.items() if key != "binary"}
            == {key: value for key, value in baseline.items() if key != "binary"})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lamlet")
    parser.add_argument("--manifest", type=Path, default=CORPUS)
    parser.add_argument("--report", type=Path, default=ROOT / "target/lint-evaluation/report.json")
    parser.add_argument("--baseline", type=Path, help="Fail if archived observations are stale")
    parser.add_argument("--case", help="Time one semantic case after verifying the full corpus")
    args = parser.parse_args()
    try:
        report = evaluate(args.binary.resolve(strict=True), args.manifest.resolve(), args.case)
        write_json(args.report, report)
        passed = report["summary"]["passed"] == report["summary"]["cases"]
        if args.baseline:
            passed &= matches_baseline(report, json.loads(args.baseline.read_bytes()))
        print(json.dumps({"summary": report["summary"], "passed": passed}, sort_keys=True))
        return 0 if passed else 1
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(json.dumps({"kind": "infrastructure_failure", "error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
