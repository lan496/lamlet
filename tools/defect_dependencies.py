#!/usr/bin/env python3
"""Capture bounded public dependency URLs and verify recovery receipts, not defects."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import sys
import time
import urllib.error
import urllib.request

import defect_corpus as source
from corpus import write_json


ROOT = source.ROOT
PLAN = ROOT / "research/defects/dependency-plan.json"
ASSESSMENT = ROOT / "research/defects/dependency-assessment.json"
LIMIT = 16 * 1024 * 1024


def read_plan(path):
    plan = json.loads(path.read_bytes())
    screening = ROOT / "research/defects/screened-cohort.json"
    if (plan["schema_version"] != 1
            or source.sha256(screening.read_bytes()) != plan["screening_sha256"]):
        raise ValueError("Recovery plan differs from frozen screening")
    pending = {c["id"] for c in json.loads(screening.read_bytes())["cases"]
               if c["decision"] == "needs_evidence"}
    entries = plan["sources"]
    if (not entries or len({s["id"] for s in entries}) != len(entries)
            or {s["case"] for s in entries} != pending):
        raise ValueError("Every deferred case needs distinct planned source records")
    for item in entries:
        if (re.fullmatch(r"[a-z][a-z0-9-]*", item["id"]) is None
                or not item["url"].startswith("https://")
                or item["kind"] not in {"context", "reference_file", "dependency_folder"}):
            raise ValueError("Invalid planned source")
    return plan


def fetch(plan_path, output):
    plan = read_plan(plan_path)
    output.mkdir(parents=True, exist_ok=False)
    snapshot = {"schema_version": 1, "plan_sha256": source.sha256(plan_path.read_bytes()),
                "retrieved_at": datetime.now(timezone.utc).isoformat(), "sources": []}
    for item in plan["sources"]:
        started = time.monotonic()
        record = {"id": item["id"], "url": item["url"]}
        request = urllib.request.Request(item["url"], headers={"User-Agent": source.USER_AGENT})
        try:
            try:
                response = urllib.request.urlopen(request, timeout=30)
            except urllib.error.HTTPError as error:
                response = error
            with response:
                body = response.read(LIMIT + 1)
                if len(body) > LIMIT:
                    raise ValueError("Response exceeds 16 MiB")
                record.update(outcome="response", http_status=response.code,
                              resolved_url=response.url, content_type=response.headers.get_content_type(),
                              bytes=len(body), sha256=source.sha256(body))
            (output / item["id"]).write_bytes(body)
        except (OSError, ValueError) as error:
            record.update(outcome="unavailable", error=str(error))
        record["seconds"] = round(time.monotonic() - started, 6)
        snapshot["sources"].append(record)
        write_json(output / "snapshot.json", snapshot)
        print(f"{item['id']}: {record.get('http_status', record['outcome'])}", file=sys.stderr)
    return snapshot


def verify(snapshot_path, plan_path=PLAN, assessment_path=ASSESSMENT, cache=None):
    plan = read_plan(plan_path)
    snapshot = json.loads(snapshot_path.read_bytes())
    if (snapshot["schema_version"] != 1
            or snapshot["plan_sha256"] != source.sha256(plan_path.read_bytes())):
        raise ValueError("Snapshot plan differs")
    records = snapshot["sources"]
    expected = {s["id"]: s for s in plan["sources"]}
    if len(records) != len(expected) or {r["id"] for r in records} != set(expected):
        raise ValueError("Every planned URL needs exactly one outcome")
    by_id = {r["id"]: r for r in records}
    for record in records:
        if record["url"] != expected[record["id"]]["url"]:
            raise ValueError("Source URL differs")
        if record["outcome"] == "response":
            if (type(record["http_status"]) is not int or not 100 <= record["http_status"] <= 599
                    or type(record["bytes"]) is not int or not 0 <= record["bytes"] <= LIMIT
                    or re.fullmatch(r"[a-f0-9]{64}", record["sha256"]) is None
                    or not record["resolved_url"].startswith("https://")
                    or not record["content_type"]):
                raise ValueError("Invalid response receipt")
            if cache:
                body = (cache / record["id"]).read_bytes()
                if len(body) != record["bytes"] or source.sha256(body) != record["sha256"]:
                    raise ValueError("Cached response differs")
        elif record["outcome"] != "unavailable" or not record.get("error"):
            raise ValueError("Invalid unavailable outcome")
    assessment = json.loads(assessment_path.read_bytes())
    if (assessment["schema_version"] != 1 or assessment["split"] != "development"
            or assessment["review"] != "model_assisted"
            or assessment["snapshot_sha256"] != source.sha256(snapshot_path.read_bytes())
            or any(type(assessment[k]) is not int or assessment[k] != 0 for k in
                   ("native_replays", "checker_runs", "agent_trials", "gold_defect_pairs",
                    "independent_adjudications"))):
        raise ValueError("Assessment provenance differs")
    cases = assessment["cases"]
    if (len(cases) != len({s["case"] for s in expected.values()})
            or {c["id"] for c in cases} != {s["case"] for s in expected.values()}):
        raise ValueError("Every deferred case needs one recovery assessment")
    for case in cases:
        if (case["decision"] != "needs_evidence" or not case["remaining_gaps"]
                or not all(isinstance(g, str) and g.strip() for g in case["remaining_gaps"])
                or not case["reason"] or not case["next_action"]
                or len({r["source"] for r in case["references"]}) != len(case["references"])):
            raise ValueError("This recovery assessment does not authorize replay admission")
        for item in case["references"]:
            record = by_id[item["source"]]
            if (expected[item["source"]]["case"] != case["id"]
                    or expected[item["source"]]["kind"] != "reference_file"
                    or record["outcome"] != "response" or record["http_status"] != 200
                    or record["content_type"] not in {"text/plain", "text/csv", "application/octet-stream"}
                    or item["relation"] != "current_reference_not_original"
                    or not item["anchors"]
                    or any(not a.strip() or len(a.split()) > 12 for a in item["anchors"])):
                raise ValueError("A reference receipt does not establish the reporter's original")
            if cache:
                text = (cache / item["source"]).read_text(encoding="utf-8-sig")
                if any(a not in text for a in item["anchors"]):
                    raise ValueError("Reference anchor absent from cached bytes")
    return {"schema_version": 1, "plan_sha256": snapshot["plan_sha256"],
            "snapshot_sha256": source.sha256(snapshot_path.read_bytes()),
            "assessment_sha256": source.sha256(assessment_path.read_bytes()),
            "summary": {"cases": len(cases), "planned_urls": len(expected),
                        "successful_http_responses": sum(r["outcome"] == "response" and
                            200 <= r["http_status"] < 300 for r in records),
                        "http_errors": sum(r["outcome"] == "response" and
                            not 200 <= r["http_status"] < 300 for r in records),
                        "unavailable_requests": sum(r["outcome"] == "unavailable" for r in records),
                        "current_reference_files": sum(len(c["references"]) for c in cases),
                        "needs_evidence": len(cases), "new_replay_candidates": 0,
                        "gold_defect_pairs": 0}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("fetch", "verify"))
    parser.add_argument("--plan", type=Path, default=PLAN)
    parser.add_argument("--assessment", type=Path, default=ASSESSMENT)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--snapshot", type=Path)
    parser.add_argument("--cache", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if args.command == "fetch" and args.output is None:
        parser.error("fetch requires --output")
    if args.command == "verify" and args.snapshot is None:
        parser.error("verify requires --snapshot")
    try:
        if args.command == "fetch":
            result = fetch(args.plan, args.output)
            print(json.dumps(result, indent=2, sort_keys=True))
            return 0 if all(r["outcome"] == "response" and 200 <= r["http_status"] < 300
                            for r in result["sources"]) else 1
        report = verify(args.snapshot, args.plan, args.assessment, args.cache)
        if args.report and report != json.loads(args.report.read_bytes()):
            raise ValueError("Published recovery report differs")
        print(json.dumps(report, indent=2, sort_keys=True))
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
