#!/usr/bin/env python3
"""Prepare the fixed lint corpus for independent review without running Lamlet."""

import argparse
from collections import Counter
import difflib
import hashlib
import json
from pathlib import Path
import sys

from corpus import write_json
from lint_evaluation import CORPUS, load_cases


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(manifest, output):
    specification, cases = load_cases(manifest)
    if output.exists():
        raise ValueError("Use a new review directory")
    output.mkdir(parents=True)
    template = {"schema_version": 1, "corpus_sha256": digest(manifest),
                "reviewer": {"name": "", "affiliation": "", "date": "",
                             "independent_of_implementation_and_labels": False},
                "cases": []}
    text = ["# Independent lint-label review", "",
            "Review the stated condition, not whether a fragment is a complete simulation.",
            "Distinguish documented advisories from errors. Record a judgement before opening",
            "the author's proposed label. No observed Lamlet output is included.", "",
            f"Corpus SHA-256: `{template['corpus_sha256']}`.", "",
            "Return a completed copy of `review.json`. Use `agree`, `disagree`, or `unsure`,",
            "with a reason for every decision. Pending entries do not count as review.", ""]
    for case in cases:
        identity = case["id"]
        rule = specification["rules"][case["rule_id"]]
        data = (manifest.parent / "inputs" / (identity + ".in")).read_text()
        template["cases"].append({"id": identity, "input_sha256": case["sha256"],
                                  "decision": "pending", "reason": ""})
        text.extend([f"## {identity}", "", f"Rule: `{case['rule_id']}` ({rule['kind']}).", "",
                     rule["applicability"], "", "```lammps", data.rstrip(), "```", "",
                     f"Input SHA-256: `{case['sha256']}`.", "", "References:", ""])
        text.extend(f"- [{source['path']}]({source['source_url']}) "
                    f"(SHA-256 `{source['sha256']}`)" for source in rule["sources"])
        if case["mutation"]:
            base = case["mutation"]["base"]
            original = (manifest.parent / "inputs" / (base + ".in")).read_text()
            diff = "".join(difflib.unified_diff(original.splitlines(True), data.splitlines(True),
                                              fromfile=base, tofile=identity))
            text.extend(["", f"Single edit relative to `{base}`:", "", "```diff", diff.rstrip(), "```"])
        text.extend(["", "<details>", "<summary>Author's proposed label and rationale</summary>", "",
                     f"Label: `{case['label']}`. Expected target outcomes: "
                     f"`{json.dumps(case['expected']['outcomes'])}`.", "", case["rationale"], "",
                     "</details>", ""])
    (output / "packet.md").write_text("\n".join(text), encoding="utf-8")
    write_json(output / "review.json", template)


def validate(manifest, review):
    _, cases = load_cases(manifest)
    if review.get("schema_version") != 1 or review.get("corpus_sha256") != digest(manifest):
        raise ValueError("Review does not identify the current corpus")
    entries = review.get("cases", [])
    if (len(entries) != len(cases) or len({entry["id"] for entry in entries}) != len(entries)
            or {entry["id"] for entry in entries} != {case["id"] for case in cases}):
        raise ValueError("Review must contain each corpus case exactly once")
    source = {case["id"]: case for case in cases}
    counts = Counter()
    for entry in entries:
        if entry.get("input_sha256") != source[entry["id"]]["sha256"]:
            raise ValueError(f"Stale reviewed input: {entry['id']}")
        decision = entry.get("decision")
        if decision not in ("pending", "agree", "disagree", "unsure"):
            raise ValueError(f"Unknown review decision: {entry['id']}")
        if decision != "pending" and not entry.get("reason", "").strip():
            raise ValueError(f"Decision requires a reason: {entry['id']}")
        counts[decision] += 1
    reviewer = review.get("reviewer", {})
    independent = (reviewer.get("independent_of_implementation_and_labels") is True
                   and all(isinstance(reviewer.get(key), str) and reviewer[key].strip()
                           for key in ("name", "affiliation", "date")))
    return {"cases": len(cases), "decisions": dict(sorted(counts.items())),
            "independence_attested": bool(independent),
            "review_complete": bool(independent and not counts["pending"]),
            "all_labels_agreed": bool(independent and counts["agree"] == len(cases)),
            "identity_verified_by_tool": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("prepare", "validate"))
    parser.add_argument("--manifest", type=Path, default=CORPUS)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--review", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            if args.output is None:
                parser.error("prepare requires --output")
            prepare(args.manifest, args.output)
            return 0
        if args.review is None:
            parser.error("validate requires --review")
        report = validate(args.manifest, json.loads(args.review.read_text()))
        print(json.dumps(report, indent=2, sort_keys=True))
        return 0 if report["all_labels_agreed"] else 1
    except (OSError, ValueError, KeyError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
