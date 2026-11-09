import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from corpus import write_json
import repair_diagnostics as diagnostics
import repair_tasks


class RepairDiagnosticTests(unittest.TestCase):
    def test_warning_denominators_separate_authored_controls_and_target_defects(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            contracts = repair_tasks.tasks()
            write_json(root / "task-contracts.json", contracts)
            write_json(root / "manifest.json", {"images": {"C": {"tag": "frozen", "id": "sha256:test"}},
                "files": {"task-contracts.json": diagnostics.digest(root / "task-contracts.json")},
                "runtime": {"lamlet_sha256": "checker"}})
            warning_inputs = {contracts["integrator-dev"]["input"]: "LML-PHYS-001",
                              contracts["sampling-dev"]["reference"]: "UNEXPECTED"}

            def invoke(arguments, **kwargs):
                mount = arguments[arguments.index("--mount") + 1]
                source = Path(mount.split("source=", 1)[1].split(",target=", 1)[0])
                code = warning_inputs.get(source.read_text())
                report = {"diagnostics": [{"code": code, "severity": "warning"}] if code else []}
                return subprocess.CompletedProcess(arguments, 0, json.dumps(report), "")

            with patch.object(diagnostics.subprocess, "check_output", return_value="sha256:test\n"), \
                 patch.object(diagnostics.subprocess, "run", side_effect=invoke):
                diagnostics.measure(root, root / "observed.json")
            observed = json.loads((root / "observed.json").read_text())
            self.assertEqual(observed["reference_cases"], 3)
            self.assertEqual(observed["references_with_warnings"], 1)
            self.assertEqual(observed["target_defects_warned"], 1)
            self.assertEqual(observed["defect_cases"], 3)
            self.assertFalse(observed["used_for_oracle_grades"])
            self.assertFalse(any("reserve" in case["id"] for case in observed["cases"]))

    def test_changed_image_fails_before_any_checker_invocation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(root / "task-contracts.json", {})
            write_json(root / "manifest.json", {"files": {},
                "images": {"C": {"tag": "frozen", "id": "sha256:expected"}}})
            with patch.object(diagnostics.subprocess, "check_output", return_value="sha256:changed"), \
                 patch.object(diagnostics.subprocess, "run") as run:
                with self.assertRaisesRegex(ValueError, "Checker image changed"):
                    diagnostics.measure(root, root / "not-written.json")
                run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
