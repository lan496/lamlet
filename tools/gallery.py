#!/usr/bin/env python3
"""Compile the gallery and keep its displayed code and counts reproducible."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
CASES = [
    (
        "temperature-sweep",
        "Five temperatures, one recipe",
        "A finite loop calls the same typed operation at five temperatures. "
        "The loop clears LAMMPS state before each call. The function reloads "
        "the structure and releases its thermostat. These are separate simulations in one "
        "LAMMPS process; stages provide separate-process execution.",
    ),
    (
        "reusable-phase",
        "One operation, four typed calls",
        "The function owns its compute and fix. Each call checks the temperature "
        "type and the declared state contracts, then releases both resources. "
        "The full definition, contracts, setup, and calls count toward the total.",
    ),
    (
        "resource-lifetimes",
        "Three resources, one scope",
        "Acquisition order determines cleanup: undump, unfix, then uncompute. "
        "Adding another managed declaration also adds its cleanup. Plain LAMMPS "
        "fix, compute, and dump commands keep their manual lifetimes.",
    ),
    (
        "explicit-units",
        "Units make intent explicit",
        "In metal units, 2 fs becomes 0.002 ps. "
        "This small example needs more Lamlet lines and characters. Its benefit "
        "is dimensional checking and explicit units.",
    ),
]


def measure(source: str) -> dict:
    return {
        "code_lines": sum(
            bool(line.strip()) and not line.lstrip().startswith("#")
            for line in source.splitlines()
        ),
        "characters": len(source),
        "sha256": hashlib.sha256(source.encode()).hexdigest(),
    }


def reduction(source: int, generated: int) -> str:
    percent = (generated - source) / generated * 100
    return f"{abs(percent):.0f}% {'fewer' if percent >= 0 else 'more'}"


def render(cases: list[dict]) -> str:
    lines = [
        "---", "hide:", "  - toc", "---", "", "# Gallery", "",
        "These examples compare Lamlet inputs with their actual compiler output. "
        "Repetition provides the "
        "largest reduction. Types and contracts sometimes add code.", "",
        '<div class="lamlet-gallery-summary" markdown>', "",
        "| Example | Lamlet lines | LAMMPS lines | Line reduction | Characters, Lamlet / LAMMPS |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for case in cases:
        before, after = case["lamlet"], case["lammps"]
        lines.append(
            f'| [{case["title"]}](#{case["id"]}) | {before["code_lines"]} '
            f'| {after["code_lines"]} | {reduction(before["code_lines"], after["code_lines"])} '
            f'| {before["characters"]} / {after["characters"]} |'
        )
    lines.extend([
        "", "</div>", "",
        "All setup, declarations, and braces are counted. "
        "Each example uses the included demonstration structure; generated "
        "paths assume execution from the repository root. "
        "[Comparison method and limits](#comparison-method).",
    ])
    for case in cases:
        before, after = case["lamlet"], case["lammps"]
        lines.extend([
            "", f'## {case["title"]} {{ #{case["id"]} }}', "",
            case["description"], "",
            '<div class="lamlet-code-pair" markdown>', "", '<div markdown>', "",
            f'<p class="lamlet-caption">Lamlet · {before["code_lines"]} lines · {before["characters"]} characters</p>',
            "", "```lamlet", case["source"].rstrip("\n"), "```", "", "</div>", "",
            '<div markdown>', "",
            f'<p class="lamlet-caption">Generated LAMMPS · {after["code_lines"]} lines · {after["characters"]} characters</p>',
            "", "```lammps", case["generated"].rstrip("\n"), "```", "", "</div>", "", "</div>", "",
            f'[Source](https://github.com/lan496/lamlet/blob/main/examples/gallery/{case["id"]}.lamlet) '
            f'· [Generated input](assets/gallery/{case["id"]}.in)',
        ])
    lines.extend([
        "", "## Comparison method { #comparison-method }", "",
        "Code lines exclude blank lines and full-line comments. Characters "
        "include all whitespace and final newlines. No boilerplate is removed "
        "from either side. These are source-size comparisons, not runtime or "
        "scientific-accuracy measurements.", "",
        "The baseline is Lamlet's expanded output. Handwritten LAMMPS can also "
        "reduce repetition with variables, `include`, and `jump`; this gallery "
        "does not compare against the shortest possible LAMMPS program.", "",
        "Each input includes setup and reads the same eight-atom "
        "[demonstration structure](https://github.com/lan496/lamlet/blob/main/examples/gallery/structure.data) "
        "with masses and initial velocities. "
        "The model illustrates compiler behavior; no MD is run to produce this "
        "gallery, and the examples are not validated materials calculations. "
        "Unsupported operand schemas can still produce Unknown diagnostics.",
        "", "## Reproduce the comparison", "",
        "The gallery is generated by the release CLI. CI recompiles all four "
        "examples and checks the displayed code, counts, and downloadable files.", "",
        "```sh", "cargo build --release --locked", "python3 tools/gallery.py --check", "```", "",
        "After an intentional example or compiler change, run "
        "`python3 tools/gallery.py` to refresh the page and "
        "[machine-readable measurements](assets/gallery/report.json). "
        "The generator never launches LAMMPS.", "",
        "For the syntax and its guarantees, see [scoped resources](scopes.md), "
        "[modules and stages](modules-and-stages.md), and the "
        "[language reference](reference.md).", "",
    ])
    return "\n".join(lines)


def generate(binary: Path) -> dict[Path, str]:
    cases, files = [], {}
    with tempfile.TemporaryDirectory(prefix="lamlet-gallery-") as directory:
        for name, title, description in CASES:
            relative = Path("examples/gallery") / f"{name}.lamlet"
            source = (ROOT / relative).read_text()
            output = Path(directory) / f"{name}.in"
            result = subprocess.run(
                [str(binary), "build", str(relative), "-o", str(output), "--format", "json"],
                cwd=ROOT, capture_output=True, text=True, check=True,
            )
            diagnostics = json.loads(result.stdout)
            if diagnostics["summary"]["errors"]:
                raise ValueError(f"{relative}: compiler reported errors")
            generated = output.read_text()
            files[Path(f"docs/assets/gallery/{name}.in")] = generated
            cases.append({
                "id": name, "title": title, "description": description,
                "source": source, "generated": generated,
                "lamlet": measure(source), "lammps": measure(generated),
                "diagnostics": diagnostics["summary"],
            })
    report = {
        "schema_version": 1,
        "method": {
            "code_lines": "Nonblank lines excluding full-line comments; all declarations and braces included",
            "characters": "Unicode characters including whitespace and final newlines",
            "execution": "Compiler only; LAMMPS not executed",
        },
        "cases": [
            {key: value for key, value in case.items() if key not in {"source", "generated", "description"}}
            for case in cases
        ],
    }
    files[Path("docs/assets/gallery/report.json")] = json.dumps(report, indent=2, sort_keys=True) + "\n"
    files[Path("docs/gallery.md")] = render(cases)
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lamlet")
    parser.add_argument("--check", action="store_true", help="Fail if generated files are stale")
    args = parser.parse_args()
    files = generate(args.binary.resolve())
    stale = []
    for path, content in files.items():
        destination = ROOT / path
        if args.check:
            if not destination.exists() or destination.read_text() != content:
                stale.append(str(path))
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(content)
    if stale:
        print("Stale gallery files: " + ", ".join(stale))
        print("Run python3 tools/gallery.py to regenerate them.")
        return 1
    print(f"Gallery: {len(CASES)} compiled examples; {len(files)} files {'checked' if args.check else 'written'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
