#!/usr/bin/env python3
"""Render a walkthrough from demo/run.py evidence with rsvg-convert and FFmpeg."""

import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import subprocess
import textwrap
import time


ROOT = Path(__file__).resolve().parents[1]
FPS = 12
DURATION = 32
INK = "#e3edf5"
MUTED = "#a8bbcc"
ACCENT = "#d1ed6b"


def label(x, y, text, size=22, color=INK, family="sans-serif", weight="400"):
    return (f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" '
            f'font-family="{family}" font-weight="{weight}">{html.escape(text)}</text>')


def code(x, y, lines, active=None):
    body = ""
    for i, line in enumerate(lines):
        if i == active:
            body += f'<rect x="{x - 10}" y="{y + i * 30 - 23}" width="540" height="30" fill="#34451b"/>'
        body += label(x, y + i * 30, line, 18, ACCENT if i == active else INK, "monospace")
    return body


def svg(body, width=1280, height=800):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}"><rect width="100%" height="100%" '
            f'fill="#0d1924"/>{body}</svg>')


def overview():
    body = label(48, 64, "LAMLET", 18, ACCENT, weight="700")
    body += label(48, 110, "Check the input. Emit standard LAMMPS.", 32, "#ffffff", weight="700")
    boxes = [(48, "YOUR INPUT", ["Existing LAMMPS scripts", "Typed quantities and paths", "Scopes and stages"]),
             (443, "LAMLET · RUST", ["Parse and preserve", "Check types and state", "Compile explicit extensions"]),
             (838, "YOUR OUTPUT", ["Standard LAMMPS scripts", "JSON diagnostics", "Dependency manifests"])]
    for x, title, lines in boxes:
        body += f'<rect x="{x}" y="155" width="350" height="190" rx="6" fill="#162b3d" stroke="#314a60"/>'
        body += label(x + 24, 192, title, 16, ACCENT, weight="700")
        for i, line in enumerate(lines):
            body += label(x + 24, 235 + 36 * i, line, 19)
    for x in (405, 800):
        body += f'<path d="M{x} 250h28m-8-7 8 7-8 7" fill="none" stroke="{ACCENT}" stroke-width="3"/>'
    body += label(48, 399, "Keep legacy bytes. Add checks incrementally. Launch LAMMPS explicitly.", 21)
    return svg(body, 1236, 440)


def command(scene, verb):
    return next(c for c in scene["commands"] if c["command"].startswith("lamlet " + verb + " "))


def typed(text, seconds, start, duration):
    return text[:int(len(text) * max(0, min(1, (seconds - start) / duration)))]


def revealed(lines, seconds, start=2.2, interval=0.65):
    return [typed(line, seconds, start + i * interval, interval * 0.7)
            for i, line in enumerate(lines) if seconds >= start + i * interval]


def panel(x, heading, lines, active=None):
    wrapped = []
    for line in lines:
        wrapped.extend(textwrap.wrap(line, 49, subsequent_indent="  ",
                                     replace_whitespace=False, drop_whitespace=False) or [""])
    if len(wrapped) > 8:
        raise ValueError("Code exceeds the video panel")
    return (f'<rect x="{x}" y="210" width="574" height="325" rx="6" fill="#162b3d" stroke="#314a60"/>'
            + label(x + 20, 244, heading, 17, MUTED)
            + code(x + 20, 285, wrapped, active))


def frame(scenes, index, seconds, logo):
    s = scenes[("units", "diagnostic", "stage", "legacy")[index]]
    titles = ["Attach units. Emit the right numbers.", "Read the error. Repair the input.",
              "Create resources. Generate their cleanup.", "Keep the script you already have."]
    left, right = s["source"].splitlines(), []
    left_title, right_title = "Source", "Generated LAMMPS"
    cmd = command(s, "build" if index != 1 else "check")["command"]
    active = None
    status, note = "", ""
    if index == 0:
        right = revealed(s["generated"].splitlines(), seconds)
        active = min(3, max(0, int((seconds - 2.2) / 0.65))) if seconds > 2.2 else None
        note = "2 fs becomes 0.002 ps; the finite loop expands to two runs."
        status = "Exit 0 · no diagnostics" if seconds > 5.4 else ""
    elif index == 1:
        error = s["evidence"]
        right_title = "JSON diagnostic · selected fields"
        fields = ["{", f'  "code": "{error["code"]}",',
                  f'  "expected": "{error["expected"]}",',
                  f'  "actual": "{error["actual"]}",',
                  f'  "severity": "{error["severity"]}"', "}"]
        right = revealed(fields, seconds, 1.25, 0.3)
        note = "The diagnostic identifies the dimension and source location."
        status = "Exit 1 · expected Time; found Temperature" if seconds > 3 else ""
        if seconds >= 4:
            repair = s["repair"]
            left_title = "Source · correcting the reported expression"
            # Replay deletion and insertion without changing any other source bytes.
            value = "300 K"[:max(0, 5 - int((seconds - 4) * 10))] if seconds < 4.5 else typed("2 fs", seconds, 4.5, 0.5)
            left = s["source"].replace("300 K", value).splitlines()
            right, status = [], ""
            right_title = "Repaired build · generated LAMMPS"
            cmd = command(repair, "build")["command"]
            right = revealed(repair["generated"].splitlines(), seconds, 6.2, 0.5)
            note = "Replace 300 K with 2 fs, then build again."
            status = "Exit 0 · repaired input compiles" if seconds > 7.1 else ""
    elif index == 2:
        left_title = "Stage source · exact excerpt"
        left = [line.strip() for line in left if line.strip().startswith(("@fix ", "@dump ", "run "))]
        left.append("}")
        selected = [line.strip() for line in s["generated"].splitlines()
                    if line.strip().startswith(("fix ", "dump ", "run ", "undump ", "unfix "))]
        right = revealed(selected, seconds, 2, 1)
        active = len(right) - 1 if right else None
        note = "Owned resources are released in reverse order: undump, then unfix."
        status = "Exit 0 · stage built; Unknown findings retained" if seconds > 6.9 else ""
    else:
        left_title, right_title = "Legacy input · exact excerpt", "Generated output · same excerpt"
        left = left[1:8]
        right = revealed(s["generated"].splitlines()[1:8], seconds, 1.8, 0.55)
        note = "Unsupported commands stay unverified. Existing bytes are preserved."
        status = f'Exit 0 · full {s["evidence"]["input_bytes"]}-byte input/output comparison: identical' if seconds > 6 else ""
    body = '<rect x="48" y="24" width="170" height="53" rx="3" fill="white"/>' + logo
    body += label(250, 56, "COMPILER WALKTHROUGH", 16, MUTED)
    body += label(1130, 56, f"{index + 1} / 4", 18, ACCENT)
    body += label(48, 125, titles[index], 33, INK, weight="700")
    body += label(48, 170, note, 20, MUTED)
    body += panel(48, left_title, left)
    body += panel(658, right_title, right, active)
    body += '<rect x="48" y="562" width="1184" height="162" rx="6" fill="#122333" stroke="#314a60"/>'
    start = 5 if index == 1 and seconds >= 4 else 0.2
    duration = 1.1 if index == 1 else 1.6
    display = "$ " + typed(cmd, seconds, start, duration)
    if seconds < start + duration and int(seconds * 4) % 2 == 0:
        display += "▌"
    for i, line in enumerate(textwrap.wrap(display, 109, subsequent_indent="  ", drop_whitespace=False)):
        body += label(68, 596 + i * 25, line, 17, ACCENT, "monospace")
    body += label(68, 700, status, 18, INK)
    body += label(48, 764, "Verified CLI evidence · edited playback · no molecular dynamics", 16, MUTED)
    body += f'<rect x="48" y="782" width="1184" height="3" fill="#314a60"/><rect x="48" y="782" width="{1184 * (index * 8 + seconds) / DURATION:.2f}" height="3" fill="{ACCENT}"/>'
    return svg(body)


def render(args):
    report = json.loads(args.report.read_text())
    scenes = {s["id"]: s for s in report["steps"]}
    if report["simulation_executed"] or not {"units", "diagnostic", "stage", "legacy"} <= scenes.keys():
        raise ValueError("Expected verified compiler-only demo evidence")
    error = scenes["diagnostic"]
    cleanup = [line for line in scenes["stage"]["generated"].splitlines()
               if line.startswith(("fix ", "dump ", "undump ", "unfix "))]
    if (len(cleanup) != 4 or error["evidence"]["code"] != "E_DIMENSION_MISMATCH"
            or command(error["repair"], "build")["exit_code"] != 0
            or error["repair"]["generated"] != "units metal\ntimestep 0.002\n"
            or not scenes["legacy"]["evidence"]["byte_identical"]):
        raise ValueError("Demo evidence changed; review the animated sequence")
    args.work.mkdir(parents=True, exist_ok=True)
    args.assets.mkdir(parents=True, exist_ok=True)
    path = args.work / "frame.svg"
    logo = (args.assets / "logo.svg").read_text()
    logo = re.sub(r'width="1040" height="320"', 'x="51" y="26" width="164" height="50"', logo, count=1)
    started = time.perf_counter()
    for number in range(FPS * DURATION):
        index, local = divmod(number / FPS, 8)
        path.write_text(frame(scenes, int(index), local, logo))
        subprocess.run(["rsvg-convert", str(path), "-o", str(args.work / f"frame-{number:04}.png")], check=True)
        if number == 0:
            elapsed = time.perf_counter() - started
            print(f"One frame: {elapsed:.3f}s; {FPS * DURATION} frames estimated at {elapsed * FPS * DURATION:.1f}s", flush=True)
        if number % (FPS * 8) == FPS * 8 - 1:
            print(f"Rendered chapter {int(index) + 1}/4", flush=True)
    (args.assets / "overview.svg").write_text(overview())
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-framerate", str(FPS),
                    "-i", str(args.work / "frame-%04d.png"), "-t", str(DURATION),
                    "-vf", "fps=24,format=yuv420p", "-c:v", "libx264", "-threads", "2", "-crf", "22",
                    "-movflags", "+faststart", str(args.assets / "demo.mp4")], check=True)
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(args.assets / "demo.mp4"),
                    "-filter_complex_threads", "1", "-filter_complex",
                    "fps=8,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse",
                    "-loop", "0", str(args.assets / "demo.gif")], check=True)
    manifest = {"schema_version": 1, "evidence_sha256": hashlib.sha256(args.report.read_bytes()).hexdigest(),
                "compiler": report["compiler"], "simulation_executed": False,
                "duration_seconds": DURATION, "animation_fps": FPS, "rendered_frames": FPS * DURATION,
                "playback": "command typing, output reveal, source repair, and ordered cleanup; edited timing",
                "files": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in sorted(args.assets.iterdir()) if p.is_file() and p.suffix in (".svg", ".gif", ".mp4")}}
    (args.assets / "demo.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, default=ROOT / "target/demo/report.json")
    parser.add_argument("--work", type=Path, default=ROOT / "target/demo-media")
    parser.add_argument("--assets", type=Path, default=ROOT / "docs/assets")
    render(parser.parse_args())
