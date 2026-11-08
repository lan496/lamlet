#!/usr/bin/env python3
"""Run the real CLI, verify the demo, and render an offline presentation."""

import argparse
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "demo"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run_demo(binary, output):
    workspace = output / "workspace"
    workspace.mkdir(parents=True, exist_ok=True)
    for name in ["units.lamlet", "wrong-unit.lamlet", "missing-state.lamlet",
                 "workflow.lamlet", "structure.data", "lamlet.toml"]:
        shutil.copyfile(FIXTURES / name, workspace / name)
    shutil.copyfile(ROOT / "examples/legacy.in", workspace / "legacy.in")
    steps = []
    simulation_outputs = [workspace / f".lamlet/build/stages/{instance}/trajectory.lammpstrj"
                          for instance in ["cold", "warm"]]
    previous_outputs = {path: path.read_bytes() if path.is_file() else None
                        for path in simulation_outputs}

    def invoke(arguments, expected=0):
        process = subprocess.run([str(binary), *arguments], cwd=workspace,
                                 capture_output=True, text=True, encoding="utf-8", timeout=30)
        require(process.returncode == expected,
                f"Unexpected exit {process.returncode}: {shlex.join(arguments)}\n"
                f"{process.stdout}\n{process.stderr}")
        require(not process.stderr, "JSON mode unexpectedly wrote to stderr")
        value = json.loads(process.stdout)
        require(value["schema_version"] == 1, "Unexpected JSON schema version")
        return {"command": "lamlet " + shlex.join(arguments),
                "exit_code": process.returncode, "report": value}

    def step(key, title, takeaway, source, commands, evidence, result, generated=None):
        record = {"id": key, "title": title, "takeaway": takeaway,
                  "source": source, "commands": commands, "evidence": evidence,
                  "result": result, "generated": generated}
        steps.append(record)
        print(f"PASS  {len(steps)}. {title}: {result}", flush=True)
        return record

    def read(name):
        return (workspace / name).read_text(encoding="utf-8")

    legacy = invoke(["build", "legacy.in", "-o", "legacy.generated.in", "--format=json"])
    original = (workspace / "legacy.in").read_bytes()
    require(original == (workspace / "legacy.generated.in").read_bytes(),
            "Legacy output differs from input")
    step("legacy", "Start with the script you have",
         "Adoption starts with an unchanged LAMMPS file.", read("legacy.in"), [legacy],
         {"byte_identical": True, "input_bytes": len(original)},
         "Generated output is byte-for-byte identical.", read("legacy.generated.in"))

    units = invoke(["build", "units.lamlet", "-o", "units.generated.in",
                    "--strict", "--format=json"])
    emitted = read("units.generated.in")
    require(emitted == "units metal\ntimestep 0.002\n    run 0\n    run 100\n",
            "Unit conversion or loop expansion changed")
    step("units", "Put the unit next to the value",
         "Write 2 fs. Let the compiler emit the value for metal units.",
         read("units.lamlet"), [units], {"time": "2 fs", "metal_timestep": 0.002},
         "2 fs becomes 0.002 ps; the finite loop expands to two commands.", emitted)

    bad_args = ["check", "wrong-unit.lamlet", "--format=json"]
    bad = invoke(bad_args, 1)
    require(bad == invoke(bad_args, 1), "Diagnostics are not deterministic")
    diagnostic = next((d for d in bad["report"]["diagnostics"]
                       if d["code"] == "E_DIMENSION_MISMATCH"), None)
    require(diagnostic is not None, "Expected the dimensional error")
    protected = workspace / "failed-build.in"
    protected.write_text("keep the previous build\n", encoding="utf-8")
    failed = invoke(["build", "wrong-unit.lamlet", "-o", "failed-build.in", "--format=json"], 1)
    require(protected.read_text(encoding="utf-8") == "keep the previous build\n",
            "Failed compilation replaced an existing build")
    repaired_source = read("wrong-unit.lamlet").replace("300 K", "2 fs")
    require(repaired_source != read("wrong-unit.lamlet"), "Expected a repairable temperature quantity")
    (workspace / "repaired-unit.lamlet").write_text(repaired_source, encoding="utf-8")
    repaired = invoke(["build", "repaired-unit.lamlet", "-o", "repaired-unit.in", "--format=json"])
    repaired_output = read("repaired-unit.in")
    require(repaired_output == "units metal\ntimestep 0.002\n", "Repaired input did not compile as expected")
    diagnostic_scene = step("diagnostic", "Catch the mistake before launching LAMMPS",
                           "A temperature cannot be used as a timestep. Agents get a code and a source span.",
                           read("wrong-unit.lamlet"), [bad, failed], diagnostic,
                           "Expected rejection, exit 1. Repaired input compiles; the previous build survives.")
    diagnostic_scene["repair"] = {"source": repaired_source, "commands": [repaired],
                                  "generated": repaired_output}

    def bindings(instance, temperature):
        return ["workflow.lamlet", "--stage", "sample", "--instance", instance,
                "--arg", 'structure=path("structure.data")',
                "--arg", 'trajectory=output("trajectory.lammpstrj")',
                "--arg", f"temperature={temperature} K", "--strict", "--format=json"]

    inspected = invoke(["inspect", "stages", "workflow.lamlet", "--format=json"])
    cold = invoke(["build", *bindings("cold", 85)])
    cold_path = workspace / ".lamlet/build/stages/cold/simulation.in"
    cold_text = cold_path.read_text(encoding="utf-8")
    cold_plan = cold["report"]["stage"]
    require(cold_plan["independent_execution"] and not cold_plan["resumable"],
            "Unexpected stage guarantee")
    require(cold_text.endswith("undump traj\nunfix thermostat\n"),
            "Expected reverse resource cleanup")
    require("timestep 0.002\n" in cold_text and "temp 85 85 0.1\n" in cold_text,
            "Stage unit conversion changed")
    require('read_data "../../../../structure.data" nocoeff' in cold_text,
            "Expected an explicitly resolved input")
    step("stage", "Make one calculation an explicit task",
         "Reuse a checked module. Bind files and parameters. Release owned resources at the end.",
         read("workflow.lamlet"), [inspected, cold], cold_plan,
         "The cold instance builds with explicit inputs and reverse resource cleanup.", cold_text)

    warm = invoke(["build", *bindings("warm", 110)])
    warm_path = workspace / ".lamlet/build/stages/warm/simulation.in"
    dependencies = invoke(["inspect", "dependencies", *bindings("warm", 110)])
    manifest = json.loads(warm_path.with_name("simulation.in.manifest.json").read_text(encoding="utf-8"))
    require(warm["report"]["stage"]["working_directory"] != cold_plan["working_directory"],
            "Stage instances share a working directory")
    require("temp 110 110 0.1\n" in warm_path.read_text(encoding="utf-8"), "Warm parameter was not bound")
    require(cold_path.read_text(encoding="utf-8") == cold_text, "Building warm changed the cold script")
    resource_report = dependencies["report"]["dependencies"]
    require(resource_report["complete"], "Expected declared stage dependencies")
    require(any(item["path"] == ".lamlet/build/stages/warm/trajectory.lammpstrj"
                for item in resource_report["outputs"]), "Expected a stage-owned output")
    require(all((path.read_bytes() if path.is_file() else None) == previous_outputs[path]
                for path in simulation_outputs), "The compiler changed simulation outputs")
    step("resources", "Change the conditions, keep the outputs separate",
         "Two instances use the same definition and input, with different parameters and output directories.",
         'structure=path("structure.data")\ntrajectory=output("trajectory.lammpstrj")\ntemperature=110 K',
         [warm, dependencies], {"cold": cold_plan["working_directory"],
                                  "warm": manifest["stage"]["working_directory"],
                                  "dependencies": resource_report},
         "Cold and warm scripts are separate. The manifest records each task; no MD was run.")

    state = invoke(["check", "missing-state.lamlet", "--stage", "production", "--format=json"], 1)
    state_diagnostic = next((d for d in state["report"]["diagnostics"]
                             if d["code"] == "E_STATE_CONTRACT"), None)
    require(state_diagnostic is not None, "Expected a missing-state contract error")
    step("state", "Do not inherit an invisible simulation",
         "A stage starts with empty simulation state. A previous script cannot supply its atoms or model.",
         read("missing-state.lamlet"), [state], state_diagnostic,
         "Expected rejection, exit 1. Required simulation state was not established.")

    baseline = json.loads((ROOT / "tests/corpus/baseline.json").read_text(encoding="utf-8"))
    version = subprocess.run([str(binary), "--version"], capture_output=True,
                             text=True, encoding="utf-8", check=True, timeout=10).stdout.strip()
    report = {"schema_version": 1, "compiler": {"version": version,
              "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()},
              "steps": steps, "simulation_executed": False,
              "compatibility_baseline": {"source": "tests/corpus/baseline.json",
                  "compiler": baseline["compiler"], "groups": baseline["groups"]}}
    output.mkdir(parents=True, exist_ok=True)
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    # Escape HTML delimiters before embedding data in a script element.
    embedded = json.dumps(report).replace("<", "\\u003c").replace(">", "\\u003e").replace("&", "\\u0026")
    template = (FIXTURES / "presentation.html").read_text(encoding="utf-8")
    require(template.count("__LAMLET_DEMO_DATA__") == 1, "Expected one presentation data placeholder")
    (output / "index.html").write_text(template.replace("__LAMLET_DEMO_DATA__", embedded), encoding="utf-8")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path,
                        default=ROOT / "target/release" / ("lamlet.exe" if sys.platform == "win32" else "lamlet"))
    parser.add_argument("--output", type=Path, default=ROOT / "target/demo")
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    if not output.is_relative_to((ROOT / "target").resolve()) or output == ROOT / "target":
        parser.error("Demo output must be a directory inside this repository's target/")
    if not binary.is_file():
        parser.error("Build the CLI first: cargo build --release --locked")
    try:
        report = run_demo(binary, output)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Demo failed: {error}", file=sys.stderr)
        return 1
    print(f"\n{len(report['steps'])} verified scenes. No LAMMPS process was started.")
    print(f"Presentation: {output / 'index.html'}")
    print(f"Evidence:     {output / 'report.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
