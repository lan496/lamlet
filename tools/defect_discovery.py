#!/usr/bin/env python3
"""Discover a bounded public forum cohort without assigning defect labels."""

import argparse
from collections import Counter
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import shutil
import sys
import time

from corpus import write_json
import defect_corpus as source


ROOT = source.ROOT
PLAN = ROOT / "research/defects/discovery-plan.json"
PUBLIC = ("plan.json", "listing.json", "sources.json", "report.json",
          "screening.json", "packet.md", "budget.json")


def read_plan(path):
    plan = json.loads(path.read_bytes())
    if (plan["schema_version"] != 1 or plan["category_url"] != "https://matsci.org/c/lammps/40"
            or not plan["pages"] or plan["pages"] != list(range(len(plan["pages"])))
            or type(plan["capture_limit"]) is not int or plan["capture_limit"] < 1
            or len(set(plan["previously_screened_topic_ids"])) != len(plan["previously_screened_topic_ids"])):
        raise ValueError("Invalid bounded discovery plan")
    return plan


def parse_page(data):
    listing = json.loads(data)["topic_list"]
    topics = []
    for topic in listing["topics"]:
        if (type(topic["id"]) is not int or topic["id"] < 1
                or not re.fullmatch(r"[a-z0-9-]+", topic["slug"])
                or not isinstance(topic["title"], str)
                or type(topic["pinned"]) is not bool
                or type(topic["category_id"]) is not int
                or type(topic["posts_count"]) is not int):
            raise ValueError("Invalid topic listing identity or metadata")
        topics.append({key: topic[key] for key in
                       ("id", "title", "slug", "pinned", "category_id", "posts_count")})
    return topics


def select(plan, pages):
    if [page["page"] for page in pages] != plan["pages"]:
        raise ValueError("Every requested page needs exactly one ordered outcome")
    candidates = {}
    selected = 0
    for page in pages:
        receipt = page.get("raw")
        if receipt is not None and (re.fullmatch(r"[a-f0-9]{64}", receipt["sha256"]) is None
                                    or type(receipt["bytes"]) is not int or receipt["bytes"] < 0):
            raise ValueError("Invalid listing receipt")
        if page["status"] != "collected":
            if page["status"] != "unavailable" or not page["error"] or page["topics"]:
                raise ValueError("Invalid unavailable-page outcome")
            continue
        if receipt is None:
            raise ValueError("Collected listing has no receipt")
        for row, topic in enumerate(page["topics"]):
            identity = f"m{topic['id']}"
            if identity not in candidates:
                if topic["pinned"]:
                    state = "pinned"
                elif topic["id"] in plan["previously_screened_topic_ids"]:
                    state = "previously_screened"
                elif selected < plan["capture_limit"]:
                    state = "selected"
                    selected += 1
                else:
                    state = "budget_deferred"
                candidates[identity] = {"id": identity,
                    "url": f"https://matsci.org/t/{topic['slug']}/{topic['id']}",
                    "title": topic["title"], "category_id": topic["category_id"],
                    "listed_posts_count": topic["posts_count"], "selection": state,
                    "occurrences": []}
            candidates[identity]["occurrences"].append({"page": page["page"], "row": row})
    return list(candidates.values())


def capture_records(candidates, snapshot, cache=None):
    selected = {c["id"]: c for c in candidates if c["selection"] == "selected"}
    good = {c["id"]: c for c in snapshot["sources"]}
    failed = {c["id"]: c for c in snapshot["failures"]}
    if (len(good) != len(snapshot["sources"]) or len(failed) != len(snapshot["failures"])
            or set(good) & set(failed) or set(good) | set(failed) != set(selected)):
        raise ValueError("Every selected topic needs exactly one capture outcome")
    for identity, record in good.items():
        if record["url"] != selected[identity]["url"]:
            raise ValueError("Capture and discovery identities disagree")
        posts = record["posts"]
        if (not posts or len({p["post_number"] for p in posts}) != len(posts)
                or len({p["post_id"] for p in posts}) != len(posts)):
            raise ValueError("Invalid complete-thread receipt")
        for name, receipt in record["files"].items():
            if (Path(name).name != name or name in {".", ".."}
                    or re.fullmatch(r"[a-f0-9]{64}", receipt["sha256"]) is None
                    or type(receipt["bytes"]) is not int or receipt["bytes"] < 0):
                raise ValueError("Invalid source-file receipt")
            if cache:
                data = (cache / "threads" / identity / name).read_bytes()
                if source.sha256(data) != receipt["sha256"] or len(data) != receipt["bytes"]:
                    raise ValueError("Changed source bytes")
        for attachment in record["attachments"]:
            if (attachment["post_number"] not in {p["post_number"] for p in posts}
                    or attachment["status"] not in {"downloaded", "unavailable"}):
                raise ValueError("Invalid attachment outcome")
            if attachment["status"] == "downloaded" and record["files"].get(attachment["file"]) != {
                    key: attachment[key] for key in ("sha256", "bytes")}:
                raise ValueError("Attachment receipt differs from source-file inventory")
        if cache:
            at = cache / "threads" / identity
            topic = json.loads((at / "topic.json").read_bytes())
            if topic["id"] != int(identity[1:]):
                raise ValueError("Wrong captured topic identity")
            extra = [post for path in sorted(at.glob("posts-*.json"))
                     for post in json.loads(path.read_bytes())["post_stream"]["posts"]]
            actual = [{"post_number": p["post_number"], "post_id": p["id"],
                       "cooked_sha256": source.sha256(p["cooked"].encode()),
                       "code_sha256": [source.sha256(c.encode()) for c in source.Post(p["cooked"]).code]}
                      for p in source.complete_posts(topic, extra)]
            if actual != posts:
                raise ValueError("Complete posts differ from capture receipts")
    if any(not record.get("error") for record in failed.values()):
        raise ValueError("Failed captures require a reason")
    return good, failed


def assemble(plan_path, pages, snapshot, cache=None):
    plan = read_plan(plan_path)
    candidates = select(plan, pages)
    good, failed = capture_records(candidates, snapshot, cache)
    for candidate in candidates:
        identity = candidate["id"]
        if identity in good:
            record = good[identity]
            attachments = {a["url"]: a for a in record["attachments"]}
            candidate.update(outcome="captured", structural={
                "posts": len(record["posts"]),
                "initial_post_code_blocks": sum(len(p["code_sha256"]) for p in record["posts"] if p["post_number"] == 1),
                "all_post_code_blocks": sum(len(p["code_sha256"]) for p in record["posts"]),
                "downloaded_attachments": sum(a["status"] == "downloaded" for a in attachments.values()),
                "unavailable_attachments": sum(a["status"] == "unavailable" for a in attachments.values())})
        elif identity in failed:
            candidate.update(outcome="capture_failed", error=failed[identity]["error"])
        else:
            candidate["outcome"] = candidate["selection"]
    counts = Counter(c["outcome"] for c in candidates)
    rows = sum(len(page["topics"]) for page in pages)
    return {"schema_version": 1, "plan_sha256": source.sha256(plan_path.read_bytes()),
            "cohort": plan["cohort"], "split": "development", "checker_evaluated": False,
            "screened_topics": 0, "independently_adjudicated": 0, "gold_defect_pairs": 0,
            "summary": {"requested_pages": len(pages),
                "collected_pages": sum(p["status"] == "collected" for p in pages),
                "listing_slots": rows, "unique_topics": len(candidates),
                "duplicate_slots": rows - len(candidates), "selected_captures": len(good) + len(failed),
                "outcomes": dict(sorted(counts.items()))}, "candidates": candidates}


def screening(report):
    return {"schema_version": 1, "plan_sha256": report["plan_sha256"],
            "reviewer": None, "cases": [{"id": c["id"], "url": c["url"],
                "capture_outcome": c["outcome"], "decision": "pending", "intent": None,
                "condition": None, "proposed_correction": None, "evidence_posts": [],
                "dependency_gaps": [], "reason": None}
                for c in report["candidates"] if c["selection"] == "selected"]}


def packet(report):
    lines = ["# Screening discovered LAMMPS discussions", "",
             "This is a candidate queue. All decisions are pending. A code block or attachment",
             "does not establish a complete input, defect, or correction. Read every post,",
             "including retractions. Record intent, bounded condition, dependencies, and",
             "source post numbers in a copy of screening.json. Missing evidence remains",
             "unresolved. Screening is separate from independent pair admission.", "",
             "Raw threads and attachments stay in the exact local source cache. Open them",
             "as data; never execute attachments. Checker observations are excluded.", ""]
    for candidate in report["candidates"]:
        if candidate["selection"] == "selected":
            lines.extend([f"## {candidate['id']}", "", candidate["url"], "",
                          f"Capture outcome: {candidate['outcome']}.", "",
                          f"Cache: threads/{candidate['id']}/thread.txt", ""])
    return "\n".join(lines)


def fetch(plan_path, output):
    plan = read_plan(plan_path)
    output.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(plan_path, output / "plan.json")
    pages = []
    budget = {}
    write_json(output / "capture.json", {"retrieved_at": datetime.now(timezone.utc).isoformat(),
                                        "collector_sha256": source.sha256(Path(__file__).read_bytes())})
    for page in plan["pages"]:
        started = time.perf_counter()
        entry = {"page": page, "status": "unavailable", "topics": [], "raw": None}
        try:
            data = source.read_url(f"{plan['category_url']}.json?page={page}")
            (output / f"page-{page}.json").write_bytes(data)
            entry["raw"] = {"sha256": source.sha256(data), "bytes": len(data)}
            entry.update(status="collected", topics=parse_page(data))
        except (OSError, ValueError, KeyError, TypeError) as error:
            entry["error"] = str(error)
        pages.append(entry)
        write_json(output / "listing.json", pages)
        if page == plan["pages"][0]:
            elapsed = time.perf_counter() - started
            budget["listing"] = {"first_seconds": elapsed, "remaining_pages": len(plan["pages"]) - 1,
                                 "estimated_remaining_seconds": elapsed * (len(plan["pages"]) - 1)}
            print(f"First listing: {elapsed:.3f}s; remaining estimate: {budget['listing']['estimated_remaining_seconds']:.3f}s", file=sys.stderr)
    snapshot = {"sources": [], "failures": []}
    selected = [c for c in select(plan, pages) if c["selection"] == "selected"]
    for number, candidate in enumerate(selected):
        started = time.perf_counter()
        at = output / "threads" / candidate["id"]
        at.mkdir(parents=True)
        try:
            snapshot["sources"].append(source.collect(candidate, at))
            print(f"Captured {candidate['id']}", file=sys.stderr)
        except (OSError, ValueError, KeyError) as error:
            snapshot["failures"].append({"id": candidate["id"], "error": str(error).replace(str(output), "cache")})
            print(f"Unavailable {candidate['id']}: {error}", file=sys.stderr)
        write_json(output / "sources.json", snapshot)
        if number == 0:
            elapsed = time.perf_counter() - started
            budget["thread"] = {"first_seconds": elapsed, "remaining_topics": len(selected) - 1,
                                "estimated_remaining_seconds": elapsed * (len(selected) - 1)}
            print(f"First thread: {elapsed:.3f}s; remaining estimate: {budget['thread']['estimated_remaining_seconds']:.3f}s", file=sys.stderr)
    write_json(output / "sources.json", snapshot)
    write_json(output / "budget.json", budget)
    report = assemble(output / "plan.json", pages, snapshot, output)
    write_json(output / "report.json", report)
    write_json(output / "screening.json", screening(report))
    (output / "packet.md").write_text(packet(report), encoding="utf-8")
    return report


def verify(directory, cache=None):
    if directory != cache and not (directory / "inventory.json").is_file():
        raise ValueError("Missing public discovery inventory")
    pages = json.loads((directory / "listing.json").read_bytes())
    if cache:
        for page in pages:
            if page["raw"] is not None:
                data = (cache / f"page-{page['page']}.json").read_bytes()
                if page["raw"] != {"sha256": source.sha256(data), "bytes": len(data)}:
                    raise ValueError("Changed listing bytes")
                try:
                    actual = parse_page(data)
                except (ValueError, KeyError, TypeError):
                    if page["status"] == "collected":
                        raise ValueError("Invalid collected listing")
                else:
                    if page["status"] != "collected" or actual != page["topics"]:
                        raise ValueError("Listing metadata differs from captured bytes")
            elif page["status"] == "collected":
                raise ValueError("Collected listing has no receipt")
    snapshot = json.loads((directory / "sources.json").read_bytes())
    report = assemble(directory / "plan.json", pages, snapshot, cache)
    if (report != json.loads((directory / "report.json").read_bytes())
            or screening(report) != json.loads((directory / "screening.json").read_bytes())
            or packet(report) != (directory / "packet.md").read_text()):
        raise ValueError("Discovery report or pending screening packet differs")
    if (directory / "inventory.json").is_file():
        inventory = json.loads((directory / "inventory.json").read_bytes())
        if set(inventory) != set(PUBLIC) | {"capture.json"} or {p.name for p in directory.iterdir()} != set(inventory) | {"inventory.json"}:
            raise ValueError("Public discovery inventory differs")
        for name, digest in inventory.items():
            if source.sha256((directory / name).read_bytes()) != digest:
                raise ValueError("Changed public discovery artifact")
    return {"unique_topics": report["summary"]["unique_topics"],
            "selected_captures": report["summary"]["selected_captures"],
            "cached_bytes_verified": cache is not None, "screened_topics": 0,
            "gold_defect_pairs": 0, "checker_evaluated": False}


def publish(cache, output):
    verify(cache, cache)
    output.mkdir(parents=True, exist_ok=False)
    for name in (*PUBLIC, "capture.json"):
        shutil.copyfile(cache / name, output / name)
    write_json(output / "inventory.json", {p.name: source.sha256(p.read_bytes()) for p in sorted(output.iterdir())})
    return verify(output, cache)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("fetch", "publish", "verify"))
    parser.add_argument("--plan", type=Path, default=PLAN)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--cache", type=Path)
    parser.add_argument("--archive", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "fetch":
            if args.output is None:
                parser.error("fetch requires a fresh --output directory")
            report = fetch(args.plan, args.output)
            result = report["summary"]
            failed = result["collected_pages"] != result["requested_pages"] or result["outcomes"].get("capture_failed", 0)
        elif args.command == "publish":
            if args.cache is None or args.output is None:
                parser.error("publish requires --cache and a fresh --output directory")
            result = publish(args.cache, args.output)
            failed = False
        else:
            if args.archive is None:
                parser.error("verify requires --archive")
            result = verify(args.archive, args.cache)
            failed = False
        print(json.dumps(result, sort_keys=True))
        return int(bool(failed))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
