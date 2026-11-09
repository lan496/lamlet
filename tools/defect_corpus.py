#!/usr/bin/env python3
"""Collect public defect reports without running Lamlet or assigning gold labels."""

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import sys
import urllib.parse
import urllib.request

from corpus import write_json


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "research/defects/cases.json"
USER_AGENT = "Lamlet research corpus collector (https://github.com/lan496/lamlet)"
DISPOSITIONS = {"reported_fix", "proposed_fix", "counterexample", "engine_bug", "unresolved"}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


class Post(HTMLParser):
    """Read text, code blocks, and attachment links without evaluating markup."""

    def __init__(self, cooked):
        super().__init__(convert_charrefs=True)
        self.text = []
        self.code = []
        self.attachments = []
        self.pre = None
        self.feed(cooked)
        self.close()

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "pre":
            self.pre = []
        if tag in ("p", "pre", "br", "li", "blockquote"):
            self.text.append("\n")
        if tag == "a" and "attachment" in attrs.get("class", "").split():
            self.attachments.append(attrs["href"])

    def handle_endtag(self, tag):
        if tag == "pre" and self.pre is not None:
            self.code.append("".join(self.pre))
            self.pre = None
        if tag in ("p", "pre", "li", "blockquote"):
            self.text.append("\n")

    def handle_data(self, data):
        self.text.append(data)
        if self.pre is not None:
            self.pre.append(data)


def read_url(url):
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=30) as response:
        data = response.read(16 * 1024 * 1024 + 1)
    if len(data) > 16 * 1024 * 1024:
        raise ValueError(f"Download exceeds 16 MiB: {url}")
    return data


def topic_id(url):
    match = re.fullmatch(r"https://matsci\.org/t/[a-z0-9-]+/(\d+)", url)
    if match is None:
        raise ValueError(f"Expected a canonical matsci topic URL: {url}")
    return int(match[1])


def selection_sha256(cases):
    selection = [{"id": case["id"], "url": case["url"]} for case in cases]
    return sha256(json.dumps(selection, sort_keys=True).encode())


def complete_posts(topic, extra=()):
    posts = topic["post_stream"]["posts"] + list(extra)
    expected = topic["post_stream"]["stream"]
    by_id = {post["id"]: post for post in posts}
    if len(by_id) != len(posts) or set(by_id) != set(expected):
        raise ValueError("Incomplete or duplicate topic posts")
    result = sorted(posts, key=lambda post: post["post_number"])
    if len({post["post_number"] for post in result}) != len(result):
        raise ValueError("Duplicate post numbers")
    return result


def collect(case, directory):
    url = case["url"]
    identity = topic_id(url)
    raw = read_url(url + ".json")
    topic = json.loads(raw)
    if topic["id"] != identity:
        raise ValueError("Fetched topic has the wrong identity")
    (directory / "topic.json").write_bytes(raw)
    extra = []
    present = {post["id"] for post in topic["post_stream"]["posts"]}
    missing = [post for post in topic["post_stream"]["stream"] if post not in present]
    for offset in range(0, len(missing), 20):
        query = urllib.parse.urlencode([("post_ids[]", post) for post in missing[offset:offset + 20]])
        data = read_url(f"https://matsci.org/t/{identity}/posts.json?{query}")
        (directory / f"posts-{offset}.json").write_bytes(data)
        extra.extend(json.loads(data)["post_stream"]["posts"])
    posts = complete_posts(topic, extra)
    canonical = {"id": identity, "title": topic["title"], "posts": [
        {key: post[key] for key in ("id", "post_number", "username", "created_at", "updated_at", "cooked")}
        for post in posts]}
    write_json(directory / "complete.json", canonical)
    receipts = []
    text = []
    for post in posts:
        parsed = Post(post["cooked"])
        text.extend([f"POST {post['post_number']} {post['username']} {post['created_at']}",
                     "".join(parsed.text)])
        receipts.append({"post_number": post["post_number"], "post_id": post["id"],
                         "cooked_sha256": sha256(post["cooked"].encode()),
                         "code_sha256": [sha256(block.encode()) for block in parsed.code]})
    (directory / "thread.txt").write_text("\n\n".join(text), encoding="utf-8")
    attachments = []
    for post in posts:
        for link in Post(post["cooked"]).attachments:
            absolute = urllib.parse.urljoin(url, link)
            parsed = urllib.parse.urlparse(absolute)
            if parsed.scheme != "https" or parsed.hostname not in {"matsci.org", "www.matsci.org"}:
                raise ValueError(f"Unexpected attachment host: {absolute}")
            # Never unpack or execute an attachment. Content-addressed names avoid
            # interpreting source filenames as local paths.
            name = "attachment-" + sha256(absolute.encode())[:16]
            record = {"url": absolute, "post_number": post["post_number"], "file": name}
            try:
                data = read_url(absolute)
                (directory / name).write_bytes(data)
                record.update(status="downloaded", sha256=sha256(data), bytes=len(data))
            except (OSError, ValueError) as error:
                record.update(status="unavailable", error=str(error))
            attachments.append(record)
    return {"id": case["id"], "url": url, "title": topic["title"],
            "posts": receipts, "attachments": attachments,
            "files": {path.name: {"sha256": sha256(path.read_bytes()), "bytes": path.stat().st_size}
                      for path in sorted(directory.iterdir()) if path.is_file()}}


def fetch(manifest, output):
    if output.exists():
        raise ValueError("Use a fresh snapshot directory")
    output.mkdir(parents=True)
    cases = json.loads(manifest.read_text())["cases"]
    snapshot = {"schema_version": 1, "selection_sha256": selection_sha256(cases),
                "retrieved_at": datetime.now(timezone.utc).isoformat(), "sources": [], "failures": []}
    for case in cases:
        directory = output / case["id"]
        directory.mkdir()
        try:
            snapshot["sources"].append(collect(case, directory))
            print(f"Collected {case['id']}", file=sys.stderr)
        except (OSError, ValueError, KeyError) as error:
            snapshot["failures"].append({"id": case["id"], "error": str(error)})
            print(f"Unavailable {case['id']}: {error}", file=sys.stderr)
        write_json(output / "snapshot.json", snapshot)
    return snapshot


def audit(manifest, lock, cache=None):
    specification = json.loads(manifest.read_text())
    if specification.get("schema_version") != 1 or lock.get("schema_version") != 1:
        raise ValueError("Unsupported collection schema")
    cases = specification["cases"]
    if lock.get("selection_sha256") != selection_sha256(cases):
        raise ValueError("Source selection changed")
    ids = [case["id"] for case in cases]
    if len(set(ids)) != len(ids) or any(re.fullmatch(r"m\d+", item) is None for item in ids):
        raise ValueError("Invalid or duplicate case IDs")
    sources = {source["id"]: source for source in lock["sources"]}
    failures = {source["id"]: source for source in lock["failures"]}
    if (len(sources) != len(lock["sources"]) or len(failures) != len(lock["failures"])
            or set(sources) & set(failures) or set(sources) | set(failures) != set(ids)):
        raise ValueError("Every screened source needs exactly one capture outcome")
    dispositions = Counter()
    families = Counter()
    for case in cases:
        if case["disposition"] not in DISPOSITIONS:
            raise ValueError(f"Unknown disposition: {case['id']}")
        if case["review"] != "pending" or case["split"] != "development":
            raise ValueError("This pilot has no independent adjudication or held-out labels")
        if not all(case.get(key) for key in ("intent", "condition", "reason", "blockers", "family", "evidence")):
            raise ValueError(f"Missing screening evidence: {case['id']}")
        if any(not e.get("anchor", "").strip() or not e.get("role") for e in case["evidence"]):
            raise ValueError("Evidence requires a nonempty text anchor and role")
        if topic_id(case["url"]) != int(case["id"][1:]):
            raise ValueError("Case and topic identities disagree")
        dispositions[case["disposition"]] += 1
        families[case["family"]] += 1
        source = sources.get(case["id"])
        if source is None:
            continue
        if source["url"] != case["url"]:
            raise ValueError("Source URL changed")
        posts = {post["post_number"] for post in source["posts"]}
        if len(posts) != len(source["posts"]) or any(e["post"] not in posts for e in case["evidence"]):
            raise ValueError(f"Evidence refers to an unavailable post: {case['id']}")
        for name, record in source["files"].items():
            if Path(name).name != name or re.fullmatch(r"[a-f0-9]{64}", record["sha256"]) is None:
                raise ValueError("Invalid cached file receipt")
            if cache:
                data = (cache / case["id"] / name).read_bytes()
                if sha256(data) != record["sha256"] or len(data) != record["bytes"]:
                    raise ValueError(f"Changed source snapshot: {case['id']}/{name}")
        for attachment in source["attachments"]:
            if attachment["post_number"] not in posts:
                raise ValueError("Attachment refers to an unavailable post")
            if attachment["status"] not in {"downloaded", "unavailable"}:
                raise ValueError("Unknown attachment status")
            if attachment["status"] == "downloaded":
                record = source["files"].get(attachment["file"])
                if record != {key: attachment[key] for key in ("sha256", "bytes")}:
                    raise ValueError("Attachment receipt and file inventory disagree")
        if cache:
            directory = cache / case["id"]
            topic = json.loads((directory / "topic.json").read_bytes())
            extra = []
            for path in sorted(directory.glob("posts-*.json")):
                extra.extend(json.loads(path.read_bytes())["post_stream"]["posts"])
            captured = complete_posts(topic, extra)
            by_number = {post["post_number"]: post for post in captured}
            if set(by_number) != posts:
                raise ValueError("Captured thread does not match post inventory")
            for receipt in source["posts"]:
                post = by_number[receipt["post_number"]]
                if (post["id"] != receipt["post_id"]
                        or sha256(post["cooked"].encode()) != receipt["cooked_sha256"]
                        or [sha256(block.encode()) for block in Post(post["cooked"]).code]
                        != receipt["code_sha256"]):
                    raise ValueError("Post receipt and captured content disagree")
            for evidence in case["evidence"]:
                text = "".join(Post(by_number[evidence["post"]]["cooked"]).text)
                anchor = " ".join(evidence["anchor"].split())
                if not anchor or anchor not in " ".join(text.split()):
                    raise ValueError(f"Missing evidence anchor: {case['id']} post {evidence['post']}")
    return {"schema_version": 1, "screened": len(cases), "captured": len(sources),
            "capture_failures": len(failures), "dispositions": dict(sorted(dispositions.items())),
            "families": dict(sorted(families.items())), "independently_adjudicated": 0,
            "gold_defect_pairs": 0, "checker_evaluated": False, "held_out_cases": 0,
            "cached_bytes_verified": cache is not None,
            "downloaded_attachments": len({a["url"] for s in sources.values()
                                           for a in s["attachments"] if a["status"] == "downloaded"}),
            "unavailable_attachments": len({a["url"] for s in sources.values()
                                            for a in s["attachments"] if a["status"] != "downloaded"}),
            "manifest_sha256": sha256(manifest.read_bytes())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("fetch", "audit"))
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--snapshot", type=Path, default=ROOT / "research/defects/source-lock.json")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--cache", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "fetch":
            if args.output is None:
                parser.error("fetch requires --output")
            result = fetch(args.manifest, args.output)
            print(json.dumps({"captured": len(result["sources"]), "failures": result["failures"]}, sort_keys=True))
            return 1 if result["failures"] else 0
        result = audit(args.manifest, json.loads(args.snapshot.read_text()), args.cache)
        if args.output:
            write_json(args.output, result)
        print(json.dumps(result, indent=2, sort_keys=True))
        return 1 if result["capture_failures"] else 0
    except (OSError, ValueError, KeyError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
