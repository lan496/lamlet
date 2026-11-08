#!/usr/bin/env python3
"""Fetch pinned inputs and measure the real Lamlet CLI; Python standard library only."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/corpus/sources.json"
COMMAND = re.compile(rb"^[ \t]*([a-zA-Z_][a-zA-Z0-9_]*)", re.MULTILINE)
FEATURES = {
    "include": rb"^[ \t]*include\s",
    "jump": rb"^[ \t]*jump\s",
    "if": rb"^[ \t]*if\s",
    "next": rb"^[ \t]*next\s",
    "variable_substitution": rb"^[^#\r\n]*\$(?:\{|\(|[a-zA-Z0-9_])",
    "continuation": rb"&[ \t]*\r?$",
    "triple_quotes": rb'^[^#\r\n]*"""',
}


def git(directory, *args):
    return subprocess.run(
        ["git", *args], cwd=directory, check=True, capture_output=True
    ).stdout


def manifest():
    return json.loads(MANIFEST.read_text(encoding="utf-8"))["sources"]


def fetch(cache):
    for source in manifest():
        directory = cache / source["id"]
        fresh = not directory.exists()
        if fresh:
            directory.parent.mkdir(parents=True, exist_ok=True)
            # Sparse checkout avoids downloading simulation data.
            git(ROOT, "clone", "--filter=blob:none", "--no-checkout", "--depth", "1",
                source["url"], str(directory))
        if git(directory, "config", "--get", "remote.origin.url").decode().strip() != source["url"]:
            raise ValueError(f"Unexpected origin in {directory}")
        if not fresh and git(directory, "status", "--porcelain").strip():
            raise ValueError(f"Refusing to change a dirty corpus checkout: {directory}")
        git(directory, "fetch", "--depth", "1", "origin", source["commit"])
        git(directory, "sparse-checkout", "set", "--no-cone", *source["sparse"])
        git(directory, "checkout", "--detach", source["commit"])
        print(f"Fetched {source['id']} at {source['commit']}", file=sys.stderr)


def selected(path, group):
    name = PurePosixPath(path)
    return (
        path.startswith(group.get("prefix", ""))
        and name.name.startswith(group.get("basename_prefix", ""))
        and ("suffixes" not in group or name.suffix in group["suffixes"])
        and ("paths" not in group or path in group["paths"])
    )


def fragments(path, data, group):
    """Extract declared script bodies without evaluating notebook or Markdown code."""
    if group.get("notebook_writefiles"):
        result = []
        for cell in json.loads(data)["cells"]:
            if cell["cell_type"] != "code":
                continue
            text = cell.get("source", "")
            text = text if isinstance(text, str) else "".join(text)
            if not text.startswith("%%writefile"):
                continue
            match = re.match(r"%%writefile[ \t]+([^\s]+)[ \t]*\r?\n", text)
            if not match:
                raise ValueError(f"Unsupported writefile header: {path}")
            name = f"{path}#writefile={match[1]}"
            if any(p == name for p, _ in result):
                raise ValueError(f"Ambiguous writefile target: {name}")
            result.append((name, text[match.end():].encode("utf-8")))
        return result
    if "markdown_fence" in group:
        declaration = group["markdown_fence"]
        # Match the opening indentation on the closing fence. Keep body bytes,
        # including indentation and line endings, exactly as stored upstream.
        blocks = re.findall(rb"(?m)^([ \t]*)```[^\r\n]*\r?\n(.*?)^\1```[ \t]*\r?$",
                            data, re.DOTALL)
        bodies = [body for _, body in blocks
                  if declaration["anchor"].encode() in body.splitlines()]
        if len(bodies) != 1:
            raise ValueError(f"Expected one declared Markdown example: {path}")
        return [(f"{path}#example={declaration['name']}", bodies[0])]
    return [(path, data)]


def inputs(cache):
    result = []
    selection = {}
    for source in manifest():
        directory = cache / source["id"]
        actual = git(directory, "rev-parse", "HEAD").decode().strip()
        if actual != source["commit"]:
            raise ValueError(f"{source['id']}: expected {source['commit']}, found {actual}")
        # Enumerate Git's tree, not just the working files: missing sparse files
        # must fail the evaluation rather than silently reducing its denominator.
        for entry in git(directory, "ls-tree", "-rz", "HEAD").split(b"\0"):
            if not entry:
                continue
            metadata, raw_path = entry.split(b"\t", 1)
            path = raw_path.decode("utf-8")
            for group in source["groups"]:
                if selected(path, group):
                    counts = selection.setdefault(group["id"], {"candidates": 0, "excluded": {}})
                    if path in source["exclude"]:
                        counts["candidates"] += 1
                        counts["excluded"][path] = source["exclude"][path]
                        continue
                    mode, kind, blob = metadata.decode().split()
                    if kind != "blob" or mode not in ("100644", "100755"):
                        raise ValueError(f"Not a regular input: {source['id']}/{path}")
                    data = (directory / path).read_bytes()
                    digest = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
                    if digest != blob:
                        raise ValueError(f"Modified corpus input: {source['id']}/{path}")
                    for name, body in fragments(path, data, group):
                        counts["candidates"] += 1
                        if name in source["exclude"]:
                            counts["excluded"][name] = source["exclude"][name]
                            continue
                        input_path = directory / path
                        if name != path:
                            key = hashlib.sha256(name.encode()).hexdigest()
                            input_path = cache / ".extracted" / source["id"] / (key + ".in")
                            input_path.parent.mkdir(parents=True, exist_ok=True)
                            input_path.write_bytes(body)
                        result.append((group["id"], source["id"], name, input_path, body))
    return sorted(result, key=lambda item: (item[0], item[2])), selection


def findings(binary, path, output=None, strict=False):
    command = [str(binary), "build" if output else "check", str(path),
               "--language", "lammps", "--format", "json"]
    if output:
        command += ["-o", str(output)]
    if strict:
        command.append("--strict")
    process = subprocess.run(command, capture_output=True, timeout=30)
    if process.returncode not in (0, 1):
        raise ValueError(f"CLI failed for {path}: {process.stdout!r} {process.stderr!r}")
    report = json.loads(process.stdout)
    if bool(report["summary"]["errors"]) != (process.returncode == 1):
        raise ValueError(f"CLI exit status and JSON disagree for {path}")
    return report


def analyze_text(data):
    # Deliberately reproduce the user's physical-line command-frequency rule.
    # These are lexical candidates, not parsed/executed command counts.
    commands = Counter(match.decode("ascii") for match in COMMAND.findall(data))
    features = [name for name, pattern in FEATURES.items()
                if re.search(pattern, data, re.MULTILINE)]
    return commands, features


def ranked(counter, limit=None):
    return [{"command": key, "count": value}
            for key, value in sorted(counter.items(), key=lambda item: (-item[1], item[0]))[:limit]]


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
                    encoding="utf-8")


def negative_cases(binary, cache):
    fixture = json.loads((ROOT / "tests/corpus/negative.json").read_text(encoding="utf-8"))
    source = next(s for s in manifest() if s["id"] == fixture["source"])
    if source["commit"] != fixture["commit"]:
        raise ValueError("Negative fixtures and source revision disagree")
    directory = cache / source["id"]
    raw_reference = (directory / fixture["path"]).read_bytes()
    blob = git(directory, "rev-parse", f"{fixture['commit']}:{fixture['path']}").decode().strip()
    digest = hashlib.sha1(b"blob " + str(len(raw_reference)).encode() + b"\0" + raw_reference).hexdigest()
    if digest != blob:
        raise ValueError("Modified upstream negative-test source")
    reference = raw_reference.decode()
    # Compare the minimal extracted commands with their pinned C++ test anchors.
    results = []
    with tempfile.TemporaryDirectory(prefix="lamlet-negative-") as temporary:
        path = Path(temporary) / "negative.in"
        for case in fixture["cases"]:
            block = re.search(r"TEST_F\([^,]+,\s*" + re.escape(case["upstream_case"])
                              + r"\)\s*\{.*?(?=\nTEST_F|\Z)", reference, re.DOTALL)
            if block is None or case["anchor"] not in block.group():
                raise ValueError(f"Missing upstream test anchor: {case['id']}")
            path.write_text(case["script"], encoding="utf-8")
            reports = [findings(binary, path, strict=strict) for strict in (False, True)]
            passed = all(
                any(d["severity"] == "error" and d["code"] == case["error"]
                    for d in report["diagnostics"])
                if case["error"] else not report["summary"]["errors"] and report["summary"]["unknowns"] > 0
                for report in reports
            )
            results.append({"id": case["id"], "expected_error": case["error"], "passed": passed})
    return {"provenance": {key: fixture[key] for key in ("source", "commit", "path")},
            "covered": sum(bool(case["error"]) for case in fixture["cases"]),
            "outside_coverage": sum(not case["error"] for case in fixture["cases"]),
            "cases": results}


def focus_subsets(inventory):
    subsets = {}
    for name in ("UNITS", "ELASTIC", "MC-LOOP", "PACKAGES"):
        items = [item for item in inventory
                 if item["file"].startswith(f"lammps/examples/{name}/")]
        subsets[name] = {
            "files": len(items),
            "round_trip_identical": sum(item["round_trip"] == "identical" for item in items),
            "static_rejected": sum(bool(item["diagnostics"]["errors"]) for item in items),
        }
    return subsets


def audit(args):
    corpus, selection = inputs(args.cache)
    if args.limit is not None:
        corpus = corpus[:args.limit]
    if not corpus:
        raise ValueError("No selected input files")
    binary = args.binary.resolve()
    schema = json.loads(subprocess.run(
        [str(binary), "schema", "--format", "json"], check=True, capture_output=True
    ).stdout)
    groups = {}
    inventory = []
    frequencies = defaultdict(Counter)
    feature_paths = defaultdict(list)
    started = time.perf_counter()
    with tempfile.TemporaryDirectory(prefix="lamlet-corpus-") as temporary:
        output = Path(temporary) / "output.in"
        for group, source, path, input_path, data in corpus:
            counts = groups.setdefault(group, {
                "files": 0, "bytes": 0, "round_trip_identical": 0,
                "round_trip_different": 0, "build_rejected": 0,
                "static_rejected": 0, "strict_rejected": 0,
                "errors": 0, "warnings": 0, "unknowns": 0, "failures": []
            })
            counts["files"] += 1
            counts["bytes"] += len(data)
            checked = findings(binary, input_path)
            strict = findings(binary, input_path, strict=True)
            output.unlink(missing_ok=True)
            built = findings(binary, input_path, output)
            if checked != built:
                raise ValueError(f"check/build diagnostic mismatch: {source}/{path}")
            for severity in ("errors", "warnings", "unknowns"):
                counts[severity] += checked["summary"][severity]
            counts["static_rejected"] += bool(checked["summary"]["errors"])
            counts["strict_rejected"] += bool(strict["summary"]["errors"])
            if built["summary"]["errors"]:
                counts["build_rejected"] += 1
                status = "rejected"
            elif output.read_bytes() == data:
                counts["round_trip_identical"] += 1
                status = "identical"
            else:
                counts["round_trip_different"] += 1
                status = "different"
            commands, features = analyze_text(data)
            frequencies[group].update(commands)
            identifier = f"{source}/{path}"
            for feature in features:
                feature_paths[f"{group}/{feature}"].append(identifier)
            item = {"file": identifier, "bytes": len(data), "round_trip": status,
                    "sha256": hashlib.sha256(data).hexdigest(),
                    "diagnostics": checked["summary"], "features": features,
                    "warnings": [{**d, "file": identifier} for d in checked["diagnostics"]
                                 if d["severity"] == "warning"]}
            inventory.append(item)
            if status != "identical" or strict["summary"]["errors"]:
                errors = []
                for diagnostic in checked["diagnostics"]:
                    if diagnostic["severity"] == "error":
                        errors.append({**diagnostic, "file": identifier})
                strict_errors = [{**d, "file": identifier} for d in strict["diagnostics"]
                                 if d["severity"] == "error"]
                counts["failures"].append({"file": identifier, "round_trip": status,
                                            "errors": errors, "strict_errors": strict_errors})
    elapsed = time.perf_counter() - started
    for group, counts in groups.items():
        counts["command_candidates"] = sum(frequencies[group].values())
        counts["top_40_command_candidates"] = ranked(frequencies[group], 40)
        counts["feature_file_counts"] = {
            name: len(feature_paths[f"{group}/{name}"]) for name in FEATURES
        }
    report = {
        "format_version": 1,
        "compiler": {"version": schema["compiler_version"],
                     "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()},
        "sources": manifest(),
        "selection_limit": args.limit,
        "selection": selection,
        "groups": groups,
        "negative_tests": negative_cases(binary, args.cache),
        "focus_subsets": focus_subsets(inventory),
        "semantic_regression": {"status": "not_run", "reason": "No LAMMPS engine was invoked"},
        "validity": "Upstream inputs and fragments; runtime validity was not established",
    }
    write_json(args.output, report)
    if args.inventory:
        write_json(args.inventory, {"files": inventory, "feature_paths": feature_paths,
                                    "command_candidates": {k: ranked(v) for k, v in frequencies.items()}})
    print(f"Evaluated {len(corpus)} files in {elapsed:.3f} s ({elapsed / len(corpus):.6f} s/file)",
          file=sys.stderr)
    print(json.dumps({key: {k: v for k, v in value.items() if isinstance(v, int)}
                      for key, value in groups.items()}, sort_keys=True))
    return int(not all(case["passed"] for case in report["negative_tests"]["cases"])
               or any(group["round_trip_identical"] != group["files"]
                   or group["static_rejected"] or group["strict_rejected"]
                   for group in groups.values()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("fetch", "audit"))
    parser.add_argument("--cache", type=Path, default=ROOT / "target/corpus/sources")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lamlet")
    parser.add_argument("--output", type=Path, default=ROOT / "target/corpus/report.json")
    parser.add_argument("--inventory", type=Path)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if args.limit is not None and args.limit < 1:
        parser.error("--limit must be positive")
    try:
        if args.action == "fetch":
            fetch(args.cache.resolve())
            return 0
        return audit(args)
    except subprocess.CalledProcessError as error:
        print(f"corpus: {error}\n{error.stderr.decode(errors='replace') if error.stderr else ''}",
              file=sys.stderr)
        return 2
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"corpus: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
