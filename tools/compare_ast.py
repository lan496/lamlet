#!/usr/bin/env python3
"""Compare pinned LAMMPS-AST and Lamlet on the same verified input bytes.

Run with LAMMPS-AST's pinned requirements installed. No MD or agents run.
"""

import argparse
from contextlib import redirect_stdout
import gzip
import hashlib
import importlib.metadata
import io
import json
import os
from pathlib import Path
import platform
import selectors
import statistics
import subprocess
import sys
import time

from corpus import ROOT, git, inputs, write_json
from evaluation import source as upstream_source, verified_inputs


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ast_worker(checkout):
    sys.path.insert(0, str(checkout))
    started = time.perf_counter()
    from lammps_ast.parser import parse_to_AST
    from lammps_ast.sanitizer import sanitize
    # Force grammar construction before accepting timed requests.
    parse_to_AST("run 0\n", lint=True)
    print(json.dumps({"ready": True, "init_seconds": time.perf_counter() - started}), flush=True)
    for line in sys.stdin:
        request = json.loads(line)
        raw = Path(request["path"]).read_text(encoding="utf-8")
        samples = []
        for _ in range(request["repeats"]):
            started = time.perf_counter()
            try:
                with redirect_stdout(io.StringIO()):
                    cleaned = sanitize(raw)
                    tree, errors = parse_to_AST(cleaned, lint=True)
                result = {"accepted": tree is not None and not errors,
                          "errors": len(errors), "sanitized_bytes": len(cleaned.encode()),
                          "diagnostics": [vars(error) for error in errors]}
            except Exception as error:
                result = {"accepted": False, "exception": type(error).__name__, "message": str(error)}
            samples.append(time.perf_counter() - started)
        print(json.dumps({**result, "seconds": samples}), flush=True)


class Worker:
    def __init__(self, command):
        self.command = command
        self.process = None
        self.start()

    def start(self):
        self.process = subprocess.Popen(self.command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, text=True, bufsize=1)
        self.ready = self.receive(60)

    def receive(self, timeout):
        with selectors.DefaultSelector() as selector:
            selector.register(self.process.stdout, selectors.EVENT_READ)
            if not selector.select(timeout):
                self.close()
                raise TimeoutError(f"Worker exceeded {timeout} seconds")
        line = self.process.stdout.readline()
        if not line:
            details = self.process.stderr.read()
            raise ValueError(f"Worker failed: {details}")
        return json.loads(line)

    def request(self, path, repeats):
        if self.process is None:
            self.start()
        self.process.stdin.write(json.dumps({"path": str(path), "repeats": repeats}) + "\n")
        self.process.stdin.flush()
        try:
            return self.receive(10)
        except TimeoutError:
            return {"accepted": False, "timeout_seconds": 10}

    def close(self):
        if self.process is not None:
            if self.process.poll() is None:
                self.process.kill()
            self.process.communicate()
            self.process = None


def summarize(cases):
    paired = [c for c in cases if c["ast"].get("accepted") and not c["lamlet"]["summary"]["errors"]]
    ast = sum(statistics.median(c["ast"]["seconds"]) for c in paired)
    parse = sum(statistics.median(c["lamlet"]["parse_seconds"]) for c in paired)
    check = sum(statistics.median(c["lamlet"]["check_seconds"]) for c in paired)
    serial = sum(statistics.median(c["lamlet"]["serialize_seconds"]) for c in paired)
    result = {"files": len(cases), "ast_accepted": sum(c["ast"].get("accepted", False) for c in cases),
              "ast_timeouts": sum("timeout_seconds" in c["ast"] for c in cases),
              "lamlet_accepted": sum(not c["lamlet"]["summary"]["errors"] for c in cases),
              "paired_accepted_files": len(paired),
              "paired_ast_seconds": ast, "paired_lamlet_parse_seconds": parse,
              "paired_lamlet_check_seconds": check, "paired_lamlet_json_seconds": serial,
              "ast_over_lamlet_check_json": ast / (check + serial) if check + serial else None,
              "scientific_correctness": None}
    if cases and all("before" in c for c in cases):
        before = sum(statistics.median(c["before"]["check_seconds"]) for c in cases)
        after = sum(statistics.median(c["lamlet"]["check_seconds"]) for c in cases)
        result.update({"before_check_seconds": before, "after_check_seconds": after,
                       "before_over_after": before / after if after else None,
                       "identical_reports": sum(c["before"]["report_sha256"] == c["lamlet"]["report_sha256"] for c in cases)})
    return result


def lamlet_result(result):
    report = result.pop("report_json")
    return {**result, "summary": json.loads(report)["summary"],
            "report_sha256": hashlib.sha256(report.encode()).hexdigest()}


def compare(args):
    specification = upstream_source()
    if git(args.ast, "rev-parse", "HEAD").decode().strip() != specification["commit"]:
        raise ValueError("LAMMPS-AST revision does not match research/holbrook/source.json")
    # Verify all checked-out Python/grammar/skill sources against the pinned tree.
    verified = {}
    for entry in git(args.ast, "ls-tree", "-rz", "HEAD").split(b"\0"):
        if not entry:
            continue
        metadata, path = entry.split(b"\t", 1)
        name = path.decode()
        if not (name.startswith("lammps_ast/") or name == "lammps-ast-eval/scripts/sanitize_and_parse.py"):
            continue
        data = (args.ast / name).read_bytes()
        blob = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        if blob != metadata.decode().split()[2]:
            raise ValueError(f"Modified LAMMPS-AST source: {name}")
        verified[name] = hashlib.sha256(data).hexdigest()
    corpus = []
    if args.dataset in ("corpus", "both"):
        corpus, _ = inputs(args.cache)
    if args.dataset in ("published", "both"):
        for case in verified_inputs(args.ast, specification):
            path = args.ast / case["path"]
            corpus.append((case["group"] + "/" + case["task"], "holbrook", case["path"], path, path.read_bytes()))
    if args.case:
        corpus = [c for c in corpus if f"{c[1]}/{c[2]}" == args.case]
    elif args.limit:
        corpus = corpus[:args.limit]
    if not corpus:
        raise ValueError("No benchmark inputs")
    workers = {}
    cases = []
    started = time.perf_counter()
    try:
        workers["ast"] = Worker([sys.executable, str(Path(__file__).resolve()), "--worker", "--ast", str(args.ast)])
        workers["lamlet"] = Worker([str(args.lamlet_worker)])
        if args.before_worker:
            workers["before"] = Worker([str(args.before_worker)])
        for index, (group, source, name, path, data) in enumerate(corpus):
            case = {"id": f"{source}/{name}", "group": group, "bytes": len(data),
                    "sha256": hashlib.sha256(data).hexdigest()}
            # Rotate order so one implementation does not always run first.
            order = list(workers)
            for key in order[index % len(order):] + order[:index % len(order)]:
                result = workers[key].request(path.resolve(), args.repeats)
                if key != "ast":
                    result = lamlet_result(result)
                case[key] = result
            cases.append(case)
            if (index + 1) % 100 == 0:
                print(f"Compared {index + 1}/{len(corpus)}", file=sys.stderr, flush=True)
        cold = []
        if not args.limit and not args.case:
            # One median-size file per source group, among jointly accepted inputs.
            common = {c["id"] for c in cases if c["ast"].get("accepted") and not c["lamlet"]["summary"]["errors"]}
            for group in sorted({item[0] for item in corpus}):
                candidates = sorted((i for i in corpus if i[0] == group and f"{i[1]}/{i[2]}" in common),
                                    key=lambda i: (len(i[4]), i[2]))
                if not candidates:
                    continue
                item = candidates[len(candidates) // 2]
                path = item[3].resolve()
                commands = {
                    "ast": [sys.executable, str(args.ast / "lammps-ast-eval/scripts/sanitize_and_parse.py"), str(path)],
                    "lamlet": [str(args.binary), "check", str(path), "--language", "lammps", "--format", "json"],
                }
                if args.before_binary:
                    commands["before"] = [str(args.before_binary), *commands["lamlet"][1:]]
                environment = {**os.environ, "PYTHONPATH": str(args.ast)}
                samples = {key: [] for key in commands}
                for iteration in range(args.repeats):
                    order = list(commands)
                    for key in order[iteration % len(order):] + order[:iteration % len(order)]:
                        tick = time.perf_counter()
                        process = subprocess.run(commands[key], env=environment, capture_output=True, timeout=30)
                        elapsed = time.perf_counter() - tick
                        if process.returncode:
                            raise ValueError(f"Cold CLI failed: {key}: {process.stderr!r}")
                        result = json.loads(process.stdout)
                        if key == "ast" and not result["parsed"] or key != "ast" and result["summary"]["errors"]:
                            raise ValueError(f"CLI/API acceptance disagreement: {key}")
                        samples[key].append(elapsed)
                cold.append({"id": f"{item[1]}/{item[2]}", "group": group, "seconds": samples})
        report = {"format_version": 1, "experiment": "static_processing_comparison",
                  "environment": {"system": platform.system(), "release": platform.release(),
                                  "machine": platform.machine(), "python": platform.python_version(),
                                  "dependencies": {p: importlib.metadata.version(p) for p in
                                                   ("lark-parser", "colorama", "simpleeval", "graphviz", "pydot", "zss", "pyparsing", "six")}},
                  "ast": {"repository": specification["repository"], "commit": specification["commit"], "files": verified},
                  "binaries": {key: digest(Path(worker.command[0])) for key, worker in workers.items() if key != "ast"},
                  "cli_sha256": digest(args.binary), "repeats": args.repeats, "limit": args.limit, "case": args.case,
                  "before_cli_sha256": digest(args.before_binary) if args.before_binary else None,
                  "benchmark_sources": {name: digest(ROOT / name) for name in
                                        ("tools/compare_ast.py", "crates/lamlet/examples/benchmark.rs",
                                         "crates/lamlet/src/manual_lint.rs", "Cargo.lock",
                                         "tests/corpus/sources.json", "research/holbrook/source.json")},
                  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
                  "dataset": args.dataset,
                  "initialization": {key: worker.ready for key, worker in workers.items()},
                  "summary": summarize(cases), "cases": cases, "cold_cli": cold,
                  "by_group": {group: summarize([c for c in cases if c["group"] == group])
                               for group in sorted({c["group"] for c in cases})},
                  "wall_seconds": time.perf_counter() - started, "lammps_launches": 0, "agent_trials": 0}
        if args.output.suffix == ".gz":
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_bytes(gzip.compress(
                (json.dumps(report, indent=2, sort_keys=True) + "\n").encode(), mtime=0))
        else:
            write_json(args.output, report)
        print(json.dumps(report["summary"], indent=2))
        if "before" in workers and report["summary"]["identical_reports"] != len(cases):
            raise ValueError("Optimization changed diagnostic JSON")
    finally:
        for worker in workers.values():
            worker.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ast", type=Path, default=ROOT / "target/evaluation/upstream")
    parser.add_argument("--cache", type=Path, default=ROOT / "target/corpus/sources")
    parser.add_argument("--lamlet-worker", type=Path, default=ROOT / "target/release/examples/benchmark")
    parser.add_argument("--before-worker", type=Path)
    parser.add_argument("--before-binary", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lamlet")
    parser.add_argument("--output", type=Path, default=ROOT / "target/performance/comparison.json")
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--limit", type=int)
    selection.add_argument("--case", help="Exact source/path identity for a timing pilot")
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--dataset", choices=("corpus", "published", "both"), default="both")
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.repeats < 1 or args.limit is not None and args.limit < 1:
        parser.error("repeats and limit must be positive")
    try:
        ast_worker(args.ast) if args.worker else compare(args)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"compare_ast: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
