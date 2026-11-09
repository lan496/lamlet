#!/usr/bin/env python3
"""Verify source-pinned development screening without adjudicating defect labels."""

import argparse
from collections import Counter
import json
from pathlib import Path
import sys

import defect_discovery as discovery


ROOT = discovery.ROOT
PLAN = ROOT / "research/defects/screening-plan.json"
MANIFEST = ROOT / "research/defects/screened-cohort.json"
DECISIONS = {"replay_candidate", "needs_evidence", "excluded"}
CLASSIFICATIONS = {"input_report", "engine_report", "build_environment",
                   "usage_question", "announcement", "feature_request"}
PROVENANCE = {"attached_scripts", "complete_inline_input", "command_fragment",
              "not_captured", "not_applicable"}


def text(value):
    return isinstance(value, str) and bool(value.strip())


def validate(manifest=MANIFEST, plan_path=PLAN, cache=None):
    plan = json.loads(plan_path.read_bytes())
    archive = ROOT / plan["discovery_archive"]
    if (plan["schema_version"] != 1 or not archive.resolve().is_relative_to(ROOT.resolve())
            or set(plan["decisions"]) != DECISIONS
            or set(plan["classification"]) != CLASSIFICATIONS
            or discovery.source.sha256((archive / "inventory.json").read_bytes())
            != plan["discovery_inventory_sha256"]):
        raise ValueError("Screening plan or discovery inventory differs")
    discovery.verify(archive, cache)
    report = json.loads((archive / "report.json").read_bytes())
    selected = {c["id"]: c for c in report["candidates"] if c["selection"] == "selected"}
    sources = {s["id"]: s for s in json.loads((archive / "sources.json").read_bytes())["sources"]}
    data = json.loads(manifest.read_bytes())
    review = {"kind": "model_assisted", "agent": "Codex", "independent": False}
    if (data["schema_version"] != 1 or data["cohort"] != plan["cohort"]
            or report["cohort"] != plan["cohort"]
            or data["plan_sha256"] != discovery.source.sha256(plan_path.read_bytes())
            or data["review"] != review or data["review"]["independent"] is not False
            or data["split"] != "development"
            or data["checker_evaluated"] is not False
            or any(type(data[k]) is not int or data[k] != 0 for k in
                   ("independently_adjudicated", "gold_defect_pairs", "native_replays", "agent_trials"))):
        raise ValueError("Screening provenance or review status differs")
    cases = data["cases"]
    if len(cases) != len(selected) or {c["id"] for c in cases} != set(selected):
        raise ValueError("Every selected topic needs exactly one screening decision")
    for case in cases:
        identity = case["id"]
        source = sources.get(identity)
        if source is None:
            raise ValueError("Complete source evidence is required for screening")
        posts = {p["post_number"]: p for p in source["posts"]}
        numbers = case["read_post_numbers"]
        if (case["url"] != selected[identity]["url"]
                or not all(type(n) is int for n in numbers)
                or numbers != sorted(posts)):
            raise ValueError("Source identity or complete post coverage differs")
        if (case["classification"] not in CLASSIFICATIONS or case["decision"] not in DECISIONS
                or case["input_provenance"] not in PROVENANCE
                or not all(text(case[k]) for k in ("reason", "next_action"))
                or not isinstance(case["dependency_gaps"], list)
                or not all(text(gap) for gap in case["dependency_gaps"])
                or any(case[k] is not None and not text(case[k])
                       for k in ("intent", "condition", "proposed_correction"))):
            raise ValueError("Invalid screening classification, fields, or rationale")
        input_report = case["classification"] == "input_report"
        if ((case["decision"] != "excluded") != input_report
                or input_report and not all(text(case[k]) for k in ("intent", "condition"))
                or case["decision"] == "needs_evidence" and not case["dependency_gaps"]):
            raise ValueError("Screening decision does not match its evidence gaps or classification")
        if case["decision"] == "replay_candidate" and (
                case["dependency_gaps"] or not text(case["proposed_correction"])
                or case["input_provenance"] not in {"attached_scripts", "complete_inline_input"}):
            raise ValueError("Replay preparation requires input, correction, and dependencies")
        evidence = case["evidence"]
        if not evidence or max(posts) not in {e["post_number"] for e in evidence}:
            raise ValueError("Evidence must include the final captured reply")
        parsed_posts = {}
        if cache:
            complete = json.loads((cache / "threads" / identity / "complete.json").read_bytes())
            parsed_posts = {p["post_number"]: "".join(discovery.source.Post(p["cooked"]).text)
                            for p in complete["posts"]}
        for item in evidence:
            if (type(item["post_number"]) is not int or item["post_number"] not in posts
                    or not text(item["anchor"]) or len(item["anchor"].split()) > 12):
                raise ValueError("Invalid source post or short evidence anchor")
            if cache and item["anchor"] not in parsed_posts[item["post_number"]]:
                raise ValueError("Evidence anchor is absent from the captured post")
        expected = {a["file"]: a for a in source["attachments"]}
        artifacts = case["artifacts"]
        if len(artifacts) != len(expected) or {a["file"] for a in artifacts} != set(expected):
            raise ValueError("Every captured attachment needs one inspection record")
        for item in artifacts:
            receipt = expected[item["file"]]
            if (item["status"] != receipt["status"]
                    or item["sha256"] != receipt.get("sha256")
                    or item["inspection"] not in {"receipt_only", "text_read"}
                    or not text(item["role"]) or not all(text(a) for a in item["anchors"])
                    or any(len(a.split()) > 12 for a in item["anchors"])
                    or item["inspection"] == "receipt_only" and item["anchors"]
                    or item["inspection"] == "text_read" and (
                        item["status"] != "downloaded" or not item["anchors"])):
                raise ValueError("Invalid attachment inspection or source receipt")
            if cache and item["anchors"]:
                body = (cache / "threads" / identity / item["file"]).read_text(encoding="utf-8")
                if any(anchor not in body for anchor in item["anchors"]):
                    raise ValueError("Evidence anchor is absent from the captured attachment")
        if case["input_provenance"] == "attached_scripts" and not any(
                a["inspection"] == "text_read" for a in artifacts):
            raise ValueError("Attached input provenance requires a text inspection record")
        if case["input_provenance"] in {"command_fragment", "complete_inline_input"} and not any(
                p["code_sha256"] for p in posts.values()):
            raise ValueError("Inline input provenance requires captured code")
    return {"schema_version": 1, "cohort": plan["cohort"], "split": "development",
            "plan_sha256": data["plan_sha256"],
            "manifest_sha256": discovery.source.sha256(manifest.read_bytes()),
            "discovery_inventory_sha256": plan["discovery_inventory_sha256"],
            "review": review, "checker_evaluated": False, "native_replays": 0, "agent_trials": 0,
            "independently_adjudicated": 0, "gold_defect_pairs": 0,
            "summary": {"selected_topics": len(selected), "screened_topics": len(cases),
                "covered_posts": sum(len(c["read_post_numbers"]) for c in cases),
                "decisions": {name: sum(c["decision"] == name for c in cases) for name in sorted(DECISIONS)},
                "classifications": dict(sorted(Counter(c["classification"] for c in cases).items()))}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("report", "verify"))
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--plan", type=Path, default=PLAN)
    parser.add_argument("--cache", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if args.command == "verify" and args.report is None:
        parser.error("verify requires --report")
    try:
        report = validate(args.manifest, args.plan, args.cache)
        if args.command == "verify":
            if report != json.loads(args.report.read_bytes()):
                raise ValueError("Published screening report differs")
            report = {"screened_topics": report["summary"]["screened_topics"],
                      "cached_anchors_verified": args.cache is not None,
                      "independently_adjudicated": 0, "gold_defect_pairs": 0}
        print(json.dumps(report, indent=2, sort_keys=True))
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
