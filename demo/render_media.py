#!/usr/bin/env python3
"""Render a walkthrough from demo/run.py evidence with rsvg-convert and FFmpeg."""

import argparse
import hashlib
import html
import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[1]


def label(x, y, text, size=22, color="#d7e5ed", family="sans-serif", weight="400"):
    return (f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" '
            f'font-family="{family}" font-weight="{weight}">{html.escape(text)}</text>')


def code(x, y, lines):
    return "".join(label(x, y + i * 32, line, 19, "#dcf5ed", "monospace")
                   for i, line in enumerate(lines))


def svg(body, width=1280, height=800):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}"><rect width="100%" height="100%" '
            f'fill="#0c1c2b"/>{body}</svg>')


def overview():
    body = label(48, 64, "LAMLET", 18, "#80e8c0", weight="700")
    body += label(48, 110, "Check the input. Emit standard LAMMPS.", 32, "#ffffff", weight="700")
    boxes = [(48, "YOUR INPUT", ["Existing LAMMPS scripts", "Typed quantities and paths", "Scopes and stages"]),
             (443, "LAMLET · RUST", ["Parse and preserve", "Check types and state", "Compile explicit extensions"]),
             (838, "YOUR OUTPUT", ["Standard LAMMPS scripts", "JSON diagnostics", "Dependency manifests"])]
    for x, title, lines in boxes:
        body += f'<rect x="{x}" y="155" width="350" height="190" rx="14" fill="#142f42" stroke="#2b5969"/>'
        body += label(x + 24, 192, title, 16, "#80e8c0", weight="700")
        for i, line in enumerate(lines):
            body += label(x + 24, 235 + 36 * i, line, 19)
    for x in (405, 800):
        body += f'<path d="M{x} 250h28m-8-7 8 7-8 7" fill="none" stroke="#80e8c0" stroke-width="3"/>'
    body += label(48, 399, "Keep legacy bytes. Add checks incrementally. Launch LAMMPS explicitly.", 21)
    return svg(body, 1236, 440)


def render(args):
    report = json.loads(args.report.read_text())
    scenes = {s["id"]: s for s in report["steps"]}
    required = {"legacy", "units", "diagnostic", "stage"}
    if not required <= scenes.keys() or report["simulation_executed"]:
        raise ValueError("Expected verified compiler-only demo evidence")
    units = scenes["units"]
    error = scenes["diagnostic"]["evidence"]
    stage_lines = scenes["stage"]["generated"].splitlines()
    cleanup = [line for line in stage_lines if line.startswith(("fix ", "dump ", "undump ", "unfix "))]
    if len(cleanup) != 4 or error["code"] != "E_DIMENSION_MISMATCH":
        raise ValueError("Demo evidence changed; review the media scenes")
    # Source excerpts retain exact lines. Long resource declarations are shown in
    # the full HTML demo; the video focuses on generated ownership operations.
    slides = [
        ("01 / PHYSICAL UNITS", "Write 2 fs. Compile to the active unit system.",
         "simulation.lamlet", units["source"].splitlines(),
         "Generated LAMMPS", units["generated"].splitlines(),
         units["commands"][0]["command"], "Time conversion checked before engine execution."),
        ("02 / PRECISE DIAGNOSTICS", "A temperature cannot be a timestep.",
         "wrong-unit.lamlet", scenes["diagnostic"]["source"].splitlines(),
         "JSON diagnostic · selected fields", ["{", '  "code": "' + error["code"] + '",',
          '  "expected": "' + error["expected"] + '",', '  "actual": "' + error["actual"] + '",',
          '  "severity": "error"', "}"],
         scenes["diagnostic"]["commands"][0]["command"], "Exit 1 · source span included · deterministic JSON."),
        ("03 / MANAGED RESOURCES", "Declare resources. Lamlet emits their cleanup.",
         "Stage output · creation", cleanup[:2], "Stage output · cleanup", cleanup[2:],
         "lamlet build workflow.lamlet --stage sample ...", "Reverse cleanup order: undump, then unfix. Command abbreviated."),
        ("04 / GRADUAL ADOPTION", "Start with the LAMMPS script you already have.",
         "Legacy input · excerpt", scenes["legacy"]["source"].splitlines()[1:9],
         "Verified result", ["Byte-for-byte identical output", "Unsupported commands stay unverified", "", "Check and build need no LAMMPS", "Run the engine only when requested"],
         scenes["legacy"]["commands"][0]["command"], "This walkthrough executes the compiler. No molecular dynamics is run."),
    ]
    args.work.mkdir(parents=True, exist_ok=True)
    args.assets.mkdir(parents=True, exist_ok=True)
    (args.assets / "overview.svg").write_text(overview())
    for i, (eyebrow, title, left_title, left, right_title, right, command, footer) in enumerate(slides):
        body = label(48, 58, "LAMLET", 24, "#80e8c0", weight="700")
        body += label(48, 112, eyebrow, 16, "#80e8c0", weight="700")
        body += label(48, 169, title, 34, "white", weight="700")
        for x, heading, lines in ((48, left_title, left), (658, right_title, right)):
            body += f'<rect x="{x}" y="210" width="574" height="366" rx="12" fill="#142c40" stroke="#315569"/>'
            body += label(x + 22, 247, heading, 18, "#91aec2")
            # Wrap only long resource commands for display, preserving tokens.
            wrapped = []
            for line in lines:
                while len(line) > 46:
                    split = line.rfind(" ", 0, 46)
                    if split <= 0:
                        split = 46
                    wrapped.append(line[:split])
                    line = "  " + line[split:].lstrip()
                wrapped.append(line)
            body += code(x + 22, 296, wrapped)
        body += label(48, 631, "$ " + command, 18, "#80e8c0", "monospace")
        body += label(48, 686, footer, 20)
        body += label(48, 749, "Rendered from verified CLI output · edited playback · not a timing benchmark", 16, "#8ba3b7")
        body += label(1110, 749, f"{i + 1} / 4", 18, "#80e8c0")
        path = args.work / f"scene-{i}.svg"
        path.write_text(svg(body))
        subprocess.run(["rsvg-convert", str(path), "-o", str(path.with_suffix(".png"))], check=True)
    playlist = args.work / "frames.txt"
    playlist.write_text("".join(f"file '{(args.work / f'scene-{i}.png').resolve()}'\nduration 8\n" for i in range(4))
                        + f"file '{(args.work / 'scene-3.png').resolve()}'\n")
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0",
                    "-i", str(playlist), "-t", "32", "-vf", "fps=24,format=yuv420p", "-c:v", "libx264", "-crf", "22",
                    "-movflags", "+faststart", str(args.assets / "demo.mp4")], check=True)
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(args.assets / "demo.mp4"),
                    "-filter_complex", "fps=2,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse",
                    "-loop", "0", str(args.assets / "demo.gif")], check=True)
    manifest = {"schema_version": 1, "evidence_sha256": hashlib.sha256(args.report.read_bytes()).hexdigest(),
                "compiler": report["compiler"], "simulation_executed": False,
                "playback": "four eight-second scenes; no runtime timing claims",
                "files": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in sorted(args.assets.iterdir()) if p.is_file() and p.suffix in (".svg", ".gif", ".mp4")}}
    (args.assets / "demo.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, default=ROOT / "target/demo/report.json")
    parser.add_argument("--work", type=Path, default=ROOT / "target/demo-media")
    parser.add_argument("--assets", type=Path, default=ROOT / "docs/assets")
    render(parser.parse_args())
