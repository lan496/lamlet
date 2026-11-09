#!/usr/bin/env python3
"""Generate and independently grade narrowly specified LAMMPS repair tasks."""

import argparse
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import tempfile


BASE = """units lj
atom_style atomic
boundary p p p
lattice fcc 0.83
region cell block 0 4 0 4 0 4 units lattice
create_box 1 cell
create_atoms 1 box
mass 1 1
pair_style lj/cut 2.5
pair_coeff * * 1 1 2.5
neighbor 0.3 bin
neigh_modify delay 0 every 1 check yes
velocity all create 0.85 1938472 mom yes rot yes dist gaussian
timestep 0.003
thermo 10
thermo_style custom step atoms temp pe etotal press
"""


def tasks():
    """The reserve split is not used for development or model trials."""
    result = {}
    for split, suffix, steps, stride, repeat, frequency, x, z, bad_y, good_y in [
        ("development", "dev", 20, 4, 7, 26, "1 1 0", "0 0 1", "0 1 0", "-1 1 0"),
        ("held_out", "reserve", 30, 5, 6, 32, "1 0 1", "0 1 0", "0 0 1", "1 0 -1"),
    ]:
        thermal = f"fix thermal all nvt temp 0.85 0.85 0.3\nrun {steps}\n"
        ballistic = f"fix ballistic all nve\nrun {steps * 2}\n"
        result[f"integrator-{suffix}"] = {
            "split": split, "kind": "integrator", "rule": "LML-PHYS-001",
            "instruction": "Repair the transition from NVT equilibration to NVE production. "
                "Every atom must be integrated exactly once in both phases. Keep every existing "
                "command, ID, parameter, and run length. You may insert unfix commands only. "
                "The first phase must remain NVT and the second must be NVE.",
            "input": BASE + thermal + ballistic,
            "reference": BASE + thermal + "unfix thermal\n" + ballistic,
        }
        start = BASE + "fix integration all nve\ncompute kinetic all ke\n"
        broken = f"fix stats all ave/time {stride} {repeat} {frequency} c_kinetic file averages.dat\n"
        fixed = f"fix stats all ave/time {stride} {repeat} {stride * repeat} c_kinetic file averages.dat\n"
        result[f"sampling-{suffix}"] = {
            "split": split, "kind": "sampling", "rule": "LML-MAN-AVE-TIME",
            "instruction": f"Repair time averaging. Retain {repeat} samples spaced {stride} steps "
                "apart and report once at the end of each nonoverlapping sampling window. "
                "Change only the output-frequency argument of the stats fix. Keep every other "
                "command and parameter, including the run length and output file.",
            "input": start + broken + "run 120\n",
            "reference": start + fixed + "run 120\n",
        }
        lattice = f"lattice fcc 0.83 orient x {x} orient y {{y}} orient z {z}\n"
        tail = "fix integration all nve\nrun 40\n"
        result[f"orientation-{suffix}"] = {
            "split": split, "kind": "orientation", "rule": "LML-MAN-LATTICE-ORIENTATION",
            "instruction": "Repair the crystal orientation. Keep the specified x and z "
                "directions and change only the three integer components of orient y. "
                "The directions must be nonzero, mutually orthogonal, and right handed. "
                "Preserve every other command, physical parameter, and run length.",
            "input": BASE.replace("lattice fcc 0.83\n", lattice.format(y=bad_y)) + tail,
            "reference": BASE.replace("lattice fcc 0.83\n", lattice.format(y=good_y)) + tail,
        }
    return result


def sha(data):
    return hashlib.sha256(data).hexdigest()


def commands(text):
    if any(marker in text for marker in ("$", "&", "\x00")):
        raise ValueError("Dynamic expressions and continuations are outside this repair contract")
    return [words for line in text.splitlines()
            if (words := shlex.split(line, comments=True))]


def contract(task, candidate):
    """Check edit restrictions and physical arithmetic without importing Lamlet."""
    original = commands(task["input"])
    current = commands(candidate)
    if task["kind"] == "integrator":
        stripped = [c for c in current if c[0] != "unfix"]
        if stripped != original:
            return False, "Commands other than inserted unfix operations changed"
        active = {}
        for command in current:
            if command[0] == "fix":
                active[command[1]] = command[3]
            elif command[0] == "unfix":
                if command[1:] not in (["thermal"], ["ballistic"]) or command[1] not in active:
                    return False, "Invalid or duplicate release"
                del active[command[1]]
            elif command[0] == "run":
                required = "nvt" if command[1] == str(task_run_lengths(task)[0]) else "nve"
                if list(active.values()) != [required]:
                    return False, "Each phase requires exactly its intended integrator"
        return True, "One intended integrator in each phase"
    if len(current) != len(original):
        return False, "Command inventory changed"
    if task["kind"] == "sampling":
        for expected, actual in zip(original, current):
            if expected[:2] != ["fix", "stats"]:
                if actual != expected:
                    return False, "A command outside the averaging fix changed"
                continue
            if len(actual) != len(expected) or actual[:6] + actual[7:] != expected[:6] + expected[7:]:
                return False, "Only the output frequency may change"
            stride, repeat, frequency = map(int, actual[4:7])
            if frequency != stride * repeat:
                return False, "The output frequency does not equal the sampling window"
        return True, "Averaging windows are contiguous and nonoverlapping"
    for expected, actual in zip(original, current):
        if expected[0] != "lattice":
            if actual != expected:
                return False, "A command outside lattice orientation changed"
            continue
        # Fixed positions belong to the explicitly restricted task grammar, not a LAMMPS parser.
        if len(actual) != len(expected) or actual[:10] + actual[13:] != expected[:10] + expected[13:]:
            return False, "Only the y direction may change"
        x, y, z = (tuple(map(int, actual[start:start + 3])) for start in (5, 10, 15))
        dot = lambda a, b: sum(i * j for i, j in zip(a, b))
        cross = (x[1]*y[2]-x[2]*y[1], x[2]*y[0]-x[0]*y[2], x[0]*y[1]-x[1]*y[0])
        if not all(any(v) for v in (x, y, z)) or any((dot(x, y), dot(y, z), dot(z, x))):
            return False, "Directions must be nonzero and orthogonal"
        if dot(cross, z) <= 0:
            return False, "Directions must be right handed"
    return True, "Orthogonal right-handed integer orientation"


def task_run_lengths(task):
    return [int(c[1]) for c in commands(task["input"]) if c[0] == "run"]


def assess(task, candidate, engine):
    record = {"submission_sha256": sha(candidate.encode()), "status": "unassessed",
              "engine_launches": 0, "full_scientific_validation": False}
    try:
        valid, reason = contract(task, candidate)
    except (ValueError, IndexError) as error:
        record["reason"] = "Outside the restricted static contract: " + str(error)
        return record
    record["reason"] = reason
    if not valid:
        record["status"] = "incorrect"
        return record
    with tempfile.TemporaryDirectory() as temporary:
        source = Path(temporary) / "solution.in"
        source.write_text(candidate)
        try:
            process = subprocess.run([str(engine), "-log", "none", "-in", str(source)],
                cwd=temporary, capture_output=True, text=True, timeout=20)
        except (OSError, subprocess.SubprocessError) as error:
            record["reason"] = "Engine infrastructure failure: " + str(error)
            return record
        record.update(engine_launches=1, engine_exit=process.returncode,
                      engine_stdout=process.stdout, engine_stderr=process.stderr)
        record["status"] = "correct" if process.returncode == 0 else "incorrect"
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", required=True, choices=tasks())
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--engine", type=Path, default=Path("/opt/lammps/bin/lmp"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.source.is_file():
        report = assess(tasks()[args.task], args.source.read_text(), args.engine)
    else:
        report = {"status": "unassessed", "reason": "No submitted input", "engine_launches": 0}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"status": report["status"]}))
    if report["status"] != "unassessed":
        (args.output.parent / "reward.txt").write_text("1\n" if report["status"] == "correct" else "0\n")


if __name__ == "__main__":
    main()
