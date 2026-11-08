#!/usr/bin/env python3
"""Independent P3 protocol checks using a bounded, instrumented LAMMPS run."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shlex
import sys
import tempfile


VERSION = "p3-protocol-v1"
POTENTIAL_SHA256 = "d02816ad43e6d9a0f5e76656b95567678683d467a30f75f9d67b602cb86a7c7a"
CRITERIA = ("structure", "potential", "groups", "boundaries", "gap", "equilibration",
            "impact_velocity", "thermal_velocity_retained", "adiabatic_integration")
ALLOWED = {"units", "dimension", "atom_style", "boundary", "variable", "lattice",
           "region", "create_box", "create_atoms", "mass", "group", "pair_style",
           "pair_coeff", "neighbor", "neigh_modify", "velocity", "timestep", "fix",
           "unfix", "run", "thermo", "thermo_style", "displace_atoms"}


class Unsupported(ValueError):
    pass


def commands(text):
    """Accept a small straight-line subset before passing anything to LAMMPS."""
    result, pending = [], ""
    if len(text) > 100_000 or "__lp3" in text:
        raise Unsupported("Input exceeds the size limit or uses reserved observation names")
    for number, raw in enumerate(text.splitlines(), 1):
        lexer = shlex.shlex(raw, posix=True)
        lexer.whitespace_split = True
        lexer.commenters = "#"
        try:
            tokens = list(lexer)
        except ValueError as error:
            raise Unsupported(f"Line {number}: {error}") from error
        if not tokens:
            continue
        # Rebuild without comments, preserving quoted expressions as one token.
        continued = tokens[-1] == "&"
        pending += " " + " ".join(shlex.quote(t) if " " in t else t
                                   for t in (tokens[:-1] if continued else tokens))
        if continued:
            continue
        line = pending.strip()
        pending = ""
        name = line.split()[0]
        if name not in ALLOWED:
            raise Unsupported(f"Line {number}: unsupported command {name!r}")
        if "$(" in line:
            raise Unsupported("Immediate expressions are outside this pilot's static schedule subset")
        result.append(line)
    if pending or len(result) > 250:
        raise Unsupported("Unfinished continuation or too many commands")
    # Variable definitions cannot introduce commands, paths, or Python callbacks.
    for line in result:
        parts = shlex.split(line)
        if parts[0] == "variable" and (len(parts) < 4 or parts[2] != "equal"
                or not re.fullmatch(r"[A-Za-z0-9_.$(){}+*/^: \-]+", " ".join(parts[3:]))):
            raise Unsupported("Only arithmetic equal-style variables are supported")
        if parts[0] == "variable":
            expression = re.sub(r"\$\{[A-Za-z0-9_]+\}", "0", " ".join(parts[3:]))
            names = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", expression)
            if any(not name.startswith("v_") and name not in {"sqrt", "abs", "ceil", "floor", "min", "max", "PI", "e", "E"} for name in names):
                raise Unsupported("Variables must depend only on static arithmetic and other equal variables")
    return result


def numbers(values):
    try:
        result = [float(value) for value in values]
    except ValueError as error:
        raise Unsupported("Unresolved or unsupported numeric expression") from error
    if not all(math.isfinite(value) for value in result):
        raise Unsupported("Nonfinite numerical value")
    return result


def finding(ok, evidence):
    return {"status": "pass" if ok else "fail", "evidence": [evidence]}


def snapshot(engine, groups):
    n = engine.extract_setting("nlocal")
    if n > 100_000:
        raise Unsupported("More than 100000 atoms")
    ids = engine.extract_atom("id")
    types = engine.extract_atom("type")
    x, v = engine.extract_atom("x"), engine.extract_atom("v")
    rows = {ids[i]: (types[i], tuple(x[i][j] for j in range(3)),
                     tuple(v[i][j] for j in range(3))) for i in range(n)}
    members = {"all": set(rows)}
    for group in groups:
        engine.command(f"variable __lp3mask atom gmask({group})")
        mask = engine.extract_variable("__lp3mask", None, 1)
        members[group] = {ids[i] for i in range(n) if mask[i] > 0.5}
        engine.command("variable __lp3mask delete")
    lo, hi, xy, yz, xz, periodic, box_change = engine.extract_box()
    mass = engine.extract_atom("mass")
    return {"rows": rows, "members": members, "lengths": [hi[i] - lo[i] for i in range(3)],
            "periodic": list(periodic), "tilts": [xy, yz, xz], "mass": mass[1]}


def slabs(state):
    members = state["members"]
    small = {frozenset(ids) for ids in members.values() if len(ids) == 16000}
    large = {frozenset(ids) for ids in members.values() if len(ids) == 32000}
    pairs = [(p, t) for p in small for t in large if not p & t and p | t == members["all"]]
    return pairs[0] if len(pairs) == 1 else (None, None)


def center(state, ids, column):
    return [sum(state["rows"][i][column][axis] for i in ids) / len(ids) for axis in range(3)]


def geometry(state, projectile, target):
    rows = state["rows"]
    a = state["lengths"][0] / 20
    ok = (3.0 < a < 3.7 and abs(state["lengths"][1] / a - 20) < 1e-7
          and all(abs(t) < 1e-9 for t in state["tilts"])
          and abs(state["mass"] - 92.90638) < 0.01)
    for ids, cells in ((projectile, 20), (target, 40)):
        minima = [min(rows[i][1][j] for i in ids) for j in range(3)]
        sites = set()
        for i in ids:
            kind, xyz, _ = rows[i]
            scaled = [2 * (xyz[j] - minima[j]) / a for j in range(3)]
            site = tuple(round(q) for q in scaled)
            ok &= (kind == 1 and all(abs(q - k) < 1e-6 for q, k in zip(scaled, site))
                   and 0 <= site[0] < 40 and 0 <= site[1] < 40 and 0 <= site[2] < 2 * cells
                   and site[0] % 2 == site[1] % 2 == site[2] % 2)
            sites.add(site)
        ok &= len(sites) == len(ids)
    pz = [rows[i][1][2] for i in projectile]
    tz = [rows[i][1][2] for i in target]
    gap = max(min(pz) - max(tz), min(tz) - max(pz))
    return bool(ok), a, gap


def observe(lines, inputs, runtime, smoke_steps):
    if runtime:
        runtime = runtime.resolve()
        sys.path.insert(0, str(runtime / "python"))
    from lammps import lammps

    potential = inputs / "prompt3.potential"
    if hashlib.sha256(potential.read_bytes()).hexdigest() != POTENTIAL_SHA256:
        raise RuntimeError("Trusted potential does not match the pinned SHA-256")
    facts = {"groups": [], "runs": [], "velocity_changes": [], "potential_used": False}
    fixes = {}
    metal_units = False
    with lammps(cmdargs=["-screen", "none", "-log", "none", "-nocite"]) as engine:
        if not engine.has_style("pair", "eam/alloy"):
            raise RuntimeError("LAMMPS lacks the MANYBODY eam/alloy style")
        facts["engine_version"] = engine.version()
        for line in lines:
            expanded = engine.expand(line)
            p = shlex.split(expanded)
            name = p[0]
            if facts["runs"] and name in {"units", "dimension", "atom_style", "boundary", "lattice",
                    "region", "create_box", "create_atoms", "mass", "group", "pair_style", "pair_coeff", "displace_atoms", "variable"}:
                raise Unsupported("Model changes after dynamics begins are outside this pilot")
            if any("\n" in t or ";" in t for t in p):
                raise Unsupported("Expanded command contains control characters")
            if name == "units" and p != ["units", "metal"]:
                raise Unsupported("This pilot supports metal units only")
            if name == "units":
                metal_units = True
            if name in {"create_box", "run"} and not metal_units:
                raise Unsupported("Explicit metal units are required before creating the system")
            if name == "atom_style" and p != ["atom_style", "atomic"]:
                raise Unsupported("This pilot supports atomic atom style only")
            if name == "variable":
                pass  # Checked before opening the engine.
            elif name == "neighbor":
                if len(p) != 3 or p[2] != "bin" or numbers(p[1:2])[0] <= 0:
                    raise Unsupported("Only positive bin neighbor settings are supported")
            elif name == "neigh_modify":
                if len(p) % 2 != 1 or any(p[i] not in ("delay", "every", "check") for i in range(1, len(p), 2)):
                    raise Unsupported("Only delay, every, and check neighbor settings are supported")
            elif name == "lattice":
                if len(p) not in (3, 18) or p[1] != "bcc":
                    raise Unsupported("Only axis-aligned BCC lattices are supported")
                if not 3 < numbers(p[2:3])[0] < 3.7:
                    raise Unsupported("Lattice constant outside the pilot observation bounds")
                if len(p) > 3 and p[3:] != "orient x 1 0 0 orient y 0 1 0 orient z 0 0 1".split():
                    raise Unsupported("Unsupported lattice orientation")
            elif name == "region":
                if len(p) != 11 or p[2] != "block" or p[9] != "units" or p[10] not in ("box", "lattice"):
                    raise Unsupported("Only static block regions with explicit units are supported")
                bounds = numbers(p[3:9])
                volume = math.prod(abs(bounds[j + 1] - bounds[j]) for j in (0, 2, 4))
                if any(abs(v) > 1000 for v in bounds) or volume > (50000 if p[10] == "lattice" else 2e6):
                    raise Unsupported("Region exceeds pilot resource bounds")
            elif name == "create_box" and (len(p) != 3 or p[1] != "1"):
                raise Unsupported("Only one Nb atom type is supported")
            elif name == "create_atoms" and (len(p) != 4 or p[1:3] != ["1", "region"]):
                raise Unsupported("Only lattice creation in a region is supported")
            elif name == "group":
                if len(p) < 4 or not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", p[1]) or p[2] not in ("region", "union", "subtract", "intersect", "type"):
                    raise Unsupported("Unsupported group declaration")
                if p[1] not in facts["groups"]:
                    facts["groups"].append(p[1])
            elif name == "pair_style":
                if p != ["pair_style", "eam/alloy"]:
                    facts["potential_error"] = "P3 requires the pinned EAM/alloy model"
                    return facts
            elif name == "pair_coeff":
                accepted = {"prompt3.potential", "/app/inputs/prompt3.potential", str(potential)}
                if len(p) != 5 or p[1:3] != ["*", "*"] or p[3] not in accepted or p[4] != "Nb":
                    facts["potential_error"] = "Coefficient file or element mapping differs from the pinned Nb model"
                    return facts
                line = f'pair_coeff * * "{potential}" Nb'
                facts["potential_used"] = True
            elif name == "fix":
                if len(p) < 4 or p[3] not in ("nvt", "nve"):
                    raise Unsupported("Only nvt and nve fixes are supported")
                if p[3] == "nvt":
                    if len(p) != 8 or p[4] != "temp":
                        raise Unsupported("Unsupported NVT arguments")
                    numbers(p[5:8])
                elif len(p) != 4:
                    raise Unsupported("Unsupported NVE arguments")
                fixes[p[1]] = p[2:]
            elif name == "unfix":
                if len(p) != 2:
                    raise Unsupported("Unsupported unfix arguments")
                fixes.pop(p[1], None)
            elif name == "velocity":
                if len(p) < 4 or p[2] not in ("create", "set", "zero", "scale"):
                    raise Unsupported("Unsupported velocity operation")
                if facts["runs"]:
                    facts["velocity_changes"].append([snapshot(engine, facts["groups"]), None, len(facts["runs"])])
            elif name == "displace_atoms":
                if len(p) != 8 or p[2] != "move" or p[6:] != ["units", "box"]:
                    raise Unsupported("Only constant displacements in box units are supported")
                numbers(p[3:6])
            elif name == "run":
                if len(p) != 2 or not re.fullmatch(r"\d+", p[1]):
                    raise Unsupported("Only run N without dynamic options is supported")
                n = int(p[1])
                if not facts["runs"]:
                    facts["initial"] = snapshot(engine, facts["groups"])
                engine.command("variable __lp3dt equal dt")
                dt = engine.extract_variable("__lp3dt", None, 0)
                engine.command("variable __lp3dt delete")
                if not math.isfinite(dt) or dt <= 0:
                    raise Unsupported("Invalid timestep")
                engine.command(f"run {min(n, smoke_steps)}")
                facts["runs"].append({"steps": n, "dt_ps": dt, "fixes": list(fixes.values()),
                                      "smoke_steps": min(n, smoke_steps),
                                      "pe_eV": engine.get_thermo("pe")})
                continue
            engine.command(line)
            if name == "create_atoms" and engine.get_natoms() > 100000:
                raise Unsupported("More than 100000 atoms")
            if name == "velocity" and facts["runs"]:
                facts["velocity_changes"][-1][1] = snapshot(engine, facts["groups"])
        return facts


def evaluate(facts):
    result = {key: {"status": "unknown", "evidence": ["Required observation is unavailable"]}
              for key in CRITERIA}
    if "potential_error" in facts:
        result["potential"] = finding(False, facts["potential_error"])
        return result
    runs = facts["runs"]
    result["potential"] = finding(facts["potential_used"] and bool(runs)
                                  and all(math.isfinite(r["pe_eV"]) and r["pe_eV"] < 0 for r in runs),
                                  f"Pinned EAM/alloy Nb file; smoke-run potential energies: {[r['pe_eV'] for r in runs]} eV")
    if not runs:
        result["equilibration"] = finding(False, "No dynamics requested")
        return result
    state = facts["initial"]
    result["boundaries"] = finding(state["periodic"] == [1, 1, 0],
                                    f"LAMMPS periodic flags: {state['periodic']}")
    projectile, target = slabs(state)
    result["groups"] = finding(projectile is not None,
        "Require disjoint 16000-atom projectile and 32000-atom target groups covering all atoms")
    if projectile is not None:
        ok, a, gap = geometry(state, projectile, target)
        result["structure"] = finding(ok, f"BCC site occupancy, [100]/[010]/[001], cells 20x20x20 and 20x20x40; a={a:.9g} Angstrom")
        result["gap"] = finding(abs(gap - 15) < 1e-6,
                                  f"Initial outermost-plane separation = {gap:.12g} Angstrom; required 15 +/- 1e-6")
    else:
        result["structure"] = finding(False, "Expected slab populations and partition were not observed")
    eq = [r for r in runs if len(r["fixes"]) == 1 and r["fixes"][0][:2] == ["all", "nvt"]]
    eq_time = sum(r["steps"] * r["dt_ps"] for r in eq)
    eq_ok = (bool(eq) and all(numbers(r["fixes"][0][3:5]) == [300, 300] for r in eq)
             and abs(eq_time - 100) < 1e-8 and runs[0] in eq)
    result["equilibration"] = finding(eq_ok,
        f"Requested all-atom NVT schedule at 300 K: {eq_time:.12g} ps; full equilibration not executed")
    impact = [r for r in runs if r not in eq]
    changes = facts["velocity_changes"]
    result["adiabatic_integration"] = finding(bool(impact) and all(
        r["steps"] > 0 and r["fixes"] == [["all", "nve"]] for r in impact)
        and runs[-1] in impact and len(changes) == 1 and changes[0][2] == len(eq)
        and runs[:len(eq)] == eq,
        f"Requested impact integration fixes: {[r['fixes'] for r in impact]}; smoke steps only")
    if projectile is not None and len(changes) == 1:
        before, after, _ = changes[0]
        if set(before["rows"]) != set(after["rows"]):
            return result
        dp = [center(after, projectile, 2)[j] - center(before, projectile, 2)[j] for j in range(3)]
        dt = [center(after, target, 2)[j] - center(before, target, 2)[j] for j in range(3)]
        direction = 1 if center(before, target, 1)[2] > center(before, projectile, 1)[2] else -1
        expected = [0, 0, direction * 20]
        result["impact_velocity"] = finding(all(abs(dp[j] - dt[j] - expected[j]) < 1e-7 for j in range(3)),
            f"Relative COM velocity increment = {dp[0]-dt[0]:.9g}, {dp[1]-dt[1]:.9g}, {dp[2]-dt[2]:.9g} Angstrom/ps; required {expected}")
        error = max(abs(after["rows"][i][2][j] - before["rows"][i][2][j]
                        - (expected[j] if i in projectile else 0)) for i in before["rows"] for j in range(3))
        mean = center(before, projectile, 2)
        variance = sum((before["rows"][i][2][j] - mean[j]) ** 2 for i in projectile for j in range(3)) / len(projectile)
        result["thermal_velocity_retained"] = finding(error < 1e-7 and variance > 0.01,
            f"Maximum per-atom error from uniform projectile velocity addition = {error:.9g}; pre-impact thermal variance = {variance:.9g}")
    elif not changes:
        result["impact_velocity"] = finding(False, "No post-equilibration velocity assignment")
        result["thermal_velocity_retained"] = finding(False, "No observed impact velocity addition")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", choices=["P3"], required=True)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--inputs-dir", type=Path, required=True)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--smoke-steps", type=int, choices=range(1, 11), default=2)
    args = parser.parse_args()
    if args.runtime:
        args.runtime = args.runtime.resolve()
    data = args.input.read_bytes()
    payload = {"schema_version": 1, "task": args.task,
               "submission_sha256": hashlib.sha256(data).hexdigest(), "oracle_version": VERSION,
               "execution": {"mode": "instrumented_smoke", "full_equilibration_executed": False}}
    try:
        lines = commands(data.decode("utf-8"))
        inputs = args.inputs_dir.resolve()
        with tempfile.TemporaryDirectory(prefix="lamlet-p3-") as temporary:
            previous = Path.cwd()
            os.chdir(temporary)
            try:
                facts = observe(lines, inputs, args.runtime, args.smoke_steps)
            finally:
                os.chdir(previous)
        payload["criteria"] = evaluate(facts)
        payload["execution"].update(engine_version=facts["engine_version"], runs=facts["runs"])
    except Unsupported as error:
        payload["criteria"] = {key: {"status": "unknown", "evidence": [str(error)]} for key in CRITERIA}
    except (OSError, ImportError, RuntimeError) as error:
        print(f"P3 oracle infrastructure failure: {error}", file=sys.stderr)
        return 2
    print(json.dumps(payload, sort_keys=True, allow_nan=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
