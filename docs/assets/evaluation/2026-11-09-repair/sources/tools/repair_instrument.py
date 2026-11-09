#!/usr/bin/env python3
"""Record candidates presented to validators; never supply correctness feedback."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time


def main():
    tool = Path(sys.argv[0]).name
    arguments = sys.argv[1:]
    real = {"lmp": "/opt/lammps/bin/lmp", "lamlet": "/opt/lamlet/lamlet"}[tool]
    source = None
    if tool == "lmp" and "-in" in arguments:
        source = Path(arguments[arguments.index("-in") + 1])
    elif tool == "lamlet" and len(arguments) > 1 and arguments[0] == "check":
        source = Path(arguments[1])
    root = Path("/app/submission")
    root.mkdir(exist_ok=True)
    record = {"tool": tool, "time_ns": time.time_ns(), "candidate_sha256": None}
    if source is not None and source.is_file():
        data = source.read_bytes()
        identity = hashlib.sha256(data).hexdigest()
        revisions = root / "validated-candidates"
        revisions.mkdir(exist_ok=True)
        (revisions / (identity + ".in")).write_bytes(data)
        record["candidate_sha256"] = identity
    with (root / "validation-events.jsonl").open("a") as stream:
        stream.write(json.dumps(record, sort_keys=True) + "\n")
    return subprocess.run([real, *arguments]).returncode


if __name__ == "__main__":
    sys.exit(main())
