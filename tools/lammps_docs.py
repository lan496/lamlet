#!/usr/bin/env python3
"""Index every source document in a pinned official LAMMPS manual.

The catalog records provenance and review coverage, not executable rules mined
from prose. Full text and candidate paragraphs remain in the ignored work tree.
"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile

from corpus import write_json


ROOT = Path(__file__).resolve().parents[1]
REVISION = "0165101ce505734e8fb249694b22e295a7ab4182"
VERSION = "30 Sep 2026"
BASE = "https://docs.lammps.org/"
SIGNAL = re.compile(r"\b(must|cannot|must not|required|should|only|not supported|not compatible)\b", re.I)


def sections(text):
    lines = text.splitlines()
    headings = []
    for index, (line, underline) in enumerate(zip(lines, lines[1:])):
        if line.strip() and re.fullmatch(r'[=\-"~^+`:#*]{3,}', underline.strip()):
            headings.append((line.strip(), index + 1))
    return [{"title": title, "start_line": start,
             "end_line": headings[i + 1][1] - 1 if i + 1 < len(headings) else len(lines)}
            for i, (title, start) in enumerate(headings)]


def page_record(name, data):
    text = data.decode("utf-8")
    headings = sections(text)
    indexes = sorted(set(re.findall(r"^\.\. index::\s*(.+)$", text, re.M)))
    return {"path": name, "url": BASE + name.removesuffix(".rst") + ".html",
            "source_url": f"https://github.com/lammps/lammps/blob/{REVISION}/doc/src/{name}",
            "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data),
            "title": headings[0]["title"] if headings else name,
            "commands": indexes, "sections": headings,
            "normative_signals": len(SIGNAL.findall(text)),
            "autodoc": bool(re.search(r"^\.\. auto(?:module|class|function)::", text, re.M))}


def collect(args):
    archive = subprocess.check_output(
        ["git", "archive", "--format=tar", REVISION, "doc/src"], cwd=args.checkout)
    pages, candidates = [], []
    args.work.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(archive)) as source:
        for member in sorted(source.getmembers(), key=lambda m: m.name):
            if not member.isfile() or not member.name.endswith(".rst"):
                continue
            name = member.name.removeprefix("doc/src/")
            data = source.extractfile(member).read()
            page = page_record(name, data)
            pages.append(page)
            destination = args.work / "text" / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
            text = data.decode("utf-8")
            for match in re.finditer(r"(?:[^\n]+\n?)+", text):
                paragraph = match.group().strip()
                if SIGNAL.search(paragraph):
                    candidates.append({"page": name, "line": text.count("\n", 0, match.start()) + 1,
                                       "text": paragraph, "review": "pending"})
    version = subprocess.check_output(
        ["git", "show", REVISION + ":src/version.h"], cwd=args.checkout, text=True)
    if VERSION not in version:
        raise ValueError("Pinned source version differs from the manual version")
    catalog = {"schema_version": 1, "repository": "https://github.com/lammps/lammps.git",
               "revision": REVISION, "version": VERSION, "base_url": BASE,
               "scope": "Every .rst file under doc/src; historical HTML, images and generated autodoc bodies excluded",
               "coverage": "indexed, not semantically verified; normative signals are review candidates only",
               "pages": pages, "page_count": len(pages),
               "source_bytes": sum(p["bytes"] for p in pages),
               "candidate_paragraphs": len(candidates)}
    write_json(args.output, catalog)
    write_json(args.work / "candidates.json", candidates)
    print(json.dumps({k: v for k, v in catalog.items() if k != "pages"}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--work", type=Path, default=ROOT / "target/lammps-manual")
    parser.add_argument("--output", type=Path, default=ROOT / "crates/lamlet/data/lammps_docs.json")
    collect(parser.parse_args())
